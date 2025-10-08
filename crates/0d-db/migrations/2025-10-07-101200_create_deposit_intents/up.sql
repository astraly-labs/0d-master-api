-- Deposit intents declared before on-chain deposit execution

CREATE TABLE deposit_intents (
    id TEXT PRIMARY KEY,
    partner_id TEXT NOT NULL,
    vault_id VARCHAR(50) NOT NULL REFERENCES vaults(id),
    chain_id BIGINT NOT NULL,
    receiver VARCHAR(100) NOT NULL,
    amount_dec NUMERIC(78, 30) NOT NULL,
    created_ts TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_ts TIMESTAMPTZ NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'matched', 'expired', 'orphan')),
    meta_json JSONB
);

CREATE INDEX di_receiver_time ON deposit_intents (receiver, created_ts);
CREATE INDEX di_status ON deposit_intents (status);

-- Attribution result, one row per confirmed transaction
CREATE TABLE attributions (
    tx_id INT PRIMARY KEY REFERENCES user_transactions(id) ON DELETE CASCADE,
    tx_hash VARCHAR(100) NOT NULL UNIQUE,
    intent_id TEXT REFERENCES deposit_intents(id) ON DELETE SET NULL,
    partner_id TEXT,
    source VARCHAR(16) NOT NULL CHECK (source IN ('explicit', 'inferred')),
    confidence NUMERIC(5, 4) NOT NULL,
    assets_dec NUMERIC(78, 30) NOT NULL,
    shares_dec NUMERIC(78, 30),
    created_ts TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- View of confirmed deposit transactions for attribution reconciliation
CREATE OR REPLACE VIEW confirmed_deposits AS
SELECT
    tx_hash,
    block_timestamp AS block_time,
    user_address AS receiver,
    vault_id,
    amount AS assets_dec,
    shares_amount AS shares_dec,
    status,
    type AS type
FROM user_transactions
WHERE type = 'deposit' AND status IN ('confirmed', 'success');
