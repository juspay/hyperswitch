-- Your SQL goes here
ALTER TABLE merchant_account DROP COLUMN routing_algorithm;
ALTER TABLE merchant_account DROP COLUMN custom_routing_rules;
ALTER TABLE merchant_account ADD COLUMN routing_algorithm JSON;
DROP TYPE "RoutingAlgorithm";
