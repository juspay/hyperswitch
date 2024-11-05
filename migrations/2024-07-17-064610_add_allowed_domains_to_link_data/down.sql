UPDATE generic_link
SET link_data = link_data - 'allowed_domains'
WHERE link_data -> 'allowed_domains' = '["*"]'::jsonb AND link_type = 'payout_link';