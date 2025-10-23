-- Your SQL goes here
ALTER TABLE vaults ADD COLUMN proxy_address VARCHAR(100) NULL;
ALTER TABLE user_transactions ADD COLUMN partner_id VARCHAR(100) NULL;

-- TODO: Ask if we need to index that one --
-- CREATE INDEX CONCURRENTLY idx_user_partner_id ON user_transactions(partner_id, created_at);