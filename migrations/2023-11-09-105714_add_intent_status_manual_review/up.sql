-- Your SQL goes here
ALTER TYPE "AttemptStatus" ADD VALUE IF NOT EXISTS 'pending_review';

ALTER TYPE "IntentStatus" ADD VALUE IF NOT EXISTS 'manual_review';