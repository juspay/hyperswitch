-- This file should undo anything in `up.sql`
ALTER TABLE merchant_account DROP COLUMN recon_status;
DROP TYPE "ReconStatus";