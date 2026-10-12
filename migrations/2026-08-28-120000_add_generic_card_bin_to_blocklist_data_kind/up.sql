-- Add the `generic_card_bin` blocklist kind, a card number prefix of 6 to 10 digits.
ALTER TYPE "BlocklistDataKind" ADD VALUE IF NOT EXISTS 'generic_card_bin';
