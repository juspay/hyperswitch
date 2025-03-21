-- Your SQL goes here
CREATE TYPE "AlgorithmType" AS ENUM ('routing', 'surcharge', '3ds');

ALTER TABLE routing_algorithm ADD COLUMN IF NOT EXISTS algorithm_type "AlgorithmType" NOT NULL;
