-- Your SQL goes here

CREATE TYPE version_enum AS ENUM ('v1', 'v2');

ALTER TABLE customers ADD COLUMN version version_enum;