
CREATE TYPE "PaymentChannel" AS ENUM (
    'ecommerce',
    'mail_order',
    'telephone_order'
);
ALTER TABLE payment_intent ADD COLUMN IF NOT EXISTS payment_channel "PaymentChannel";
