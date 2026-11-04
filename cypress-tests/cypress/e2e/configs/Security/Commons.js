// Shared test data for cypress/e2e/spec/Security/OMPScoping/, following the
// same getCustomExchange(Request/Response) convention as
// configs/Payment/Commons.js and configs/Payout/Commons.js - each uses its
// own domain's getCustomExchange (Payment vs Payout) since each merges
// overrides onto that domain's own default exchange (e.g. Payment's
// defaults to a 501 not-implemented Response; see Modifiers.js in each
// folder), not a generic shared one.
import { getCustomExchange as getPaymentCustomExchange } from "../Payment/Modifiers";
import { getCustomExchange as getPayoutCustomExchange } from "../Payout/Modifiers";

// PI_PA / PI_PB creation in 00-Setup.cy.js. Matches the shape
// configs/Payment/Commons.js's own PaymentIntent getter uses.
export const PaymentIntent = () =>
  getPaymentCustomExchange({
    Request: { currency: "USD" },
    Response: {
      status: 200,
      body: { status: "requires_payment_method" },
    },
  });

// PO_PA / PO_PB creation in 00-Setup.cy.js. Matches Adyenplatform's card_pm
// "Confirm" fixture shape (configs/Payout/AdyenPlatform.js) - with
// confirm=true, auto_fulfill=false, Adyenplatform returns
// "requires_fulfillment" rather than failing outright. Adyenplatform is
// hardcoded as the payout connector in 00-Setup.cy.js regardless of
// CYPRESS_CONNECTOR, since it's the only connector in this repo's creds
// file with a payout-capable ("_payout") credentials entry but no plain
// payment one - so this Request intentionally doesn't vary by connector.
export const CardPayout = () =>
  getPayoutCustomExchange({
    Request: {
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
    },
    Response: { status: 200, body: {} },
  });
