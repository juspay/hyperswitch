DELETE FROM pg_enum
WHERE
    enumlabel = 'requires_customer_action'
    AND enumtypid = (
        SELECT oid
        FROM pg_type
        WHERE
            typname = 'PayoutStatus'
    );

DELETE FROM pg_enum
WHERE
    enumlabel = 'outgoing_payment_sent'
    AND enumtypid = (
        SELECT oid
        FROM pg_type
        WHERE
            typname = 'PayoutStatus'
    );

DELETE FROM pg_enum
WHERE
    enumlabel = 'funds_refunded'
    AND enumtypid = (
        SELECT oid
        FROM pg_type
        WHERE
            typname = 'PayoutStatus'
    );

DELETE FROM pg_enum
WHERE
    enumlabel = 'expired'
    AND enumtypid = (
        SELECT oid
        FROM pg_type
        WHERE
            typname = 'PayoutStatus'
    );

ALTER TYPE "PayoutStatus" RENAME VALUE 'processing' TO 'pending';