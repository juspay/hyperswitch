-- Your SQL goes here
ALTER TABLE mandate
ADD IF NOT EXISTS single_use_amount INTEGER DEFAULT NULL,
ADD IF NOT EXISTS single_use_currency "Currency" DEFAULT NULL;
