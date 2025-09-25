-- Add 'synced' status to indexer_state table
ALTER TABLE indexer_state 
DROP CONSTRAINT IF EXISTS indexer_state_status_check;

ALTER TABLE indexer_state 
ADD CONSTRAINT indexer_state_status_check 
CHECK (status IN ('active', 'paused', 'error', 'synced'));