CREATE TYPE "FraudCheckLastStep" AS ENUM (
    'processing',
    'checkout_or_sale',
    'transaction_or_record_refund',
    'fullfillment'
);

alter table fraud_check add column last_step "FraudCheckLastStep" NOT NULL DEFAULT 'processing'::"FraudCheckLastStep";