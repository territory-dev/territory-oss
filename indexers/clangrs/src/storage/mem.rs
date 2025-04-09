use std::collections::HashMap;
use std::path::PathBuf;
use std::thread;

use super::common::StorageChannel;

pub struct MemStorage {
    join: thread::JoinHandle<HashMap<PathBuf, Vec<u8>>>,
}

impl MemStorage {
    pub fn start() -> (Self, StorageChannel) {
        let (sender, rcv) = StorageChannel::new(2);
        let jh = thread::spawn(move || {
            let mut mem = HashMap::new();
            loop {
                let rcv_res = { let mut rcv_hold = rcv.blocking_lock(); rcv_hold.blocking_recv() };
                match rcv_res {
                    Some((k, v)) => {
                        mem.insert(k, v);
                    }
                    None => { break; },
                }
            }
            mem
        });
        (Self {join: jh}, sender)
    }

    pub fn get_mem(self) -> HashMap<PathBuf, Vec<u8>> {
        self.join.join().unwrap()
    }
}

