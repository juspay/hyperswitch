-- Populate the id column for existing V1-created customer rows where id is NULL.
-- During the V1/V2 coexistence period, the V2 binary looks up customers by the
-- id column (WHERE id = $1), so V1-created customers need id populated with the
-- same value as customer_id.
-- V2-created rows already have id populated on insert, so they are unaffected.
UPDATE customers SET id = customer_id WHERE id IS NULL AND version = 'v1';
