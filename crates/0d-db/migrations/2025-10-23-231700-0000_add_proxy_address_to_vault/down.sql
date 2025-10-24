-- This file should undo anything in `up.sql`
ALTER TABLE vaults DROP COLUMN proxy_address;
ALTER TABLE user_transactions DROP COLUMN partner_id;