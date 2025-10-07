-- Create indexes on id columns for better query performance
-- These indexes will improve performance for queries that filter or join on the id columns

CREATE INDEX IF NOT EXISTS customers_id_index ON customers (id);

CREATE INDEX IF NOT EXISTS payment_intent_id_index ON payment_intent (id);

CREATE INDEX IF NOT EXISTS payment_attempt_id_index ON payment_attempt (id);

CREATE INDEX IF NOT EXISTS payment_methods_id_index ON payment_methods (id);

CREATE INDEX IF NOT EXISTS refund_id_index ON refund (id);
