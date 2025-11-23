use reqwest::Url;
use serde_json::Value;
use tokio::{time::Duration, time::sleep};

fn main() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let url = Url::parse(
            "https://lite-api.jup.ag/price/v3?ids=So11111111111111111111111111111111111111112",
        )
        .unwrap();

        loop {
            // fetch new data EACH iteration
            let response = reqwest::get(url.clone()).await.unwrap();
            let data: Value = response.json().await.unwrap();

            let usd_price = data["So11111111111111111111111111111111111111112"]["usdPrice"]
                .as_f64()
                .unwrap();

            println!("USD Price: {}", usd_price);

            sleep(Duration::from_millis(10)).await;
        }
    });
}
