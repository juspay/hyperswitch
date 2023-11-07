-- Your SQL goes here

CREATE TYPE "RoutingAlgorithmKind" AS ENUM ('single', 'priority', 'volume_split', 'advanced');

CREATE TABLE routing_algorithm (
    algorithm_id VARCHAR(64) PRIMARY KEY,
    profile_id VARCHAR(64) NOT NULL,
    merchant_id VARCHAR(64) NOT NULL,
    name VARCHAR(64) NOT NULL,
    description VARCHAR(256),
    kind "RoutingAlgorithmKind" NOT NULL,
    algorithm_data JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL,
    modified_at TIMESTAMP NOT NULL
);

CREATE INDEX routing_algorithm_profile_id_modified_at ON routing_algorithm (profile_id, modified_at DESC);

CREATE INDEX routing_algorithm_merchant_id_modified_at ON routing_algorithm (merchant_id, modified_at DESC);
