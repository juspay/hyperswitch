-- Your SQL goes here
ALTER TYPE "Currency" ADD VALUE IF NOT EXISTS 'RON' AFTER 'QAR';
ALTER TYPE "Currency" ADD VALUE IF NOT EXISTS 'TRY' AFTER 'TTD';
