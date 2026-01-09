use crate::execution::TriggerEvent;
use crate::prices::PriceEvent;

pub async fn trigger_engine(
    mut rx: tokio::sync::mpsc::Receiver<PriceEvent>,
    exec_tx: tokio::sync::mpsc::Sender<TriggerEvent>,
) {
    let trigger_price = 137.665;
    let mut triggered = false;

    while let Some(event) = rx.recv().await {
        println!("PriceEvent: {:?}", event);

        if !triggered && event.price <= trigger_price {
            triggered = true;
            println!("Trigger condition met!");

            let _ = exec_tx.send(TriggerEvent { price: event.price }).await;
        }
    }
}
