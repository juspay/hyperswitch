-- This file should undo anything in `up.sql`
-- We do not delete the enum variant from the type
ALTER TYPE "EventType" RENAME VALUE 'merchant_action_required' TO 'action_required';
