-- This file should undo anything in `up.sql`
-- We do not delete the enum variant from the type
-- This is added so that the revert command succeeds
SELECT version();
