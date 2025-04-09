use std::future::Future;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use log::info;


#[derive(Debug)]
pub struct Timers {
    counts: HashMap<String, Duration>,
}

impl Timers {
    pub fn new() -> Self {
        Self { counts: HashMap::new() }
    }

    pub fn timed<T>(&mut self, label: &str, f: impl FnOnce() -> T) -> T{
        let start = Instant::now();
        let res = f();
        let elapsed = start.elapsed();

        let counter = self.counts.entry(label.to_string()).or_default();
        *counter += elapsed;

        res
    }

    pub async fn async_timed<F, T>(&mut self, label: &str, f: F) -> T
        where F : Future<Output = T>
    {
        let start = Instant::now();
        let res = f.await;
        let elapsed = start.elapsed();

        let counter = self.counts.entry(label.to_string()).or_default();
        *counter += elapsed;

        res
    }

    pub fn dump(&self) {
        info!("{:#?}", self);

        info!("Total: {:?}", self.counts.values().sum::<Duration>());
    }
}
