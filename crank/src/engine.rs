use crate::execution::TriggerEvent;
use crate::prices::PriceEvent;
use std::collections::BTreeMap;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub enum Status {
    Pending,
    Triggered,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Intent {
    pub id: u32,
    pub trigger_price: u64,
    pub status: Status,
}

pub struct IntentEngine {
    intents: BTreeMap<u64, Vec<Intent>>,
    triggered_intents: Vec<Intent>,
    last_price: Option<u64>,
    tolerance_bps: u64,
}

impl IntentEngine {
    pub fn new(tolerance_bps: u64) -> Self {
        Self {
            intents: BTreeMap::new(),
            triggered_intents: Vec::new(),
            last_price: None,
            tolerance_bps,
        }
    }

    #[inline]
    fn to_bps(price: f64) -> u64 {
        (price * 1000.0).round() as u64
    }

    #[inline]
    fn from_bps(bps: u64) -> f64 {
        bps as f64 / 1000.0
    }

    pub fn add_intent(&mut self, intent: Intent) {
        self.intents
            .entry(intent.trigger_price)
            .or_default()
            .push(intent);
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

pub async fn trigger_engine(
    mut rx: mpsc::Receiver<PriceEvent>,
    exec_tx: mpsc::Sender<TriggerEvent>,
) {
    let mut engine = IntentEngine::new(50); // 0.05 tolerance = 50 basis points

    for intent in [
        Intent {
            id: 1,
            trigger_price: 142_490,
            status: Status::Pending,
        },
        Intent {
            id: 2,
            trigger_price: 142_575,
            status: Status::Pending,
        },
        Intent {
            id: 3,
            trigger_price: 142_650,
            status: Status::Pending,
        },
    ] {
        engine.add_intent(intent);
    }

    println!(
        "Intent engine started with {} pending intents (tolerance: 0.05)",
        engine.get_pending_count()
    );

    // --- main loop ---
    while let Some(event) = rx.recv().await {
        println!("PriceEvent: {:?}", event);

        let triggered = engine.check_triggers(&event);

        for intent in triggered {
            println!(
                "✓ Triggering BUY intent {} at price {} (target was {})",
                intent.id,
                event.price,
                IntentEngine::from_bps(intent.trigger_price)
            );

            if let Err(e) = exec_tx
                .send(TriggerEvent {
                    // intent_id: intent.id,
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

    println!("Intent engine shutting down");
}
