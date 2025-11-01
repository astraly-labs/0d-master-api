-- Your SQL goes here
ALTER TABLE vaults ADD COLUMN proxy_address VARCHAR(100) NULL;
ALTER TABLE user_transactions ADD COLUMN partner_id VARCHAR(100) NULL;
