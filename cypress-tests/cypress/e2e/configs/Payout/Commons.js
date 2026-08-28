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
};
