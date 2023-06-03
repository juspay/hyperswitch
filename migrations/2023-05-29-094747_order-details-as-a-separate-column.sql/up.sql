ALTER TABLE payment_intent ADD COLUMN order_details jsonb;


ALTER TABLE payment_intent ALTER COLUMN order_details type jsonb [] using array[order_details];
