-- Your SQL goes here
ALTER TABLE merchant_connector_account
    ALTER COLUMN connector_account_details TYPE bytea
    USING convert_to(connector_account_details::text, 'UTF8');

ALTER TABLE merchant_account
    ALTER COLUMN merchant_name TYPE bytea USING convert_to(merchant_name, 'UTF8'),
    ALTER merchant_details TYPE bytea USING convert_to(merchant_details::text, 'UTF8');

ALTER TABLE address
    ALTER COLUMN line1 TYPE bytea USING convert_to(line1, 'UTF8'),
    ALTER COLUMN line2 TYPE bytea USING convert_to(line2, 'UTF8'),
    ALTER COLUMN line3 TYPE bytea USING convert_to(line3, 'UTF8'),
    ALTER COLUMN state TYPE bytea USING convert_to(state, 'UTF8'),
    ALTER COLUMN zip TYPE bytea USING convert_to(zip, 'UTF8'),
    ALTER COLUMN first_name TYPE bytea USING convert_to(first_name, 'UTF8'),
    ALTER COLUMN last_name TYPE bytea USING convert_to(last_name, 'UTF8'),
    ALTER COLUMN phone_number TYPE bytea USING convert_to(phone_number, 'UTF8');

ALTER TABLE customers
    ALTER COLUMN name TYPE bytea USING convert_to(name, 'UTF8'),
    ALTER COLUMN email TYPE bytea USING convert_to(email, 'UTF8'),
    ALTER COLUMN phone TYPE bytea USING convert_to(phone, 'UTF8');
