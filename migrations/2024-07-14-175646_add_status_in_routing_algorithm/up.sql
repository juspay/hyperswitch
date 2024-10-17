-- Your SQL goes here
CREATE TYPE "RoutingAlgorithmStatus" AS ENUM (
    'enabled',
    'disabled'
);

ALTER TABLE routing_algorithm 
ADD COLUMN IF NOT EXISTS 
status 
"RoutingAlgorithmStatus"
DEFAULT "RoutingAlgorithmStatus"('enabled');
