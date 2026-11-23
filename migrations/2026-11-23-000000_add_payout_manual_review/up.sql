ALTER TYPE "PayoutStatus" ADD VALUE IF NOT EXISTS 'requires_review';

ALTER TYPE "EventType" ADD VALUE IF NOT EXISTS 'payout_review_required';
