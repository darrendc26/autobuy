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
    let (reload_tx, reload_rx) = tokio::sync::mpsc::channel(10);

    let pool = sqlx::PgPool::connect("postgres://postgres:postgres@localhost:5432/intents")
        .await
        .unwrap();

    // Spawn trigger engine
    let engine_pool = pool.clone();
    tokio::spawn(async move {
        trigger_engine(price_rx, exec_tx, reload_rx, engine_pool).await;
    });

    // Spawn price ingestor
    tokio::spawn(price_ingestor(price_tx));

    // Spawn execution worker with DB pool
    let exec_pool = pool.clone();
    tokio::spawn(execution_worker(exec_rx, exec_pool));

    // Spawn periodic reload (every 30 seconds)
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            println!("Triggering intent reload...");
            if reload_tx.send(()).await.is_err() {
                println!("Engine shut down, stopping reload task");
                break;
            }
        }
    });

    // Spawn API server
    tokio::spawn(api_server());

    println!("All workers started");
    println!(" - Price ingestor: running");
    println!(" - Trigger engine: running");
    println!(" - Execution worker: running");
    println!(" - Periodic reload: every 30s");
    println!(" - API server: running");

    // keep main alive (supervisor role)
    loop {
        sleep(Duration::from_secs(60)).await;
    }
}
