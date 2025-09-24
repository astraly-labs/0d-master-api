CREATE TABLE user_portfolio_history (
    id SERIAL PRIMARY KEY,
    user_address VARCHAR(100) NOT NULL,
    vault_id VARCHAR(50) NOT NULL,
    portfolio_value DECIMAL(36, 18) NOT NULL,
    share_balance DECIMAL(36, 18) NOT NULL,
    share_price DECIMAL(36, 18) NOT NULL,
    calculated_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for efficient queries
CREATE INDEX idx_portfolio_history_lookup ON user_portfolio_history(user_address, vault_id, calculated_at);
CREATE INDEX idx_portfolio_history_date ON user_portfolio_history(calculated_at);
CREATE INDEX idx_portfolio_history_user_vault ON user_portfolio_history(user_address, vault_id);