// Shared test data for cypress/e2e/spec/Security/OMPScoping/, mirroring the
// Commons.js convention used by configs/Payment and configs/Payout.

// Matches Adyenplatform's card_pm "Confirm" fixture shape
// (configs/Payout/AdyenPlatform.js) - with confirm=true, auto_fulfill=false,
// Adyenplatform returns "requires_fulfillment" rather than failing outright.
// Used only to create PO_PA/PO_PB in 00-Setup.cy.js; Adyenplatform is
// hardcoded as the payout connector there regardless of CYPRESS_CONNECTOR,
// since it's the only connector in this repo's creds file with a
// payout-capable ("_payout") credentials entry but no plain payment one.
export const ompCardPayoutData = {
  payout_method_data: {
    card: {
      card_number: "4111111111111111",
      expiry_month: "03",
      expiry_year: "2030",
      card_holder_name: "John Smith",
    },
  },
  currency: "EUR",
  payout_type: "card",
};
