-- The hyperswitch_ai_interaction table was part of the experimental AI chat feature.
-- The feature was never enabled in production, so the table and its partitions are dropped.
-- CASCADE drops all child partitions, including any manually created quarterly partitions.
DROP TABLE IF EXISTS hyperswitch_ai_interaction CASCADE;
