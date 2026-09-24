-- Your SQL goes here
-- No query in the codebase filters payment_attempt by preprocessing_step_id anymore
-- (see find_by_processor_merchant_id_preprocessing_id, removed alongside this migration).
DROP INDEX IF EXISTS preprocessing_step_id_index;
