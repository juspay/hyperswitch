-- Active: 1754937758962@@127.0.0.1@5432@hyperswitch_db
-- Your SQL goes here
ALTER TABLE payment_methods ADD COLUMN IF NOT EXISTS external_vault_source VARCHAR(64);