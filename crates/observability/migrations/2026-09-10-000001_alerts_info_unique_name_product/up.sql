-- A definition is identified by (name, product).
--
-- Definitions are addressed by UUID over the API, but nothing else knows those ids: the alert
-- manager reads configuration for a detector by name, and merchants_alert_external_config
-- references the pair. Without this constraint a name can resolve to two rows and both of those
-- lookups become "pick one".
--
-- It also protects the reserved `all` definition, which holds suppression that applies to every
-- detector rather than to one: there can only ever be a single row holding it.
--
-- This migration fails on a database that already holds two definitions with one name, and that is
-- the intended behaviour. Deduplicating automatically would delete configuration somebody wrote —
-- suppression rules and thresholds among it — with no way to tell which of the two rows the alert
-- manager had been reading. An operator resolves it before the constraint goes on.

CREATE UNIQUE INDEX IF NOT EXISTS alerts_info_name_product_idx ON alerts_info (name, product);
