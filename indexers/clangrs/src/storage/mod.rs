mod common;
#[cfg(feature = "gcloud")]
pub mod gcloud;
pub mod file;
pub mod mem;
pub mod null;

use crate::args::{Args, StorageMode};

pub use common::{StoreRequest, StorageChannel, Done};
pub use mem::MemStorage;


#[cfg(feature = "gcloud")]
pub async fn init_cloud_storage(args: &Args) -> (Done, StorageChannel) {
    gcloud::start(args.bucket.clone(), args.compression, args.store_concurrency).await
}


#[cfg(not(feature = "gcloud"))]
pub async fn init_cloud_storage(_args: &Args) -> (Done, StorageChannel) {
    unimplemented!("cloud storage feature not enabled");
}


pub async fn start_from_args(args: &Args) -> (Done, StorageChannel) {
    match args.storage_mode {
        StorageMode::None  => null::start().await,
        StorageMode::File  => file::start(args.clone()).await,
        StorageMode::Cloud => init_cloud_storage(args).await,
    }
}
