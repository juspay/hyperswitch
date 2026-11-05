INSERT INTO configs (key, config)
VALUES ('payment_callback', '{"mode":"enforced"}')
ON CONFLICT (key) DO UPDATE SET config = EXCLUDED.config;
