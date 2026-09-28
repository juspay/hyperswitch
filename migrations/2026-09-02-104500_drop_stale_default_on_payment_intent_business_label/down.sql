-- Restores the default this migration removed. Reverting reintroduces the
-- divergence between a freshly migrated database and a long-running one.
ALTER TABLE payment_intent ALTER COLUMN business_label SET DEFAULT 'default';
