use tokio::{time::Duration, time::sleep};

#[derive(Debug)]
pub struct TriggerEvent {
    pub price: f64,
}

pub async fn execution_worker(mut rx: tokio::sync::mpsc::Receiver<TriggerEvent>) {
    while let Some(event) = rx.recv().await {
        println!("EXECUTING at price: {}", event.price);
        sleep(Duration::from_millis(500)).await;
        println!("EXECUTED");
    }
}
