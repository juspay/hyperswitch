-- This file should undo anything in `up.sql`
-- Drop the new column
ALTER TABLE dispute
DROP COLUMN IF EXISTS dispute_amount;

-- Optionally, if you want to revert the UPDATE statement as well (assuming you have a backup)
-- You can restore the original data from the backup or use the old values to update the table

-- For example, if you have a backup and want to revert the changes made by the UPDATE statement
-- You can replace the data with the backup data or the original values
-- For demonstration purposes, we're assuming you have a backup table named dispute_backup

-- Restore the original values from the backup or any other source
-- UPDATE dispute
-- SET dispute_amount = backup.dispute_amount
-- FROM dispute_backup AS backup
-- WHERE dispute.id = backup.id;
