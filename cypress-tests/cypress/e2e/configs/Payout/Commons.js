// This file is the default. To override, add to connector.js
import { getCustomExchange } from "./Modifiers";

const card_data = {
  card_number: "4111111111111111",
  expiry_month: "3",
  expiry_year: "2030",
  card_holder_name: "John Smith",
};

const payment_card_data = {
  card_number: "4111111111111111",
  card_exp_month: "03",
  card_exp_year: "2030",
  card_holder_name: "John Doe",
};

const billing = {
  address: {
    line1: "Raadhuisplein",
    line2: "92",
    city: "Hoogeveen",
    state: "FL",
    zip: "7901 BW",
    country: "NL",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "9123456789",
    country_code: "+31",
  },
};

export const connectorDetails = {
  card_pm: {
    Create: getCustomExchange({
      Request: {
        payout_type: "card",
        payout_method_data: {
          card: card_data,
        },
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_confirmation",
          payout_type: "card",
        },
      },
    }),
    Confirm: getCustomExchange({
      Request: {
        payout_type: "card",
        payout_method_data: {
          card: card_data,
        },
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          payout_type: "card",
        },
      },
    }),
    Fulfill: getCustomExchange({
      Request: {
        payout_type: "card",
        payout_method_data: {
          card: card_data,
        },
        currency: "EUR",
      },
    }),
    SavePayoutMethod: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        card: payment_card_data,
        metadata: {
          city: "NY",
          unit: "245",
        },
      },
      Response: {
        status: 200,
      },
    }),
    Token: getCustomExchange({
      Request: {
        payout_token: "token",
        payout_type: "card",
      },
    }),
  },
  bank_transfer_pm: {
    Create: getCustomExchange({
      Request: {
        payout_type: "bank",
        priority: "regular",
        payout_method_data: {
          bank: {
            iban: "NL57INGB4654188101",
          },
        },
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_confirmation",
          payout_type: "bank",
        },
      },
    }),
    Confirm: getCustomExchange({
      Request: {
        payout_type: "bank",
        priority: "regular",
        payout_method_data: {
          bank: {
            iban: "NL57INGB4654188101",
          },
        },
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          payout_type: "bank",
        },
      },
    }),
    Fulfill: getCustomExchange({
      Request: {
        payout_type: "bank",
        priority: "regular",
        payout_method_data: {
          bank: {
            iban: "NL57INGB4654188101",
          },
        },
        billing: billing,
      },
    }),
    Token: getCustomExchange({
      Request: {
        payout_token: "token",
        payout_type: "card",
      },
    }),
    EntityTypeCompany: getCustomExchange({
      Request: {
        entity_type: "Company",
      },
    }),
    EntityTypeDefault: getCustomExchange({
      Request: {},
    }),
    EntityTypeIndividual: getCustomExchange({
      Request: {
        entity_type: "Individual",
      },
    }),
    EntityTypeInvalid: getCustomExchange({
      Request: {
        entity_type: "InvalidType",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Json deserialize error: unknown variant `InvalidType`",
            code: "IR_06",
          },
        },
      },
    }),
    EntityTypeNaturalPerson: getCustomExchange({
      Request: {
        entity_type: "NaturalPerson",
      },
    }),
    EntityTypeNonProfit: getCustomExchange({
      Request: {
        entity_type: "NonProfit",
      },
    }),
    EntityTypePersonal: getCustomExchange({
      Request: {
        entity_type: "Personal",
      },
    }),
    EntityTypePublicSector: getCustomExchange({
      Request: {
        entity_type: "PublicSector",
      },
    }),
  },
  payout_link_pm: {
    PayoutLinkBase: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "USD",
        amount: 100,
        description: "Test Payout Link",
        payout_link_config: {
          test_mode: true,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payout_method_data",
        },
      },
    }),
    PayoutLinkBankTransfer: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "EUR",
        amount: 100,
        description: "Test Payout Link Bank Transfer",
        payout_link_config: {
          test_mode: true,
          enabled_payment_methods: [
            {
              payment_method: "bank_transfer",
              payment_method_types: ["sepa_bank_transfer"],
            },
          ],
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payout_method_data",
          payout_type: "bank",
        },
      },
    }),
    PayoutLinkValidationError: getCustomExchange({
      Request: {
        payout_link: true,
        currency: "USD",
        amount: 100,
        description: "Test missing customer_id",
        customer_id: null,
        payout_link_config: {
          test_mode: true,
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_04",
            message:
              "Missing required param: customer or customer_id when payout_link is true",
          },
        },
      },
    }),
    PayoutLinkConfirmConflict: getCustomExchange({
      Request: {
        payout_link: true,
        confirm: true,
        currency: "USD",
        amount: 100,
        description: "Test confirm + payout_link conflict",
        payout_link_config: {
          test_mode: true,
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "cannot confirm a payout while creating a payout link",
            code: "IR_06",
          },
        },
      },
    }),
    PayoutLinkWithoutLink: getCustomExchange({
      Request: {
        payout_link: false,
        currency: "USD",
        amount: 100,
        description: "Test without payout link",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payout_method_data",
        },
      },
    }),
  },
  wallet_pm: {
    // Cross-connector skeleton for wallet payouts. Wallet payouts are only
    // supported by wallet-capable payout connectors, so the connector-driven
    // success keys (Create / ConfirmMerchantAuth / ConfirmClientAuth /
    // RetrieveAfterConfirm) default to the 501 not-implemented response —
    // connectors override them with their real responses. The client-auth
    // restricted-field validations are enforced by the router before the
    // payout state is loaded or mutated, so their error envelopes are
    // connector-agnostic and shared here as the default.
    Create: getCustomExchange({
      Request: {
        payout_type: "wallet",
      },
    }),
    ConfirmClientAuthRestrictedFields: getCustomExchange({
      Request: {
        amount: 10000,
        payout_type: "wallet",
      },
      Response: {
        status: 422,
        body: {
          error: {
            type: "invalid_request",
            message:
              "The following fields cannot be provided for client-authenticated payout confirmation: amount",
            code: "IR_06",
          },
        },
      },
    }),
    ConfirmClientAuthMultipleRestrictedFields: getCustomExchange({
      Request: {
        amount: 10000,
        currency: "USD",
        auto_fulfill: false,
        payout_type: "wallet",
      },
      Response: {
        status: 422,
        body: {
          error: {
            type: "invalid_request",
            message:
              "The following fields cannot be provided for client-authenticated payout confirmation: amount, auto_fulfill, currency",
            code: "IR_06",
          },
        },
      },
    }),
    ConfirmClientAuthMissingClientSecret: getCustomExchange({
      Request: {
        client_secret: null,
        amount: 10000,
        payout_type: "wallet",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Missing required param: client_secret",
            code: "IR_04",
          },
        },
      },
    }),
    ConfirmMerchantAuth: getCustomExchange({
      Request: {
        amount: 100,
        payout_type: "wallet",
      },
    }),
    ConfirmClientAuth: getCustomExchange({
      Request: {
        payout_type: "wallet",
      },
    }),
    RetrieveAfterClientAuthErrors: getCustomExchange({
      Response: {
        status: 200,
        body: {
          status: "requires_confirmation",
          payout_type: "wallet",
        },
      },
    }),
    RetrieveAfterConfirm: getCustomExchange({
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
        },
      },
    }),
  },
};
