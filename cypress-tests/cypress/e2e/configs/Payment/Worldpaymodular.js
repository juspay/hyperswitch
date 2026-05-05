// WorldpayModular connector configuration
// NOTE: This connector supports wallets only (Apple Pay, Google Pay, Mandates)
// Card payments are NOT supported - API returns IR_19 "card is not supported by worldpaymodular"

import { getCustomExchange } from "./Modifiers";

export const connectorDetails = {
  // Wallet payment methods are supported - placeholder for future wallet test implementation
  wallet_pm: {
    PaymentIntent: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON:
          "Wallet payment test implementation pending. Connector supports Apple Pay, Google Pay, and Mandates but automated test configs need to be developed.",
      },
      Request: {
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
  },
};
