ALTER TABLE payment_intent
ALTER COLUMN business_country DROP DEFAULT,
    ALTER COLUMN business_country TYPE "CountryCode" USING business_country::"CountryCode";
