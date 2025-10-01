use std::fmt;

use starknet::core::types::{Felt, U256};

/// Item scheduled for execution by the relayer queue.
#[derive(Clone)]
pub struct QueueItem {
    pub priority: i64,
    pub kind: QueueItemKind,
}

impl QueueItem {
    pub const fn new(priority: i64, kind: QueueItemKind) -> Self {
        Self { priority, kind }
    }

    pub fn vault_id(&self) -> &str {
        match &self.kind {
            QueueItemKind::AumReport { vault_id, .. }
            | QueueItemKind::RedeemClaim { vault_id, .. } => vault_id.as_str(),
        }
    }

    pub fn task_type(&self) -> &'static str {
        match &self.kind {
            QueueItemKind::AumReport { .. } => "AUM_REPORT",
            QueueItemKind::RedeemClaim { .. } => "REDEEM_CLAIM",
        }
    }
}

impl fmt::Debug for QueueItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            QueueItemKind::AumReport {
                vault_id,
                contract_address,
                aum_provider,
            } => f
                .debug_struct("QueueItem::AumReport")
                .field("vault_id", vault_id)
                .field("contract_address", contract_address)
                .field("aum_provider", aum_provider)
                .field("priority", &self.priority)
                .finish(),
            QueueItemKind::RedeemClaim {
                vault_id,
                contract_address,
                redeem_ids,
            } => f
                .debug_struct("QueueItem::RedeemClaim")
                .field("vault_id", vault_id)
                .field("contract_address", contract_address)
                .field("redeem_ids", redeem_ids)
                .field("priority", &self.priority)
                .finish(),
        }
    }
}

/// Specific execution variants supported by the relayer.
#[derive(Clone)]
pub enum QueueItemKind {
    AumReport {
        vault_id: String,
        contract_address: Felt,
        aum_provider: Option<String>,
    },
    RedeemClaim {
        vault_id: String,
        contract_address: Felt,
        redeem_ids: Vec<U256>,
    },
}

impl QueueItemKind {
    pub fn key(&self) -> (&str, &'static str) {
        match self {
            QueueItemKind::AumReport { vault_id, .. } => (vault_id, "AUM_REPORT"),
            QueueItemKind::RedeemClaim { vault_id, .. } => (vault_id, "REDEEM_CLAIM"),
        }
    }

    pub fn contract_address(&self) -> Felt {
        match self {
            QueueItemKind::AumReport {
                contract_address, ..
            }
            | QueueItemKind::RedeemClaim {
                contract_address, ..
            } => *contract_address,
        }
    }
}

/// Priority based queue behaving similarly to the TypeScript implementation.
#[derive(Default)]
pub struct ExecutionQueue {
    items: Vec<QueueItem>,
}

impl ExecutionQueue {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn items(&self) -> &[QueueItem] {
        &self.items
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn push(&mut self, item: QueueItem) {
        let (vault_id, task_type) = item.kind.key();
        if let Some(existing) = self
            .items
            .iter_mut()
            .find(|existing| existing.kind.key() == (vault_id, task_type))
        {
            if existing.priority < item.priority {
                existing.priority = item.priority;
            }
        } else {
            self.items.push(item);
        }

        self.items.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.task_type().cmp(b.task_type()))
        });
    }

    pub fn pop(&mut self) -> Option<QueueItem> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.items.remove(0))
        }
    }
}
