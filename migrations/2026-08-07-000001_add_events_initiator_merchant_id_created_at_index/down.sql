-- This file should undo anything in `up.sql`
DROP INDEX CONCURRENTLY IF EXISTS events_initiator_merchant_id_created_at_initial_index;
