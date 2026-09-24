-- This file should undo anything in `up.sql`
CREATE INDEX preprocessing_step_id_index ON payment_attempt (preprocessing_step_id);
