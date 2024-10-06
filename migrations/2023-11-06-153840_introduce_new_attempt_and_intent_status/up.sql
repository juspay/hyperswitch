-- Your SQL goes here
ALTER TYPE "IntentStatus" ADD VALUE IF NOT EXISTS 'partially_captured_and_capturable';
ALTER TYPE "AttemptStatus" ADD VALUE IF NOT EXISTS 'partial_charged_and_chargeable';