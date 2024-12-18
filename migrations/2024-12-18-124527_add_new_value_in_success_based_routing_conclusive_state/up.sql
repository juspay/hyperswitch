-- Your SQL goes here
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_enum 
        WHERE enumlabel = 'non_deterministic'
          AND enumtypid = (SELECT oid FROM pg_type WHERE typname = 'SuccessBasedRoutingConclusiveState')
    ) THEN
        ALTER TYPE "SuccessBasedRoutingConclusiveState" ADD VALUE 'non_deterministic';
    END IF;
END $$;
