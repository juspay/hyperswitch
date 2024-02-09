ALTER TABLE configs ALTER column config TYPE TEXT USING convert_from(config,'UTF8')::text;
