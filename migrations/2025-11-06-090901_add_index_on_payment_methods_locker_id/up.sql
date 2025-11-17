-- Your SQL goes here
CREATE INDEX payment_method_merchant_id_customer_id_locker_id_index ON payment_methods (merchant_id, customer_id, locker_id);