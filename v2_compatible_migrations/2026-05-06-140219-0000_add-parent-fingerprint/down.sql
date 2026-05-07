-- This file should undo anything in `up.sql`
Alter table payment_methods drop column parent_fingerprint_id;