ALTER TABLE payment_intent ADD COLUMN attempt_count SMALLINT NOT NULL DEFAULT 1;

UPDATE payment_intent
SET attempt_count = payment_id_count.count
FROM (SELECT payment_id, count(payment_id) FROM payment_attempt GROUP BY payment_id) as payment_id_count
WHERE payment_intent.payment_id = payment_id_count.payment_id;
