use std::fs::{File, create_dir_all};
use std::io::Write;
use std::thread;

use tokio::sync::oneshot;

use crate::args::Args;

pub use super::common::{StoreRequest, StorageChannel, Done};

pub async fn start(args: Args) -> (Done, StorageChannel) {
    let (sender, receiver) = StorageChannel::new(args.store_concurrency * 2);

    let (done_sender, done_rcv) = oneshot::channel();

    let join_handles: Vec<thread::JoinHandle<()>> = (0..args.store_concurrency).map(|_thread_num| {
        let t_receiver = receiver.clone();
        let t_args = args.clone();
        thread::spawn(move || {
            loop {
                let rcv_res = { let mut rcv_hold = t_receiver.blocking_lock(); rcv_hold.blocking_recv() };
                match rcv_res {
                    Some(req) => do_work(&t_args, req),
                    None => { break; },
                }
            }
        })
    }).collect();

    thread::spawn(move || {
        for jh in join_handles {
            jh.join().unwrap();
        }
        done_sender.send(()).expect("\"storage done\" receiver closed");
    });

    (done_rcv, sender)
}

fn do_work(args: &Args, (path, data): StoreRequest) {
    let abs_path = args.outdir.join(path);
    create_dir_all(abs_path.parent().unwrap_or(&args.outdir)).unwrap();

    let mut f = File::create(&abs_path).expect(&format!("creating file: {}", abs_path.to_string_lossy()));
    f.write_all(&data).expect(&format!("writing to file: {}", abs_path.to_string_lossy()));
}
