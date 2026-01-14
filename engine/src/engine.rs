use crate::execution::TriggerEvent;
use crate::prices::PriceEvent;
use std::collections::{BTreeMap, HashSet};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub enum Status {
    Pending,
    Triggered,
    Completed,
    Failed,
}

impl std::str::FromStr for Status {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(Status::Pending),
            "triggered" => Ok(Status::Triggered),
            "completed" => Ok(Status::Completed),
            "failed" => Ok(Status::Failed),
            _ => Err(format!("Invalid status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub struct Intent {
    pub id: u32,
    pub trigger_price: u64,
    pub status: Status,
}

// Database representation
#[derive(Debug, sqlx::FromRow)]
pub struct IntentRow {
    pub id: i64,
    pub trigger_price: sqlx::types::Decimal,
    pub status: String,
}

impl From<IntentRow> for Intent {
    fn from(row: IntentRow) -> Self {
        let price_f64 = row.trigger_price.to_string().parse::<f64>().unwrap_or(0.0);
        Intent {
            id: row.id as u32,
            trigger_price: IntentEngine::to_bps(price_f64),
            status: row.status.parse().unwrap_or(Status::Pending),
        }
    }
}

pub struct IntentEngine {
    intents: BTreeMap<u64, Vec<Intent>>,
    known_intents: HashSet<u32>,
    triggered_intents: Vec<Intent>,
    last_price: Option<u64>,
    tolerance_bps: u64,
}

impl IntentEngine {
    pub fn new(tolerance_bps: u64) -> Self {
        Self {
            intents: BTreeMap::new(),
            known_intents: HashSet::new(),
            triggered_intents: Vec::new(),
            last_price: None,
            tolerance_bps,
        }
    }

    #[inline]
    pub fn to_bps(price: f64) -> u64 {
        (price * 1000.0).round() as u64
    }

    #[inline]
    pub fn from_bps(bps: u64) -> f64 {
        bps as f64 / 1000.0
    }

    pub fn add_intent(&mut self, intent: Intent) {
        if self.known_intents.contains(&intent.id) {
            return;
        }

        self.known_intents.insert(intent.id);
        self.intents
            .entry(intent.trigger_price)
            .or_default()
            .push(intent);
    }

    /// Load intents from database rows
    pub async fn load_intents(&mut self, intent_rows: Vec<IntentRow>) {
        let count_before = self.get_pending_count();

        for row in intent_rows {
            let intent: Intent = row.into();
            if intent.status == Status::Pending {
                self.add_intent(intent);
            }
        }

        let count_after = self.get_pending_count();
        println!("Loaded {} new pending intents", count_after - count_before);
    }

    pub fn remove_intent(&mut self, id: u32) -> bool {
        for bucket in self.intents.values_mut() {
            if let Some(pos) = bucket.iter().position(|i| i.id == id) {
                bucket.remove(pos);
                return true;
            }
        }
        false
    }

    pub fn get_pending_count(&self) -> usize {
        self.intents
            .values()
            .flat_map(|v| v.iter())
            .filter(|i| i.status == Status::Pending)
            .count()
    }

    pub fn mark_completed(&mut self, intent_id: u32) -> bool {
        if let Some(intent) = self
            .triggered_intents
            .iter_mut()
            .find(|i| i.id == intent_id)
        {
            intent.status = Status::Completed;
            true
        } else {
            false
        }
    }

    pub fn mark_failed(&mut self, intent_id: u32) -> bool {
        if let Some(intent) = self
            .triggered_intents
            .iter_mut()
            .find(|i| i.id == intent_id)
        {
            intent.status = Status::Failed;
            true
        } else {
            false
        }
    }

    pub fn get_triggered_intents(&self) -> &[Intent] {
        &self.triggered_intents
    }

    fn check_triggers(&mut self, event: &PriceEvent) -> Vec<Intent> {
        let mut triggered = Vec::new();
        let current_price_bps = Self::to_bps(event.price);

        let lower = current_price_bps.saturating_sub(self.tolerance_bps);
        let upper = current_price_bps.saturating_add(self.tolerance_bps);

        let trigger_prices: Vec<u64> = self.intents.range(lower..=upper).map(|(&k, _)| k).collect();

        for trigger_price in trigger_prices {
            if let Some(bucket) = self.intents.remove(&trigger_price) {
                for mut intent in bucket {
                    if intent.status != Status::Pending {
                        continue;
                    }

                    let should_trigger = if let Some(last) = self.last_price {
                        last > trigger_price && current_price_bps <= trigger_price
                    } else {
                        current_price_bps <= trigger_price
                    };

                    if should_trigger {
                        intent.status = Status::Triggered;
                        self.known_intents.remove(&intent.id);
                        triggered.push(intent.clone());
                        self.triggered_intents.push(intent);
                    } else {
                        self.intents.entry(trigger_price).or_default().push(intent);
                    }
                }
            }
        }

        self.intents.retain(|_, v| !v.is_empty());

        triggered
    }
}

// Database helper functions
pub async fn load_pending_intents(pool: &sqlx::PgPool) -> Result<Vec<IntentRow>, sqlx::Error> {
    sqlx::query_as::<_, IntentRow>(
        "SELECT id, trigger_price, status FROM intents WHERE status = 'pending' ORDER BY trigger_price ASC"
    )
    .fetch_all(pool)
    .await
}

pub async fn update_intent_status(
    pool: &sqlx::PgPool,
    intent_id: u32,
    status: Status,
) -> Result<(), sqlx::Error> {
    let status_str = match status {
        Status::Pending => "pending",
        Status::Triggered => "triggered",
        Status::Completed => "completed",
        Status::Failed => "failed",
    };

    sqlx::query("UPDATE intents SET status = $1 WHERE id = $2")
        .bind(status_str)
        .bind(intent_id as i64)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn trigger_engine(
    mut rx: mpsc::Receiver<PriceEvent>,
    exec_tx: mpsc::Sender<TriggerEvent>,
    mut reload_rx: mpsc::Receiver<()>,
    pool: sqlx::PgPool,
) {
    let mut engine = IntentEngine::new(50); // 0.05 tolerance = 50 basis points (0.05 * 1000 = 50)

    // Load intents from database
    println!("Loading intents from database...");
    match load_pending_intents(&pool).await {
        Ok(rows) => {
            println!("Found {} pending intents in database", rows.len());
            engine.load_intents(rows).await;
        }
        Err(e) => {
            eprintln!("Failed to load intents from database: {}", e);
            return;
        }
    }

    println!(
        "Intent engine started with {} pending intents (tolerance: 0.05)",
        engine.get_pending_count()
    );

    loop {
        tokio::select! {
            Some(event) = rx.recv() => {
                println!("PriceEvent: {:?}", event);

                let triggered = engine.check_triggers(&event);

                for intent in triggered {
                    println!(
                        "Triggering BUY intent {} at price {} (target was {})",
                        intent.id,
                        event.price,
                        IntentEngine::from_bps(intent.trigger_price)
                    );

                    if let Err(e) = update_intent_status(&pool, intent.id, Status::Triggered).await {
                        eprintln!("Failed to update intent {} status in DB: {}", intent.id, e);
                    }

                    if let Err(e) = exec_tx
                        .send(TriggerEvent {
                            intent_id: intent.id,
                            price: event.price,
                        })
                        .await
                    {
                        eprintln!(
                            "Failed to send trigger event for intent {}: {}",
                            intent.id, e
                        );
                    }
                }

                engine.last_price = Some(IntentEngine::to_bps(event.price));
            }

            Some(_) = reload_rx.recv() => {
                println!("Reloading intents from database");
                match load_pending_intents(&pool).await {
                    Ok(rows) => {
                        engine.load_intents(rows).await;
                        println!(
                            "Reload complete. Now tracking {} pending intents",
                            engine.get_pending_count()
                        );
                    }
                    Err(e) => {
                        eprintln!("Failed to reload intents: {}", e);
                    }
                }
            }

            else => break,
        }
    }

    println!("Intent engine shutting down");
}
