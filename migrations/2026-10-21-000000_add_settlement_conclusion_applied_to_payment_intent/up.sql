ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS settlement_conclusion_applied BOOLEAN;
