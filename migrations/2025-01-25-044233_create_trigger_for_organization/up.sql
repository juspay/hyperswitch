-- Your SQL goes here
CREATE OR REPLACE FUNCTION set_not_null_field()
RETURNS TRIGGER AS $$
BEGIN
    -- If 'org_id' is NULL, default it to 'id'
    IF NEW.org_id IS NULL THEN
        NEW.org_id := NEW.id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER before_insert_trigger
BEFORE INSERT ON organization
FOR EACH ROW
EXECUTE FUNCTION set_not_null_field();