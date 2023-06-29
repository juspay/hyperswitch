-- Your SQL goes here

CREATE TYPE "CardNetwork" AS ENUM (
    'EUROSHELLFUELCARD',
    'REDFUELCARD',
    'SODEXO',
    'DINERS',
    'PHHFUELCARD',
    'LUKOILFUELCARD',
    'REDLIQUIDFUELCARD',
    'GECAPITAL',
    'BANKCARD(INACTIVE)',
    'AIRPLUS',
    'BPFUELCARD',
    'DISCOVER',
    'BAJAJ',
    'RBSGIFTCARD',
    'PRIVATELABELCARD',
    'ELO',
    'LOYALTYCARD',
    'AMEX',
    'STARREWARDS',
    'RUPAY',
    'CHINAUNIONPAY',
    'MAESTRO',
    'CHJONESFUELCARD',
    'VISA',
    'MASTERCARD',
    'JCB',
    'UNIONPAY',
    'UKFUELCARD',
    'PRIVATE'
);

ALTER TABLE cards_info
ALTER COLUMN card_network TYPE "CardNetwork"
USING card_network::"CardNetwork";