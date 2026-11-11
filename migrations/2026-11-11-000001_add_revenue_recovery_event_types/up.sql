-- Your SQL goes here
ALTER TYPE "EventClass"
ADD VALUE IF NOT EXISTS 'revenue_recovery';

ALTER TYPE "EventType"
ADD VALUE IF NOT EXISTS 'recovery_retry_scheduled';

ALTER TYPE "EventType"
ADD VALUE IF NOT EXISTS 'recovery_limits_exhausted';

ALTER TYPE "EventObjectType"
ADD VALUE IF NOT EXISTS 'recovery_retry_details';
