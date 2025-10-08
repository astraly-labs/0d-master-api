use std::sync::Arc;

use opentelemetry::{KeyValue, global, metrics::Counter};

#[derive(Debug)]
pub struct MetricsRegistry {
    pub deposits: Arc<DepositMetrics>,
}

impl MetricsRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            deposits: DepositMetrics::new(),
        })
    }
}

#[derive(Debug)]
pub struct DepositMetrics {
    intents_created: Counter<u64>,
    intents_matched: Counter<u64>,
}

impl DepositMetrics {
    fn new() -> Arc<Self> {
        let meter = global::meter("0d-master-api");
        let intents_created = meter
            .u64_counter("deposit_intents_created_total")
            .with_description("Number of off-chain deposit intents created")
            .with_unit("count")
            .init();

        let intents_matched = meter
            .u64_counter("deposit_intents_matched_total")
            .with_description("Number of off-chain deposit intents successfully matched")
            .with_unit("count")
            .init();

        Arc::new(Self {
            intents_created,
            intents_matched,
        })
    }

    pub fn record_intent_created(&self, vault_id: &str, chain_id: i64, partner_id: &str) {
        self.intents_created.add(
            1,
            &[
                KeyValue::new("vault_id", vault_id.to_string()),
                KeyValue::new("chain_id", chain_id.to_string()),
                KeyValue::new("partner_id", partner_id.to_string()),
            ],
        );
    }

    pub fn record_intent_matched(
        &self,
        vault_id: &str,
        chain_id: i64,
        partner_id: Option<&str>,
        source: AttributionSource,
    ) {
        let mut attributes = vec![
            KeyValue::new("vault_id", vault_id.to_string()),
            KeyValue::new("chain_id", chain_id.to_string()),
            KeyValue::new("source", source.as_str().to_string()),
        ];

        if let Some(partner) = partner_id {
            attributes.push(KeyValue::new("partner_id", partner.to_string()));
        }

        self.intents_matched.add(1, &attributes);
    }
}

#[derive(Clone, Copy, Debug)]
pub enum AttributionSource {
    Explicit,
    Inferred,
}

impl AttributionSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::Inferred => "inferred",
        }
    }
}
