ALTER TABLE roles ADD COLUMN IF NOT EXISTS merchant_product_type VARCHAR(64);

UPDATE roles SET merchant_product_type = 'orchestration' where merchant_product_type IS NULL;
