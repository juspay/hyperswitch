-- This file should undo anything in `up.sql`
ALTER TABLE payment_methods DROP CONSTRAINT payment_methods_pkey;

ALTER TABLE payment_methods
ADD PRIMARY KEY (id);
