-- Your SQL goes here
ALTER TABLE dispute ADD COLUMN IF NOT EXISTS dispute_currency TYPE "Currency" USING currency::"Currency"; -- Migration query to be run after deployment before running this query