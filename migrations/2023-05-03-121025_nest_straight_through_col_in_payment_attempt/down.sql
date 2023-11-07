-- This file should undo anything in `up.sql`
UPDATE payment_attempt
SET straight_through_algorithm = CASE WHEN straight_through_algorithm->>'algorithm' IS NULL THEN
    NULL
ELSE
    straight_through_algorithm->'algorithm'
END;
