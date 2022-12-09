-- Your SQL goes here

CREATE TYPE "MerchantStorageScheme" AS ENUM (
    'postgres_only',
    'redis_kv'
);

ALTER TABLE merchant_account ADD COLUMN storage_scheme "MerchantStorageScheme" NOT NULL DEFAULT 'postgres_only';
