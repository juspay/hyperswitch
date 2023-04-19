-- Your SQL goes here
-- Appends 'payment_failed' enum variant to the list
ALTER TYPE "EventType"
ADD VALUE IF NOT EXISTS 'payment_failed';
