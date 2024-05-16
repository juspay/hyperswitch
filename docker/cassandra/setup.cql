CREATE KEYSPACE IF NOT EXISTS sessionizer WITH REPLICATION =
{ 'class' : 'SimpleStrategy',
'replication_factor' : '1'
};

CREATE TABLE IF NOT EXISTS sessionizer.payments (
    id TEXT,
    state text,
    version varint,
    PRIMARY KEY((id))
);

INSERT INTO sessionizer.payments
    (id, state, version)
    VALUES ('test_p1', '{"payment_id": "p1", "xyz": "test", "merchant_id": "test", "attempts" : {"a1" : {}}}', 1);