-- Your SQL goes here
-- =====================================================
-- 0D Finance Vault Aggregator Database Schema
-- =====================================================
-- This schema handles user data (indexed from blockchain)
-- and vault metadata. Vault performance data comes from
-- individual vault APIs.
-- =====================================================

-- =====================================================
-- CORE TABLES
-- =====================================================

-- Vault registry and metadata
CREATE TABLE vaults (
    id VARCHAR(50) PRIMARY KEY, -- e.g., 'starknet-elp'
    name VARCHAR(255) NOT NULL,
    description TEXT,
    chain VARCHAR(50) NOT NULL DEFAULT 'starknet',
    chain_id VARCHAR(50), -- for future multi-chain support
    symbol VARCHAR(20) NOT NULL, -- e.g., '0D-ELP'
    base_asset VARCHAR(20) NOT NULL DEFAULT 'USDC',
    status VARCHAR(20) NOT NULL CHECK (status IN ('live', 'paused', 'retired')),
    inception_date DATE,
    
    -- Contract info
    contract_address VARCHAR(100) NOT NULL,
    
    -- Fee structure
    mgmt_fee_bps INTEGER DEFAULT 0,
    perf_fee_bps INTEGER NOT NULL,
    
    -- Documentation
    strategy_brief TEXT,
    docs_url VARCHAR(500),
    
    -- Deposit constraints
    min_deposit DECIMAL(36, 18),
    max_deposit DECIMAL(36, 18),
    deposit_paused BOOLEAN DEFAULT FALSE,
    
    -- Withdraw constraints  
    instant_liquidity BOOLEAN DEFAULT FALSE,
    instant_slippage_max_bps INTEGER,
    redeem_24h_threshold_pct_of_aum DECIMAL(5, 2),
    redeem_48h_above_threshold BOOLEAN DEFAULT FALSE,
    
    -- Icons
    icon_light_url VARCHAR(500),
    icon_dark_url VARCHAR(500),
    
    -- Vault API endpoint
    api_endpoint VARCHAR(500) NOT NULL, -- Individual vault API URL
    
    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_vaults_status ON vaults(status);
CREATE INDEX idx_vaults_chain ON vaults(chain);

-- =====================================================
-- USER TABLES (Filled by indexer from blockchain events)
-- =====================================================

-- User registry
CREATE TABLE users (
    address VARCHAR(100) PRIMARY KEY,
    chain VARCHAR(50) NOT NULL DEFAULT 'starknet',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_users_chain ON users(chain);

-- User positions in vaults (current state)
CREATE TABLE user_positions (
    id SERIAL PRIMARY KEY,
    user_address VARCHAR(100) NOT NULL REFERENCES users(address),
    vault_id VARCHAR(50) NOT NULL REFERENCES vaults(id),
    
    -- Position data (updated by indexer on each transaction)
    share_balance DECIMAL(36, 18) NOT NULL DEFAULT 0,
    cost_basis DECIMAL(36, 18) NOT NULL DEFAULT 0, -- Total deposits in base asset
    
    -- Timestamps
    first_deposit_at TIMESTAMPTZ,
    last_activity_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(user_address, vault_id)
);

CREATE INDEX idx_user_positions_user ON user_positions(user_address);
CREATE INDEX idx_user_positions_vault ON user_positions(vault_id);
CREATE INDEX idx_user_positions_balance ON user_positions(share_balance) WHERE share_balance > 0;

-- User transactions (indexed from blockchain)
CREATE TABLE user_transactions (
    id SERIAL PRIMARY KEY,
    tx_hash VARCHAR(100) NOT NULL,
    block_number BIGINT NOT NULL,
    block_timestamp TIMESTAMPTZ NOT NULL,
    
    user_address VARCHAR(100) NOT NULL REFERENCES users(address),
    vault_id VARCHAR(50) NOT NULL REFERENCES vaults(id),
    
    type VARCHAR(20) NOT NULL CHECK (type IN ('deposit', 'withdraw', 'fee', 'rebalance')),
    status VARCHAR(20) NOT NULL CHECK (status IN ('pending', 'confirmed', 'failed')),
    
    -- Amounts
    amount DECIMAL(36, 18) NOT NULL, -- In base asset
    shares_amount DECIMAL(36, 18), -- Share tokens involved
    share_price DECIMAL(36, 18), -- Share price at transaction time
    
    -- Additional transaction data
    gas_fee DECIMAL(36, 18),
    metadata JSONB, -- For any additional chain-specific data
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_user_transactions_user ON user_transactions(user_address);
CREATE INDEX idx_user_transactions_vault ON user_transactions(vault_id);
CREATE INDEX idx_user_transactions_type ON user_transactions(type);
CREATE INDEX idx_user_transactions_block ON user_transactions(block_number DESC);
CREATE INDEX idx_user_transactions_timestamp ON user_transactions(block_timestamp DESC);
CREATE INDEX idx_user_transactions_tx_hash ON user_transactions(tx_hash);

-- =====================================================
-- CALCULATED/CACHED TABLES
-- =====================================================

-- User KPIs (calculated periodically or on-demand)
CREATE TABLE user_kpis (
    id SERIAL PRIMARY KEY,
    user_address VARCHAR(100) NOT NULL REFERENCES users(address),
    vault_id VARCHAR(50) NOT NULL REFERENCES vaults(id),
    
    -- P&L metrics (in USD)
    all_time_pnl DECIMAL(36, 18),
    unrealized_pnl DECIMAL(36, 18),
    realized_pnl DECIMAL(36, 18),
    
    -- Risk metrics
    max_drawdown_pct DECIMAL(5, 2),
    sharpe_ratio DECIMAL(10, 4),
    
    -- Activity metrics
    total_deposits DECIMAL(36, 18),
    total_withdrawals DECIMAL(36, 18),
    total_fees_paid DECIMAL(36, 18),
    
    -- Calculation metadata
    calculated_at TIMESTAMPTZ DEFAULT NOW(),
    share_price_used DECIMAL(36, 18), -- Current share price used for calculations
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(user_address, vault_id)
);

CREATE INDEX idx_user_kpis_user ON user_kpis(user_address);
CREATE INDEX idx_user_kpis_vault ON user_kpis(vault_id);
CREATE INDEX idx_user_kpis_calculated ON user_kpis(calculated_at);

-- =====================================================
-- INDEXER TRACKING
-- =====================================================

-- Track indexer progress per vault
CREATE TABLE indexer_state (
    id SERIAL PRIMARY KEY,
    vault_id VARCHAR(50) NOT NULL REFERENCES vaults(id),
    last_processed_block BIGINT NOT NULL DEFAULT 0,
    last_processed_timestamp TIMESTAMPTZ,
    last_error TEXT,
    last_error_at TIMESTAMPTZ,
    status VARCHAR(20) DEFAULT 'active' CHECK (status IN ('active', 'paused', 'error')),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(vault_id)
);

CREATE INDEX idx_indexer_state_vault ON indexer_state(vault_id);
CREATE INDEX idx_indexer_state_status ON indexer_state(status);

-- =====================================================
-- AUDIT & MONITORING
-- =====================================================

-- API call logs (optional, for monitoring)
CREATE TABLE api_logs (
    id BIGSERIAL PRIMARY KEY,
    endpoint VARCHAR(255) NOT NULL,
    method VARCHAR(10) NOT NULL,
    user_address VARCHAR(100),
    vault_id VARCHAR(50),
    response_time_ms INTEGER,
    status_code INTEGER,
    error_message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Partition by month for efficient cleanup
CREATE INDEX idx_api_logs_created ON api_logs(created_at DESC);
CREATE INDEX idx_api_logs_endpoint ON api_logs(endpoint);
CREATE INDEX idx_api_logs_user ON api_logs(user_address) WHERE user_address IS NOT NULL;

-- =====================================================
-- FUNCTIONS & TRIGGERS
-- =====================================================

-- Update timestamp trigger
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply update trigger to all relevant tables
CREATE TRIGGER update_vaults_updated_at BEFORE UPDATE ON vaults
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_positions_updated_at BEFORE UPDATE ON user_positions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_transactions_updated_at BEFORE UPDATE ON user_transactions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_kpis_updated_at BEFORE UPDATE ON user_kpis
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_indexer_state_updated_at BEFORE UPDATE ON indexer_state
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- =====================================================
-- VIEWS (for common queries)
-- =====================================================

-- Active user positions with current value
CREATE VIEW active_user_positions AS
SELECT 
    up.user_address,
    up.vault_id,
    v.name as vault_name,
    v.symbol as vault_symbol,
    up.share_balance,
    up.cost_basis,
    up.first_deposit_at,
    up.last_activity_at,
    v.status as vault_status
FROM user_positions up
JOIN vaults v ON up.vault_id = v.id
WHERE up.share_balance > 0
    AND v.status = 'live';

-- User transaction summary
CREATE VIEW user_transaction_summary AS
SELECT 
    user_address,
    vault_id,
    COUNT(*) as total_transactions,
    COUNT(CASE WHEN type = 'deposit' THEN 1 END) as deposit_count,
    COUNT(CASE WHEN type = 'withdraw' THEN 1 END) as withdraw_count,
    SUM(CASE WHEN type = 'deposit' THEN amount ELSE 0 END) as total_deposited,
    SUM(CASE WHEN type = 'withdraw' THEN amount ELSE 0 END) as total_withdrawn,
    MAX(block_timestamp) as last_transaction_at
FROM user_transactions
WHERE status = 'confirmed'
GROUP BY user_address, vault_id;

-- =====================================================
-- INDEXES FOR COMMON QUERY PATTERNS
-- =====================================================

-- For getUserVaultSummary - quick lookup of user position
CREATE INDEX idx_user_pos_lookup ON user_positions(user_address, vault_id) 
    INCLUDE (share_balance, cost_basis, first_deposit_at);

-- For transaction history with pagination
CREATE INDEX idx_user_tx_pagination ON user_transactions(user_address, vault_id, block_timestamp DESC, id);

-- For calculating user KPIs
CREATE INDEX idx_user_tx_for_kpis ON user_transactions(user_address, vault_id, type, status)
    WHERE status = 'confirmed';

-- =====================================================
-- COMMENTS FOR DOCUMENTATION
-- =====================================================

COMMENT ON TABLE vaults IS 'Registry of all vaults with metadata and configuration';
COMMENT ON TABLE users IS 'User registry indexed from blockchain';
COMMENT ON TABLE user_positions IS 'Current user positions in vaults, updated by indexer';
COMMENT ON TABLE user_transactions IS 'All user transactions indexed from blockchain events';
COMMENT ON TABLE user_kpis IS 'Calculated KPIs for users, can be refreshed periodically';
COMMENT ON TABLE indexer_state IS 'Track indexer progress for each vault';
COMMENT ON TABLE api_logs IS 'Optional API monitoring and debugging';

COMMENT ON COLUMN vaults.api_endpoint IS 'Individual vault API endpoint to fetch performance data';
COMMENT ON COLUMN user_positions.cost_basis IS 'Total amount deposited minus withdrawals in base asset';
COMMENT ON COLUMN user_transactions.metadata IS 'Flexible JSONB for chain-specific or additional data';