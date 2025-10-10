-- Your SQL goes here
-- =====================================================
-- VAULT REPORTS TABLE
-- =====================================================
-- Tracks vault report events (epoch processing, AUM updates, fee minting)

CREATE TABLE vault_reports (
    id SERIAL PRIMARY KEY,
    tx_hash VARCHAR(100) NOT NULL,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,

    vault_id VARCHAR(50) NOT NULL REFERENCES vaults(id),

    -- Report data from ReportEvent
    new_epoch DECIMAL(36, 18) NOT NULL,
    new_handled_epoch_len DECIMAL(36, 18) NOT NULL,
    total_supply DECIMAL(36, 18) NOT NULL,
    total_aum DECIMAL(36, 18) NOT NULL,
    management_fee_shares DECIMAL(36, 18) NOT NULL,
    performance_fee_shares DECIMAL(36, 18) NOT NULL,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_vault_reports_vault ON vault_reports(vault_id);
CREATE INDEX idx_vault_reports_block ON vault_reports(block_number DESC);
CREATE INDEX idx_vault_reports_timestamp ON vault_reports(block_timestamp DESC);
CREATE INDEX idx_vault_reports_tx_hash ON vault_reports(tx_hash);
CREATE INDEX idx_vault_reports_epoch ON vault_reports(vault_id, new_epoch DESC);

-- Apply update trigger
CREATE TRIGGER update_vault_reports_updated_at BEFORE UPDATE ON vault_reports
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

COMMENT ON TABLE vault_reports IS 'Vault report events indexed from blockchain, tracking epoch processing and fee minting';
