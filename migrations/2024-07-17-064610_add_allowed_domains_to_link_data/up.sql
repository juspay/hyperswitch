UPDATE generic_link
SET link_data = jsonb_set(link_data, '{allowed_domains}', '["*"]'::jsonb)
WHERE
    NOT link_data ? 'allowed_domains'
    AND link_type = 'payout_link';