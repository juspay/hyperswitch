ALTER table payment_link RENAME column link_open to link_to_pay;

ALTER table payment_link DROP COLUMN link_secure;