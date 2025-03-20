-- This file should undo anything in `up.sql`

ALTER TABLE relay
    ALTER COLUMN created_at SET DEFAULT now()::TIMESTAMP,
    ALTER COLUMN modified_at SET DEFAULT now()::TIMESTAMP;