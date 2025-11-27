-- This file should undo anything in `up.sql`
ALTER TABLE authentication
DROP COLUMN IF EXISTS earliest_supported_version,
DROP COLUMN IF EXISTS latest_supported_version,
DROP COLUMN IF EXISTS mcc,
DROP COLUMN IF EXISTS platform,
DROP COLUMN IF EXISTS device_type,
DROP COLUMN IF EXISTS device_brand,
DROP COLUMN IF EXISTS device_os,
DROP COLUMN IF EXISTS device_display,
DROP COLUMN IF EXISTS browser_name,
DROP COLUMN IF EXISTS browser_version,
DROP COLUMN IF EXISTS scheme_name,
DROP COLUMN IF EXISTS exemption_requested,
DROP COLUMN IF EXISTS exemption_accepted,
DROP COLUMN IF EXISTS issuer_id,
DROP COLUMN IF EXISTS issuer_country,
DROP COLUMN IF EXISTS merchant_country_code;
