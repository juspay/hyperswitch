-- Create function
CREATE OR REPLACE FUNCTION set_org_id_org_name()
RETURNS TRIGGER AS $$
BEGIN
    -- If 'org_id' is NULL, default it to 'id'
    IF NEW.org_id IS NULL THEN
        NEW.org_id := NEW.id;
        NEW.org_name := NEW.organization_name;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to call the function before insert
CREATE TRIGGER trigger_set_org_id_org_name
BEFORE INSERT ON organization
FOR EACH ROW
EXECUTE FUNCTION set_org_id_org_name();

-- Create function
CREATE OR REPLACE FUNCTION update_org_name()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.organization_name IS DISTINCT FROM OLD.org_name THEN
        NEW.org_name = NEW.organization_name;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to call the function before update
CREATE TRIGGER trigger_update_org_name
BEFORE UPDATE ON organization
FOR EACH ROW
EXECUTE FUNCTION update_org_name();
