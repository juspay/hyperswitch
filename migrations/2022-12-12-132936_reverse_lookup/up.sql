CREATE TABLE reverse_lookup (
    lookup_id VARCHAR(255) NOT NULL PRIMARY KEY,
    sk_id VARCHAR(50) NOT NULL,
    pk_id VARCHAR(255) NOT NULL,
    source VARCHAR(30) NOT NULL
);

CREATE INDEX lookup_id_index ON reverse_lookup (lookup_id);
