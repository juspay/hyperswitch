-- Your SQL goes here

ALTER TABLE relay
    ALTER COLUMN created_at DROP DEFAULT,
    ALTER COLUMN modified_at DROP DEFAULT;