use std::collections::HashSet;
use std::path::PathBuf;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::io::Write;
use std::env::var;

use serde::{Serialize, Deserialize};
use serde_json::StreamDeserializer;
use serde_json::de::IoRead;

use territory_core::RelativePath;

use crate::ast::{Block, ClangCommand};


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockGrant {
    pub already_processed: HashSet<RelativePath>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Control {
    Next,
    TUDone { source_set: HashSet<RelativePath> },
    GotBlock { block: Block },
    GotCommands { commands: Vec<ClangCommand> },
    Reschedule { command: ClangCommand },
    Log { content: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Locking {
    LockAll { paths: HashSet<RelativePath> },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ScannerSays {
    Control(Control),
    Locking(Locking),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanCommandsArgs {
    pub single_file: Vec<PathBuf>,
    pub remove_path_prefix: Option<String>,
    pub clang_extra_args: Option<Vec<String>>,
}


#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanOpts {
    pub total_count: usize,
    pub index_system: bool,
    pub max_block_len: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DriverSays {
    ScanCommands(ScanCommandsArgs),
    ClangCommand { command: ClangCommand, opts: ScanOpts },
    LockResponse(LockGrant),
    Finish,
    Again,
    Continue,
    BlockReceived,
}

impl DriverSays {
    pub fn is_continue(&self) -> bool {
        if let Self::Continue = self {
            true
        } else {
            false
        }
    }

    pub fn is_block_received(&self) -> bool {
        if let Self::BlockReceived = self {
            true
        } else {
            false
        }
    }
}


pub struct DriverConn<'a, R, W: std::io::Write> {
    read: StreamDeserializer<'a, R, DriverSays>,
    write: std::io::BufWriter<W>,
}

pub type USDriverConn<'a> = DriverConn<'a, IoRead<&'a UnixStream>, &'a UnixStream>;

impl<'a> USDriverConn<'a> {
    pub fn new(con: &'a UnixStream) -> Self {
        let read = serde_json::de::Deserializer::from_reader(
            con
        ).into_iter();
        let write = std::io::BufWriter::new(con);
        DriverConn { read, write }
    }


    pub fn send(&mut self, r: ScannerSays) -> Result<(), serde_json::Error> {
        if var("DEBUG_IPC") == Ok("yes".to_string()) {
            println!("send: {:?}", r);
        }
        serde_json::to_writer(&mut self.write, &r)?;
        self.write.flush().unwrap();
        Ok(())
    }

    pub fn receive(&mut self) -> Result<DriverSays, serde_json::Error> {
        let driver_says = self.read.next().expect("driver lost");

        if var("DEBUG_IPC") == Ok("yes".to_string()) {
            println!("received: {:?}", driver_says);
        }

        driver_says
    }
}

pub type ConnHandle<'a> = Arc<Mutex<USDriverConn<'a>>>;

pub fn log<'a>(c: &ConnHandle<'a>, t: &str) {
    c.lock().unwrap().send(ScannerSays::Control(Control::Log { content: t.to_owned() })).unwrap();
}
