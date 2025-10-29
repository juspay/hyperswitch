
ALTER TABLE business_profile ADD COLUMN IF NOT EXISTS dispute_polling_interval INTEGER;
ALTER TYPE "DisputeStage" ADD VALUE 'arbitration';
AlTER TYPE "DisputeStage" ADD VALUE 'dispute_reversal';
