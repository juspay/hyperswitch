-- This file should undo anything in `up.sql`
CREATE TYPE "RoutingAlgorithmKind_new" AS ENUM ('single', 'priority', 'volume_split', 'advanced');

ALTER TABLE routing_algorithm
    ALTER COLUMN kind TYPE "RoutingAlgorithmKind_new" USING kind::text::"RoutingAlgorithmKind_new";

DROP TYPE "RoutingAlgorithmKind";

ALTER TYPE "RoutingAlgorithmKind_new" RENAME TO "RoutingAlgorithmKind";
