CREATE TABLE reverse_lookup (
    sk_id SERIAL PRIMARY KEY,
    pk_id VARCHAR(255) NOT NULL,
    lookup_id VARCHAR(255) NOT NULL,
    result_id VARCHAR(255) NOT NULL,
    source VARCHAR(30) NOT NULL
)
