-- Your SQL goes here
ALTER TYPE "IntentStatus" ADD VALUE 'partially_captured_and_capturable' after 'partially_captured';
ALTER TYPE "AttemptStatus" ADD VALUE 'partial_charged_and_chargeable' after 'partial_charged';