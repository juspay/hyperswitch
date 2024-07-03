-- The following queries must be run after the newer version of the application is deployed.
-- Running these queries can even be deferred for some time (a couple of weeks or even a month) until the
-- new version being deployed is considered stable
ALTER TABLE payment_intent DROP COLUMN id;

ALTER TABLE payment_attempt DROP COLUMN id;
