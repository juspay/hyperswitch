CREATE KEYSPACE IF NOT EXISTS sessionizer WITH REPLICATION =
{ 'class' : 'SimpleStrategy',
'replication_factor' : '1'
};

CREATE TABLE IF NOT EXISTS sessionizer.payments (
    id TEXT,
    id_type TEXT,
    state text,
    version varint,
    PRIMARY KEY((id, id_type))
);

INSERT INTO sessionizer.payments
    (id, id_type, state, version)
    VALUES ('test_p1', 'merchant_payment', '{"payment_id": "p1", "xyz": "test", "merchant_id": "test", "attempts" : {"a1" : {}}}', 1);