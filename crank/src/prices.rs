use reqwest::Url;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::{time::Duration, time::sleep};

#[derive(Debug)]
pub struct PriceEvent {
    pub price: f64,
    pub ts: u64,
}

pub async fn price_ingestor(tx: tokio::sync::mpsc::Sender<PriceEvent>) {
    let url = Url::parse(
        "https://lite-api.jup.ag/price/v3?ids=So11111111111111111111111111111111111111112",
    )
    .unwrap();

    loop {
        match reqwest::get(url.clone()).await {
            Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                Ok(json) => {
                    if let Some(price) =
                        json["So11111111111111111111111111111111111111112"]["usdPrice"].as_f64()
                    {
                        let price = (price * 1000.0).round() / 1000.0;
                        let ts = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();

                        let event = PriceEvent { price, ts };
                        let _ = tx.send(event).await;
                    }
                }
                Err(e) => eprintln!("JSON parse error: {:?}", e),
            },
            Ok(resp) => eprintln!("HTTP error: {}", resp.status()),
            Err(e) => eprintln!("Request error: {:?}", e),
        }

        sleep(Duration::from_secs(1)).await;
    }
}
