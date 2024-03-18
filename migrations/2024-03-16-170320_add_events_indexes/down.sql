DROP INDEX events_merchant_id_event_id_index;

DROP INDEX events_merchant_id_initial_attempt_id_index;

DROP INDEX events_merchant_id_initial_events_index;

DROP INDEX events_business_profile_id_event_id_index;

DROP INDEX events_business_profile_id_initial_attempt_id_index;

DROP INDEX events_business_profile_id_initial_events_index;

ALTER TABLE events DROP COLUMN delivery_attempt;

DROP TYPE "WebhookDeliveryAttempt";
