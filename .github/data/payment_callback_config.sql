INSERT INTO configs (key, config)
VALUES ('payment_callback', '{"mode":"migration"}')
ON CONFLICT (key) DO UPDATE SET config = EXCLUDED.config;
