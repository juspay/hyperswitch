-- Add CaptureReview variant to AttemptStatus enum
-- Represents the attempt-level status when capture fails after initial success
ALTER TYPE "AttemptStatus" ADD VALUE IF NOT EXISTS 'capture_review';
