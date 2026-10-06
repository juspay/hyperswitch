-- This file should undo anything in `up.sql`
-- CASCADE will automatically drop all child partitions
DROP TABLE IF EXISTS hyperswitch_ai_interaction CASCADE;
