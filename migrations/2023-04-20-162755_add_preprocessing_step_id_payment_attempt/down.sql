-- This file should undo anything in `up.sql`
DROP INDEX preprocessing_step_id_index;
ALTER TABLE payment_attempt DROP COLUMN preprocessing_step_id;
