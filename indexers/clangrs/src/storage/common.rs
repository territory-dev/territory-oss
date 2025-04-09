use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::oneshot;


pub type StoreRequest = (PathBuf, Vec<u8>);

pub type Done = oneshot::Receiver<()>;


#[derive(Clone)]
pub struct StorageChannel {
    send: Arc<tokio::sync::mpsc::Sender<StoreRequest>>,
}

impl StorageChannel {
    pub fn submit_blob_blocking(&self, path: PathBuf, data: Vec<u8>) {
        self.send.blocking_send((path, data)).unwrap();
    }

    pub async fn submit_blob(&self, path: PathBuf, data: Vec<u8>) {
        self.send.send((path, data)).await.unwrap();
    }

    pub(super) fn new(s: usize) -> (Self, Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<StoreRequest>>>) {
        let (s, r) = tokio::sync::mpsc::channel(s);
        let ch = Self {
            send: Arc::new(s),
        };
        (ch, Arc::new(tokio::sync::Mutex::new(r)))
    }

    pub(super) fn new_owned(s: usize) -> (Self, tokio::sync::mpsc::Receiver<StoreRequest>) {
        let (s, r) = tokio::sync::mpsc::channel(s);
        let ch = Self {
            send: Arc::new(s),
        };
        (ch, r)
    }
}


