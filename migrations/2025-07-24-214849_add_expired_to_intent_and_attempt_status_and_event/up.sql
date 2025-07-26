ALTER TYPE "IntentStatus" ADD VALUE IF NOT EXISTS 'expired';

ALTER TYPE "AttemptStatus" ADD VALUE IF NOT EXISTS 'expired';

ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'payment_expired';