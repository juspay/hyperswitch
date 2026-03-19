-- Your SQL goes here
UPDATE dynamic_routing_stats
SET payment_method_type = 'pix_qr'
WHERE payment_method_type = 'pix';

UPDATE payment_methods
SET payment_method_type = 'pix_qr'
WHERE payment_method_type = 'pix';

UPDATE payment_attempt
SET payment_method_type = 'pix_qr'
WHERE payment_method_type = 'pix';

UPDATE payment_attempt
SET payment_method_data = jsonb_set(
    payment_method_data #- '{bank_transfer,pix}', 
    '{bank_transfer,pix_qr}', 
    payment_method_data->'bank_transfer'->'pix',
    true
)
WHERE payment_method_type = 'pix' 
  AND payment_method_data->'bank_transfer' ? 'pix';


UPDATE payment_attempt
SET straight_through_algorithm = jsonb_set(
    straight_through_algorithm #- '{pre_routing_results,pix}', 
    '{pre_routing_results,pix_qr}',                             
    straight_through_algorithm->'pre_routing_results'->'pix',   
    true                                                       
)
WHERE payment_method_type = 'pix' 
  AND straight_through_algorithm->'pre_routing_results' ? 'pix';