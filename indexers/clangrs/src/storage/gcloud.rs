use std::sync::Arc;
use std::time::{Duration, Instant};

use google_cloud_storage::client::{ClientConfig, Client};
use google_cloud_storage::http::Error as GCSError;
use google_cloud_storage::http::objects::Object;
use google_cloud_storage::http::objects::upload::{UploadObjectRequest, UploadType};
use tokio::select;
use tokio::sync::oneshot;
use tokio::task::{JoinError, JoinSet};

use crate::args::CompressionMode;
use super::common::{StorageChannel, StoreRequest, Done};


const BASE_CHILL_TIME: Duration = Duration::from_secs(1);
const MAX_CHILL_TIME: Duration = Duration::from_secs(64);


#[derive(Default, PartialEq)]
enum Stage {
    #[default]
    Running,
    Terminating,
    Done,
}


#[derive(Default)]
struct State {
    stage: Stage,

    joinset: JoinSet<StoreResult>,
    retries: Vec<StoreRequest>,

    running_count: usize,
    accepted_ctr: usize,
    done_ctr: usize,
    failed_ctr: usize,

    chill_time: Duration,
}


pub async fn start(
    bucket_name: String,
    compression: CompressionMode,
    store_concurrency: usize,
) -> (Done, StorageChannel) {
    let (sender, mut rcv) = StorageChannel::new_owned(store_concurrency * 2);
    let (done_sender, done_rcv) = oneshot::channel();

    tokio::spawn(async move {
        let mut state = State {
            chill_time: BASE_CHILL_TIME,
            ..Default::default()
        };

        let actuator = Arc::new(Actuator::new(bucket_name, compression).await);

        let mut last_print = Instant::now();
        while state.stage != Stage::Done {
            if last_print.elapsed() > Duration::from_secs(10) {
                print_status(&state).await;
                last_print = Instant::now();
            }

            if state.running_count == store_concurrency {
                // busy
                let res = state.joinset.join_next().await;
                on_join(&mut state, res).await;
            } else if let Some(r) = state.retries.pop() {
                // retry
                on_request(&mut state, Arc::clone(&actuator), r).await;
            } else if state.stage == Stage::Terminating {
                // terminating
                let res = state.joinset.join_next().await;
                match res {
                    None      => { state.stage = Stage::Done; },
                    Some(res) => { on_result(&mut state, res.unwrap()).await },
                }
            } else if state.running_count == 0 {
                // idle
                let rcv_result = rcv.recv().await;
                on_receive(&mut state, &actuator, rcv_result).await;
            } else {
                // have capacity
                select! {
                    rcv_result = rcv.recv() => {
                        on_receive(&mut state, &actuator, rcv_result).await;
                    },
                    res = state.joinset.join_next() => {
                        on_join(&mut state, res).await;
                    },
                };
            }
        }

        print_status(&state).await;
        done_sender.send(()).unwrap();
    });

    (done_rcv, sender)
}


async fn on_result(state: &mut State, res: StoreResult) {
    state.running_count -= 1;

    match res {
        StoreResult::Ok => {
            state.done_ctr += 1;
            state.chill_time = BASE_CHILL_TIME;
        },
        StoreResult::Fail => { state.failed_ctr += 1; },
        StoreResult::Retry(request) => {
            println!("[CloudStorage] chill for {:?}", state.chill_time);
            tokio::time::sleep(state.chill_time).await;
            state.chill_time *= 2;
            if state.chill_time > MAX_CHILL_TIME {
                state.chill_time = MAX_CHILL_TIME;
            }
            state.retries.push(request);
        }
    }
}

async fn on_join(state: &mut State, res: Option<Result<StoreResult, JoinError>>) {
    let res = res.expect("empty joinset").unwrap();
    on_result(state, res).await;
}


async fn on_receive(state: &mut State, actuator: &Arc<Actuator>, rcv_result: Option<StoreRequest>) {
    match rcv_result {
        None      => {
            state.stage = Stage::Terminating;
        },
        Some(req) => {
            state.accepted_ctr += 1;
            on_request(state, Arc::clone(&actuator), req).await;
        },
    }
}


async fn on_request(state: &mut State, actuator: Arc<Actuator>, req: StoreRequest) {
    state.running_count += 1;

    let actuator = Arc::clone(&actuator);
    state.joinset.spawn(async move {
        actuator.store(req).await
    });
}

async fn print_status(state: &State) {
    println!(
        "[CloudStorage] accepted: {}  in flight: {}  done: {}  failed: {}",
        state.accepted_ctr,
        state.running_count,
        state.done_ctr,
        state.failed_ctr,
    );
}

enum StoreResult {
    Ok,
    Fail,
    Retry(StoreRequest),
}


struct Actuator {
    bucket_name: String,
    compression: CompressionMode,
    client: Client,
}

impl Actuator {
    async fn new(bucket_name: String, compression: CompressionMode) -> Self {
        let config = ClientConfig { ..ClientConfig::default() }
            .with_auth()
            .await.unwrap();
        let client = Client::new(config);
        Actuator { bucket_name, compression, client }
    }

    async fn store(&self, (path, data): StoreRequest) -> StoreResult {
        let path_str = path.to_str().unwrap().to_owned();
        let content_encoding = match self.compression {
            CompressionMode::None => None,
            CompressionMode::Gzip => Some("gzip".to_string()),
        };

        let upload_type = UploadType::Multipart(Box::new(Object {
            name: path_str,
            bucket: self.bucket_name.clone(),
            content_encoding,
            ..Default::default()
        }));
        let result = self.client
            .upload_object(
                &UploadObjectRequest {
                    bucket: self.bucket_name.to_owned(),
                    ..Default::default()
                },
                data.clone(),
                &upload_type)
            .await;

        match result {
            Ok(_) => {
                return StoreResult::Ok;
            }
            Err(e) => {
                match e {
                    GCSError::Response(resp) if resp.is_retriable() => {
                        if resp.code != 429 {
                            println!("[CloudStorage] upload error (retrying): {:?}", resp);
                        }
                        return StoreResult::Retry((path, data));
                    }
                    _ => {
                        println!("[CloudStorage] upload failed: {:?}", e);
                        return StoreResult::Fail;
                    }
                }
            }
        };
    }
}


#[cfg(test)]
#[cfg(feature="live_tests")]
mod test {
    use std::path::PathBuf;
    use google_cloud_storage::http::objects::delete::DeleteObjectRequest;
    use rand::{thread_rng, Rng};
    use rand::distributions::Alphanumeric;

    use google_cloud_storage::client::{ClientConfig, Client};
    use google_cloud_storage::http::objects::get::GetObjectRequest;
    use google_cloud_storage::http::objects::download::Range;

    use crate::args::CompressionMode;


    #[test]
    fn store_on_cloud_storage() {
        let rand_string: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();


        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            let (done, channel) = super::start(
                "territory-index-scrap".to_string(),
                CompressionMode::None,
                1).await;

            let data: Vec<u8> = vec![1, 2, 3, 4, 5];
            channel.submit_blob(PathBuf::from("test").join(&rand_string), data).await;

            drop(channel);
            done.await.unwrap();


            println!("checking result");
            let config = ClientConfig::default().with_auth().await.unwrap();
            let client = Client::new(config);

            let data = client.download_object(&GetObjectRequest {
                bucket: "territory-index-scrap".to_string(),
                object: format!("test/{}", rand_string),
                ..Default::default()
            }, &Range::default()).await.unwrap();

            let _ = client.delete_object(&DeleteObjectRequest {
                bucket: "territory-index-scrap".to_string(),
                object: format!("test/{}", rand_string),
                ..Default::default()
            }).await;

            data
        });

        assert_eq!(result, vec![1, 2, 3, 4, 5]);

    }
}

