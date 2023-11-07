-- Your SQL goes here
CREATE TYPE "ReconStatus" AS ENUM ('requested','active', 'disabled','not_requested');
ALTER TABLE merchant_account ADD recon_status "ReconStatus" NOT NULL DEFAULT "ReconStatus"('not_requested');