ALTER TYPE "PayoutStatus"
ADD
    VALUE IF NOT EXISTS 'requires_customer_action';

ALTER TYPE "PayoutStatus"
ADD
    VALUE IF NOT EXISTS 'outgoing_payment_sent';

ALTER TYPE "PayoutStatus" ADD VALUE IF NOT EXISTS 'funds_refunded';

ALTER TYPE "PayoutStatus" ADD VALUE IF NOT EXISTS 'expired';

ALTER TYPE "PayoutStatus" RENAME VALUE 'pending' TO 'processing';