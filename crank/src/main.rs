// use reqwest::Url;
// use serde_json::Value;
use tokio::{time::Duration, time::sleep};

// use std::time::{SystemTime, UNIX_EPOCH};
pub mod engine;
pub mod execution;
pub mod prices;
pub use engine::trigger_engine;
pub use execution::execution_worker;
pub use prices::price_ingestor;

#[tokio::main]
async fn main() {
    let (price_tx, price_rx) = tokio::sync::mpsc::channel(32);
    let (exec_tx, exec_rx) = tokio::sync::mpsc::channel(8);

    tokio::spawn(price_ingestor(price_tx));
    tokio::spawn(trigger_engine(price_rx, exec_tx));
    tokio::spawn(execution_worker(exec_rx));

    // keep main alive
    loop {
        sleep(Duration::from_secs(60)).await;
    }
}
