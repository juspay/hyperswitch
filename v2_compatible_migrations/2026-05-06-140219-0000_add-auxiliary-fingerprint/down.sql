-- This file should undo anything in `up.sql`
Alter table payment_methods drop column auxiliary_fingerprint_id;