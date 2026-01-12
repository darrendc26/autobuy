use axum::{Router, routing::get};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::{time::Duration, time::sleep};

pub mod api;
pub mod engine;
pub mod execution;
pub mod prices;

pub use api::api_server;
pub use engine::trigger_engine;
pub use execution::execution_worker;
pub use prices::price_ingestor;

#[tokio::main]
async fn main() {
    // channels
    let (price_tx, price_rx) = tokio::sync::mpsc::channel(32);
    let (exec_tx, exec_rx) = tokio::sync::mpsc::channel(8);

    // spawn workers
    tokio::spawn(price_ingestor(price_tx));
    tokio::spawn(trigger_engine(price_rx, exec_tx));
    tokio::spawn(execution_worker(exec_rx));

    // spawn API server
    tokio::spawn(api_server());

    // keep main alive (supervisor role)
    loop {
        sleep(Duration::from_secs(60)).await;
    }
}

async fn handler() -> &'static str {
    "Hello, world!"
}
