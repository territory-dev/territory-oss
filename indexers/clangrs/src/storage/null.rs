use tokio::sync::oneshot;

use super::common::{Done, StorageChannel};


pub async fn start() -> (Done, StorageChannel) {
    let (sender, mut rcv) = StorageChannel::new_owned(2);
    let (done_sender, done_rcv) = oneshot::channel();

    tokio::spawn(async move {
        while let Some(_) = rcv.recv().await { }
        done_sender.send(()).unwrap();
    });

    (done_rcv, sender)
}
