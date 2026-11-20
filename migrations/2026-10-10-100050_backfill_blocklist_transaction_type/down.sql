-- Keep payment values intact; the preceding schema migration drops the
-- transaction_type column when the full change is rolled back.
SELECT 1;
