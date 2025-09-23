-- Add share_balance column to user_kpis table
-- This will store the share balance at the time of each KPI calculation
-- for accurate historical portfolio value reconstruction

ALTER TABLE user_kpis ADD COLUMN share_balance DECIMAL(36,18);