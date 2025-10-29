-- Your SQL goes here
CREATE TYPE "RoutingApproach" AS ENUM (
  'success_rate_exploitation',
  'success_rate_exploration',
  'contract_based_routing',
  'debit_routing',
  'rule_based_routing',
  'volume_based_routing',
  'default_fallback'
);


ALTER TABLE payment_attempt
ADD COLUMN IF NOT EXISTS routing_approach "RoutingApproach";

