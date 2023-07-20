DROP TABLE PAYOUT_ATTEMPT;

DROP TABLE PAYOUTS;

DROP TYPE "PayoutStatus";

DROP TYPE "PayoutType";

-- Alterations

ALTER TABLE
    merchant_account DROP COLUMN payout_routing_algorithm;

ALTER TABLE locker_mock_up DROP COLUMN enc_card_data;

DELETE FROM pg_enum
WHERE
    enumlabel = 'payout_processor'
    AND enumtypid = (
        SELECT oid
        FROM pg_type
        WHERE typname = 'ConnectorType'
    )