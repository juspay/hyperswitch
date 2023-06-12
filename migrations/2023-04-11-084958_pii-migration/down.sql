-- This file should undo anything in `up.sql`

ALTER TABLE merchant_connector_account
    ALTER COLUMN connector_account_details TYPE JSON
    USING convert_from(connector_account_details, 'UTF8')::json;

ALTER TABLE merchant_account
    ALTER COLUMN merchant_name TYPE VARCHAR(128) USING convert_from(merchant_name, 'UTF8')::text,
    ALTER merchant_details TYPE JSON USING convert_from(merchant_details, 'UTF8')::json;

ALTER TABLE address
    ALTER COLUMN line1 TYPE VARCHAR(255) USING convert_from(line1, 'UTF8')::text,
    ALTER COLUMN line2 TYPE VARCHAR(255) USING convert_from(line2, 'UTF8')::text,
    ALTER COLUMN line3 TYPE VARCHAR(255) USING convert_from(line3, 'UTF8')::text,
    ALTER COLUMN state TYPE VARCHAR(128) USING convert_from(state, 'UTF8')::text,
    ALTER COLUMN zip TYPE VARCHAR(16) USING convert_from(zip, 'UTF8')::text,
    ALTER COLUMN first_name TYPE VARCHAR(255) USING convert_from(first_name, 'UTF8')::text,
    ALTER COLUMN last_name TYPE VARCHAR(255) USING convert_from(last_name, 'UTF8')::text,
    ALTER COLUMN phone_number TYPE VARCHAR(32) USING convert_from(phone_number, 'UTF8')::text;

ALTER TABLE customers
    ALTER COLUMN name TYPE VARCHAR(255) USING convert_from(name, 'UTF8')::text,
    ALTER COLUMN email TYPE VARCHAR(255) USING convert_from(email, 'UTF8')::text,
    ALTER COLUMN phone TYPE VARCHAR(32) USING convert_from(phone, 'UTF8')::text;
