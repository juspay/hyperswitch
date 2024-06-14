-- Your SQL goes here
ALTER TYPE "PayoutStatus" ADD VALUE IF NOT EXISTS 'initiated';
ALTER TYPE "PayoutStatus" ADD VALUE IF NOT EXISTS 'expired';
ALTER TYPE "PayoutStatus" ADD VALUE IF NOT EXISTS 'reversed';