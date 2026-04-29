-- Revert: clear the id column for all V1-created rows.
-- V2-created rows (version = 'v2') always populate id on insert and are left untouched.
UPDATE customers SET id = NULL WHERE version = 'v1';
