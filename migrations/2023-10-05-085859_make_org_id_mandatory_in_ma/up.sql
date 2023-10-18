-- Your SQL goes here
UPDATE merchant_account
SET organization_id = 'org_abcdefghijklmn'
WHERE organization_id IS NULL;

ALTER TABLE merchant_account
ALTER COLUMN organization_id
SET NOT NULL;
