ALTER TABLE payment_intent ADD COLUMN meta_data jsonb;
    
UPDATE payment_intent SET meta_data = metadata;

UPDATE payment_intent SET meta_data = jsonb_set(meta_data, '{order_details}', to_jsonb(ARRAY(select metadata -> 'order_details')), true);


