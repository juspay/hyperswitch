-- Your SQL goes here
ALTER TYPE "DashboardMetadata"
ADD VALUE IF NOT EXISTS 'recon_status';
