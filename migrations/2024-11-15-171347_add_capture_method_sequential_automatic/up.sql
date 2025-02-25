DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 
        FROM pg_enum 
        WHERE enumlabel = 'sequential_automatic' 
          AND enumtypid = (SELECT oid FROM pg_type WHERE typname = 'CaptureMethod')
    ) THEN
        ALTER TYPE "CaptureMethod" ADD VALUE 'sequential_automatic' AFTER 'manual';
    END IF;
END $$;
