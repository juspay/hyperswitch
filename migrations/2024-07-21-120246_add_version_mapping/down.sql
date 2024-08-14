-- This file should undo anything in `up.sql`
ALTER TABLE customers DROP COLUMN version;
DROP TYPE "ApiVersion";