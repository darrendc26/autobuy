use crate::engine;

#[derive(Debug)]
pub struct TriggerEvent {
    pub intent_id: u32,
    pub price: f64,
}

pub async fn execution_worker(
    mut rx: tokio::sync::mpsc::Receiver<TriggerEvent>,
    pool: sqlx::PgPool,
) {
    while let Some(event) = rx.recv().await {
        println!("EXECUTING at price: {}", event.price);

        // ... your execution logic ...

        // Update DB after execution
        if let Err(e) =
            engine::update_intent_status(&pool, event.intent_id, engine::Status::Completed).await
        {
            eprintln!("Failed to update intent {} status: {}", event.intent_id, e);
        } else {
            println!("EXECUTED intent {}", event.intent_id);
        }
    }
}
