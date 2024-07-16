-- Your SQL goes here
ALTER TABLE payouts ADD COLUMN IF NOT EXISTS client_secret VARCHAR(128) DEFAULT NULL;

ALTER TYPE "PayoutStatus" ADD VALUE IF NOT EXISTS 'requires_confirmation';