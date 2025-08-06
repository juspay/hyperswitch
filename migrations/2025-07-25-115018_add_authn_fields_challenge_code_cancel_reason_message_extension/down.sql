ALTER TABLE authentication
    DROP COLUMN IF EXISTS challenge_code,
    DROP COLUMN IF EXISTS challenge_cancel,
    DROP COLUMN IF EXISTS challenge_code_reason,
    DROP COLUMN IF EXISTS message_extension;