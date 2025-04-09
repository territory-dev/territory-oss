use std::collections::{HashMap, HashSet};

use std::sync::mpsc::SyncSender;

use cscanner::ipc::{DriverSays, LockGrant, Locking};
use territory_core::RelativePath;


#[derive(Debug)]
pub struct FileHold {
    holder: Option<usize>,
    waiting: HashSet<usize>,
}

#[derive(Clone, Debug)]
pub enum ThreadHold {
    Holding(HashSet<RelativePath>),
    Waiting(HashSet<RelativePath>, SyncSender<DriverSays>),
    Idle,
}


#[derive(Debug)]
pub struct LocksAgent {
    processed: HashSet<RelativePath>,

    thread_holds: Vec<ThreadHold>,
    file_holds: HashMap<RelativePath, FileHold>,
}

impl LocksAgent {
    pub fn new(thread_count: usize) -> Self {
        LocksAgent {
            processed: HashSet::new(),
            thread_holds: vec![ThreadHold::Idle; thread_count+1],  // [0] is a dummy
            file_holds: HashMap::new(),
        }
    }

    pub fn release_thread(&mut self, thread: usize) {
        // thread = idle
        //
        // for f: file held by thread
        //      f.holder = None
        //      for t: f.waiting
        //          t can be resumed?
        //              set new holder for awaited files
        //              t = holding
        //              notify t


        let ThreadHold::Holding(held_paths) = std::mem::replace(
            self.thread_holds.get_mut(thread).unwrap(),
            ThreadHold::Idle,
        ) else {
            return;
        };

        self.thread_holds[thread] = ThreadHold::Idle;
        let mut notify_candidates: HashSet<usize> = HashSet::new();
        for path in held_paths {
            let fh = self.file_holds.remove(&path).expect("held path not in file_holds");
            self.processed.insert(path);
            notify_candidates.extend(fh.waiting.into_iter());
        }

        for thread in notify_candidates {
            self.try_acquire_all(thread);
        }
    }

    pub fn handle_message(
        &mut self,
        thread: usize,
        msg: Locking,
        resp_ch: SyncSender<DriverSays>,
    ) {
        match msg {
            Locking::LockAll { paths } => {
                // can all files be acquired?
                //      thread = holding
                //      set files to held
                // else
                //      add to waiting for each file

                self.release_thread(thread);
                for path in &paths {
                    if self.processed.contains(path) { continue; }

                    let file_hold = self.file_holds
                        .entry(path.clone())
                        .or_insert(FileHold { holder: None, waiting: HashSet::new() });
                    file_hold.waiting.insert(thread);

                }
                *self.thread_holds.get_mut(thread).unwrap() = ThreadHold::Waiting(paths, resp_ch);

                self.try_acquire_all(thread);
            },
        }
    }

    pub fn dump_state(&self) {
        println!("thread holds: {:#?}", self.thread_holds);
        println!("file holds: {:#?}", self.file_holds);
    }

    pub fn is_held(&self, thread: usize, path: &RelativePath) -> bool {
        let ThreadHold::Holding(fs) = &self.thread_holds[thread] else {
            return false;
        };

        fs.contains(path)
    }

    fn try_acquire_all(&mut self, thread: usize) -> bool {
        match self.thread_holds.get(thread).unwrap() {
            ThreadHold::Idle => {
                unreachable!("unexpected Idle thread on try_acquire_all");
            },
            ThreadHold::Holding(_) => { return false; },
            ThreadHold::Waiting(paths, _) if !self.can_acquire_all(paths) => { return false; }
            ThreadHold::Waiting(_, _) => {}
        };

        let ThreadHold::Waiting(paths, resp_ch) = std::mem::replace(&mut self.thread_holds[thread], ThreadHold::Idle) else {
            unreachable!("thead must be Waiting");
        };

        let (already_processed, need_processing) = paths
            .into_iter()
            .partition::<HashSet<_>, _>(|relpath| self.processed.contains(&relpath));

        for f in &need_processing {
            let file_hold = self.file_holds
                .entry(f.clone())
                .or_insert(FileHold { holder: None, waiting: HashSet::new() });
            file_hold.waiting.remove(&thread);
            let prev_holder = file_hold.holder.replace(thread);
            assert_eq!(prev_holder, None);
        }
        self.thread_holds[thread] = ThreadHold::Holding(need_processing);

        let lock_grant = LockGrant { already_processed };
        resp_ch.send(DriverSays::LockResponse(lock_grant)).unwrap();

        true
    }

    fn can_acquire_all(&self, paths: &HashSet<RelativePath>) -> bool {
        for path in paths {
            if self.processed.contains(path) { continue; }

            let Some(hold) = self.file_holds.get(path) else { continue; };
            if hold.holder.is_some() { return false; }
        }

        true
    }
}


#[cfg(test)]
mod test {
    use std::{collections::HashSet, path::PathBuf, sync::mpsc::sync_channel};

    use cscanner::ipc::{DriverSays, LockGrant};
    use territory_core::RelativePath;

    use super::LocksAgent;

    fn relpath(p: &str) -> RelativePath {
        RelativePath::from(PathBuf::from(p))
    }

    fn lock_all(paths: &[&str]) -> cscanner::ipc::Locking {
        cscanner::ipc::Locking::LockAll {
            paths: paths.iter().map(|p| relpath(p)).collect()
        }
    }

    #[test]
    fn acquire() {
        let mut la = LocksAgent::new(1);

        let (resp_sender, resp_rcv) = sync_channel(1);
        la.handle_message(1, lock_all(&["p1"]), resp_sender);

        assert_eq!(resp_rcv.try_recv(), Ok(DriverSays::LockResponse(LockGrant {
            already_processed: HashSet::new(),
        })));
    }

    #[test]
    fn wait_chain() {
        let mut la = LocksAgent::new(3);

        let (resp_sender_1, _resp_rcv_1) = sync_channel(1);
        la.handle_message(
            1,
            lock_all(&["p1", "p2"]),
            resp_sender_1);

        let (resp_sender_2, resp_rcv_2) = sync_channel(1);
        la.handle_message(
            2,
            lock_all(&["p2", "p3"]),
            resp_sender_2);
        assert_eq!(resp_rcv_2.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        let (resp_sender_3, resp_rcv_3) = sync_channel(1);
        la.handle_message(
            3,
            lock_all(&["p3"]),
            resp_sender_3);
        assert_eq!(resp_rcv_3.try_recv(), Ok(DriverSays::LockResponse(LockGrant {
            already_processed: HashSet::new(),
        })));

        la.release_thread(1);
        assert_eq!(resp_rcv_2.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        la.release_thread(3);
        assert_eq!(resp_rcv_2.try_recv(), Ok(DriverSays::LockResponse(LockGrant {
            already_processed: HashSet::from([ relpath("p2"), relpath("p3") ]),
        })));
    }

    #[test]
    fn already_processed() {
        let mut la = LocksAgent::new(3);

        let (resp_sender_1, _resp_rcv_1) = sync_channel(1);
        la.handle_message(
            1,
            lock_all(&["p1", "p2"]),
            resp_sender_1);

        let (resp_sender_2, resp_rcv_2) = sync_channel(1);
        la.handle_message(
            2,
            lock_all(&["p1"]),
            resp_sender_2);
        assert_eq!(resp_rcv_2.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        let (resp_sender_3, resp_rcv_3) = sync_channel(1);
        la.handle_message(
            3,
            lock_all(&["p2"]),
            resp_sender_3);
        assert_eq!(resp_rcv_3.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty));

        la.release_thread(1);
        assert_eq!(resp_rcv_2.try_recv(), Ok(DriverSays::LockResponse(LockGrant {
            already_processed: HashSet::from([ relpath("p1") ]),
        })));
        assert_eq!(resp_rcv_3.try_recv(), Ok(DriverSays::LockResponse(LockGrant {
            already_processed: HashSet::from([ relpath("p2") ]),
        })));
    }

    #[test]
    fn is_held() {
        let mut la = LocksAgent::new(3);

        let path = relpath("p1");

        let (resp_sender_1, _resp_rcv_1) = sync_channel(1);
        la.handle_message(
            1,
            lock_all(&["p1"]),
            resp_sender_1);

        let (resp_sender_2, _resp_rcv_2) = sync_channel(1);
        la.handle_message(
            2,
            lock_all(&["p1"]),
            resp_sender_2);

        assert!( la.is_held(1, &path));
        assert!(!la.is_held(2, &path));
        assert!(!la.is_held(3, &path));

        la.release_thread(1);
        assert!(!la.is_held(1, &path));
    }
}
