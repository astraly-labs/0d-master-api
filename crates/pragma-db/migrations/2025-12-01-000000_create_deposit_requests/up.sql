CREATE TABLE deposit_requests (
    id VARCHAR(36) PRIMARY KEY,
    vault_id VARCHAR(50) NOT NULL REFERENCES vaults(id),
    user_address VARCHAR(100) NOT NULL,
    amount NUMERIC(36, 18) NOT NULL,
    referral_code VARCHAR(100),
    transaction JSONB NOT NULL,
    tx_hash VARCHAR(100),
    status VARCHAR(20) NOT NULL,
    error_code VARCHAR(64),
    error_message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_deposit_requests_vault_id ON deposit_requests(vault_id);
CREATE INDEX idx_deposit_requests_user_address ON deposit_requests(user_address);
CREATE INDEX idx_deposit_requests_tx_hash ON deposit_requests(tx_hash);
