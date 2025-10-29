ALTER TABLE mandate
ADD COLUMN
IF NOT EXISTS customer_user_agent_extended VARCHAR
(2048);
