ALTER TYPE "AttemptStatus" ADD VALUE IF NOT EXISTS 'unresolved';
ALTER TYPE "IntentStatus" ADD VALUE IF NOT EXISTS 'requires_merchant_action' after 'requires_customer_action';
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'action_required';
ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'payment_processing';
