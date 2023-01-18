-- This file should undo anything in `up.sql`
CREATE TYPE "RoutingAlgorithm" AS ENUM (
    'round_robin',
    'max_conversion',
    'min_cost',
    'custom'
);

ALTER TABLE merchant_account DROP COLUMN routing_algorithm;
ALTER TABLE merchant_account ADD COLUMN custom_routing_rules JSON;
ALTER TABLE merchant_account ADD COLUMN routing_algorithm "RoutingAlgorithm";
