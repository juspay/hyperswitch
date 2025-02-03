-- Create function
CREATE OR REPLACE FUNCTION set_id_organization_name()
RETURNS TRIGGER AS $$
BEGIN
    -- If 'id' is NULL, default it to 'org_id'
    IF NEW.id IS NULL THEN
        NEW.id := NEW.org_id;
        NEW.organization_name := NEW.org_name;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to call the function before insert
CREATE TRIGGER trigger_set_id_organization_name
BEFORE INSERT ON organization
FOR EACH ROW
EXECUTE FUNCTION set_id_organization_name();

-- Create function
CREATE OR REPLACE FUNCTION update_organization_name()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.org_name IS DISTINCT FROM OLD.organization_name THEN
        NEW.organization_name = NEW.org_name;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to call the function before update
CREATE TRIGGER trigger_update_organization_name
BEFORE UPDATE ON organization
FOR EACH ROW
EXECUTE FUNCTION update_organization_name();