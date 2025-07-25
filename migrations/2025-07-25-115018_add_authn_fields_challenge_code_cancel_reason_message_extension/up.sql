ALTER TABLE authentication
    ADD COLUMN challenge_code VARCHAR NULL,
    ADD COLUMN challenge_cancel VARCHAR NULL,
    ADD COLUMN challenge_code_reason VARCHAR NULL,
    ADD COLUMN message_extension JSONB NULL;
