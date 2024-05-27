-- Your SQL goes here
ALTER TYPE "PayoutStatus" ADD VALUE IF NOT EXISTS 'created';
ALTER TYPE "PayoutStatus" ADD VALUE IF NOT EXISTS 'expired';
ALTER TYPE "PayoutStatus" ADD VALUE IF NOT EXISTS 'reversed';