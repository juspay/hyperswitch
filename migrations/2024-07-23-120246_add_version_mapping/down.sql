-- This file should undo anything in `up.sql`

DROP TYPE "ApiVersion";
ALTER TABLE customers DROP COLUMN version;