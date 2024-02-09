ALTER TABLE configs ALTER COLUMN config TYPE BYTEA USING convert_to(config, 'UTF8');
