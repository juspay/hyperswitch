-- Your SQL goes here

CREATE TYPE "ApiVersion" AS ENUM ('v1', 'v2');

ALTER TABLE customers ADD COLUMN IF NOT EXISTS version "ApiVersion" NOT NULL DEFAULT 'v1';