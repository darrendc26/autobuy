use axum::{Json, Router, routing::post};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Deserialize, Serialize, Debug)]
struct Intent {
    trigger_price: f64,
}

pub async fn api_server() {
    let app = Router::new().route("/", post(create_intent));
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn create_intent(Json(intent): Json<Intent>) -> Json<String> {
    println!("Received intent: {:?}", intent);

    let pool = sqlx::PgPool::connect("postgres://postgres:postgres@localhost:5432/intents")
        .await
        .unwrap();

    let intent_id: i64 =
        sqlx::query_scalar("INSERT INTO intents (trigger_price) VALUES ($1) RETURNING id")
            .bind(intent.trigger_price)
            .fetch_one(&pool)
            .await
            .unwrap();

    Json(format!("Intent created with ID {}", intent_id))
}
