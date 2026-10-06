ALTER TABLE payment_methods
DROP COLUMN IF EXISTS payment_method_subtype,
DROP COLUMN IF EXISTS payment_method_type_v2,
DROP COLUMN IF EXISTS id;
