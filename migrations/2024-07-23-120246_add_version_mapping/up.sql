-- Your SQL goes here

CREATE TYPE api_version AS ENUM ('v1', 'v2');

ALTER TABLE customers ADD COLUMN version api_version NOT NULL DEFAULT 'v1';