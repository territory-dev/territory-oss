use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Write};
use std::os::unix::net::UnixStream;
use std::{
    os::unix::net::UnixListener,
    error::Error,
    thread
};
use std::sync::mpsc::{sync_channel, RecvTimeoutError, SyncSender};

use serde_json::to_writer;
use log::{info, warn};

use crate::locks_agent::LocksAgent;
use crate::args::{Args, get_scanner_ipc_timeout};

use territory_core::RelativePath;
use cscanner::ast::{ClangCommand, Block};
use cscanner::ipc::{Control, DriverSays, ScanCommandsArgs, ScanOpts, ScannerSays};


enum ScanCommandsState {
    NotStarted,
    Scanning(Vec<SyncSender<DriverSays>>),
    GotCommands(Vec<ClangCommand>, usize),
}

pub fn driver_loop<State>(
    args: &Args,
    mut state: State,
    mut log_file: Option<&mut File>,
    mut on_block: impl FnMut(&mut State, Block, usize) -> Result<(), Box<dyn Error>>,
    mut on_tu_done: impl FnMut(&mut State, usize, &HashSet<RelativePath>) -> Result<(), Box<dyn Error>>,
) {
    let sock = UnixListener::bind(&args.scanner_socket_path).expect("failed to open socket file");
    sock.set_nonblocking(true).unwrap();

    let (send, recv) = sync_channel(args.par);

    let mut la = LocksAgent::new(args.par);

    info!("waiting for scanners");
    let started_waiting = std::time::Instant::now();
    let mut threads = Vec::new();
    for i in 1..=args.par {
        let con = 'accept: loop {
            match sock.accept() {
                Ok((con, _addr)) => {
                    info!("scanner {} connected", i);
                    break 'accept con;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    if started_waiting.elapsed() > std::time::Duration::from_secs(args.scanner_socket_timeout as u64) {
                        panic!("timed out waiting for scanner");
                    }
                    thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => {
                    panic!("Error accepting connection: {}", e);
                }
            }
        };
        let mut args = args.clone();
        args.slice = i;
        let send = send.clone();
        let t = thread::spawn(move || handle_connection(&args, con, send, i));
        threads.push(t);
    }
    drop(send);

    let mut commands = ScanCommandsState::NotStarted;
    let mut pending_commands = 0;
    loop {
        let (thread, msg, responder) = match recv.recv_timeout(get_scanner_ipc_timeout()) {
            Ok(rcvd) => rcvd,
            Err(RecvTimeoutError::Timeout) => {
                panic!("scanner ipc TIMEOUT");
            },
            Err(RecvTimeoutError::Disconnected) => {
                break;
            }
        };

        match msg {
            ScannerSays::Locking(lock_msg) => {
                la.handle_message(thread, lock_msg, responder);
            }
            ScannerSays::Control(Control::Next) => {
                match commands {
                    ScanCommandsState::GotCommands(ref mut commands, commands_count) => {
                        let Some(cmd) = commands.pop() else {
                            responder.send(DriverSays::Finish).unwrap();
                            continue;
                        };
                        pending_commands = commands.len();
                        responder.send(DriverSays::ClangCommand {
                            command: cmd,
                            opts: ScanOpts {
                                total_count: commands_count,
                                index_system: args.index_system,
                                max_block_len: args.max_node_len,
                            },
                        }).unwrap();
                    },
                    ScanCommandsState::NotStarted => {
                        let scan_commands_args = ScanCommandsArgs {
                            clang_extra_args: args.clang_extra_args.clone(),
                            single_file: args.single_file.clone(),
                            remove_path_prefix: args.remove_path_prefix.clone(),
                        };
                        responder.send(DriverSays::ScanCommands(scan_commands_args)).unwrap();
                        commands = ScanCommandsState::Scanning(Vec::new());
                    },
                    ScanCommandsState::Scanning(ref mut waitlist) => {
                        waitlist.push(responder);
                    }
                }
            }
            ScannerSays::Control(Control::Reschedule { command }) => {
                let ScanCommandsState::GotCommands(commands, _count) = &mut commands else {
                    panic!("got Reschedule before GotCommands");
                };
                commands.insert(0, command);
            }
            ScannerSays::Control(Control::GotBlock { block }) => {
                if !la.is_held(thread, &block.context.relative_path) {
                    panic!(
                        "trying to store blocks from path {} that the thread {} does not hold a lock on",
                        block.context.relative_path, thread);
                }

                on_block(&mut state, block, thread).unwrap();
                responder.send(DriverSays::BlockReceived).unwrap();
            }
            ScannerSays::Control(Control::TUDone { source_set }) => {
                on_tu_done(&mut state, thread, &source_set).unwrap();
                la.release_thread(thread);
                responder.send(DriverSays::Continue).unwrap();
            }
            ScannerSays::Control(Control::GotCommands { commands: mut c }) => {
                let ScanCommandsState::Scanning(waitlist) = commands else {
                    panic!("GotCommands but ScanCommandsState is not Scanning");
                };
                c.reverse();
                let commands_count = c.len();
                pending_commands = commands_count;
                for sender in waitlist {
                    sender.send(DriverSays::Again).unwrap();
                }
                commands = ScanCommandsState::GotCommands(c, commands_count);
            }
            ScannerSays::Control(Control::Log { content }) => {
                if let Some(log_file) = &mut log_file {
                    let write_res = log_file.write_fmt(format_args!("<{}> {}\n", thread, content));
                    if let Err(e) = write_res {
                        warn!("failed to write log: {:?}", e);
                    }
                }
                println!("<{}> {}", thread, content);
            }
        }
    }

    la.dump_state();

    for t in threads {
        t.join().unwrap();
    }
    let _ = std::fs::remove_file(&args.scanner_socket_path);

    if pending_commands > 0 {
        panic!("scanners terminated with {} pending commands", pending_commands);
    }
    info!("scanner done");
}


fn handle_connection(
    _args: &Args,
    con: UnixStream,
    send: SyncSender<(usize, ScannerSays, SyncSender<DriverSays>)>,
    thread_num: usize,
) {
    con.set_nonblocking(false).unwrap();
    let reader = std::io::BufReader::new(con.try_clone().unwrap());
    let mut de = serde_json::de::Deserializer::from_reader(reader).into_iter();

    let scanner_ipc_timeout = get_scanner_ipc_timeout();

    while let Some(req) = de.next() {
        let req = req.unwrap();

        let (responder, response_recv) = sync_channel(10);
        send.send((thread_num, req, responder)).unwrap();
        match response_recv.recv_timeout(scanner_ipc_timeout) {
            Ok(response) =>  {
                to_writer(&con, &response).unwrap();
            }
            Err(RecvTimeoutError::Timeout) => {
                panic!("failed to receive driver response: TIMEOUT (waiting thread {})", thread_num);
            }
            Err(RecvTimeoutError::Disconnected) => {
                continue;
            }
        };
    }
    drop(send);

    // if !locks.is_empty() {
    //     warn!("got {} leftover locks", locks.len());
    // }
}
