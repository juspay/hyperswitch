import { cardRequiredField, customerAcceptance } from "./Commons";
import { getCustomExchange } from "./Modifiers";

// Card details for non-3DS payment
const successfulNo3DSCardDetails = {
  card_number: "4111111111111111", // Visa test card
  card_exp_month: "10",
  card_exp_year: "2050",
  card_holder_name: "Test User",
  card_cvc: "123",
};
const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};
const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};
// Finix doesn't support 3DS, but keeping this for common pattern
const failedNo3DSCardDetails = {
  card_number: "4000000000000002", // Failed card
  card_exp_month: "01",
  card_exp_year: "2035",
  card_holder_name: "Test User",
  card_cvc: "123",
};

// auth code is dynamic hence ignoring intest cases
// Payment method data for non-3DS
// const payment_method_data_no3ds = {
//   card: {
//     last4: "1111",
//     card_type: "DEBIT",
//     card_network: "Visa",
//     card_issuer: "CONOTOXIA SP Z O.O.",
//     card_issuing_country: "POLAND",
//     card_isin: "411111",
//     card_extended_bin: null,
//     card_exp_month: "10",
//     card_exp_year: "2050",
//     card_holder_name: "Test User",
//     payment_checks: { address_verification: "POSTAL_CODE_AND_STREET_MATCH" },
//     authentication_data: null,
//     auth_code: "826685",
//   },
//   billing: null,
// };

const requiredFields = {
  payment_methods: [
    {
      payment_method: "card",
      payment_method_types: [
        {
          payment_method_type: "credit",
          card_networks: [
            {
              eligible_connectors: ["finix"],
            },
          ],
          required_fields: cardRequiredField,
        },
      ],
    },
  ],
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "on_session",
        },
      },
    },
    PaymentIntentWithShippingCost: {
      Request: {
        currency: "USD",
        amount: 11500,
        shipping_cost: 1500,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          shipping_cost: 1500,
          amount: 11500,
        },
      },
    },
    PaymentIntentOffSession: {
      Request: {
        currency: "USD",
        setup_future_usage: "off_session",
        amount: 8000,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
          amount: 8000,
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    ZeroAuthMandate: {
      Request: {
        payment_method: "card",
        customer_acceptance: customerAcceptance,
        mandate_data: singleUseMandateData.mandate_type,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          mandate_id: "mandate_id_placeholder",
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Request: {
        amount: 0,
        currency: "USD",
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
          amount: 0,
        },
      },
    },
    ZeroAuthConfirmPaymentIntent: {
      Request: {
        payment_method: "card",
        customer_acceptance: customerAcceptance,
        mandate_data: singleUseMandateData.mandate_type,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: null,
        },
        currency: "USD",
        customer_acceptance: null,
        amount: 8000,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 8000,
          amount_capturable: 8000,
        },
      },
    },
    Capture: {
      Request: {
        amount_to_capture: 8000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 8000,
          amount_received: 8000,
        },
      },
    },
    Void: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
          amount: 8000,
          amount_capturable: 0,
        },
      },
    },
    Refund: {
      Request: {
        amount: 8000,
        reason: "Customer request",
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    PartialRefund: {
      Request: {
        amount: 4000,
        reason: "Partial refund",
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
          amount: 4000,
        },
      },
    },
    SyncRefund: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "pending",
          amount: 8000,
        },
      },
    },
    IncrementalAuthorization: {
      Request: {
        amount: 10000,
        reason: "Additional services",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 10000,
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        mandate_data: singleUseMandateData.mandate_type,
        customer_acceptance: customerAcceptance,
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          mandate_id: "mandate_id_placeholder",
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        mandate_data: singleUseMandateData.mandate_type,
        customer_acceptance: customerAcceptance,
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          mandate_id: "mandate_id_placeholder",
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        mandate_data: multiUseMandateData.mandate_type,
        customer_acceptance: customerAcceptance,
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          mandate_id: "mandate_id_placeholder",
        },
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        mandate_data: multiUseMandateData.mandate_type,
        customer_acceptance: customerAcceptance,
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          mandate_id: "mandate_id_placeholder",
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: null,
        },
        mandate_data: multiUseMandateData.mandate_type,
        customer_acceptance: customerAcceptance,
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          mandate_id: "mandate_id_placeholder",
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: null,
        },
        mandate_data: multiUseMandateData.mandate_type,
        customer_acceptance: customerAcceptance,
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          mandate_id: "mandate_id_placeholder",
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: null,
        },
        currency: "USD",
        customer_acceptance: null,
        amount: 8000,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          amount: 8000,
          amount_capturable: 8000,
        },
      },
    },
    Confirm: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    "3DSAutoCapture": getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
      },
    }),
    "3DSManualCapture": getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
      },
    }),
    "3DSConfirm": getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
      },
    }),
    "3DSRetrieve": getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
      },
    }),
    "3DSNotSupportedCard": getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: failedNo3DSCardDetails,
        },
        currency: "USD",
      },
    }),
    "3DSTimeout": getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
      },
    }),
    NoThreedsConfirmRequest: {
      Request: {
        confirm: true,
        payment_type: "normal",
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
    },
    OrderDetails: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        order_details: [
          {
            product_name: "Test Product",
            quantity: 1,
            amount: 6000,
          },
        ],
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    OrderDetailsWithBilling: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: {
            address: {
              line1: "123 Main St",
              city: "San Francisco",
              state: "CA",
              zip: "94102",
              country: "US",
              first_name: "John",
              last_name: "Doe",
            },
            email: "john.doe@example.com",
          },
        },
        currency: "USD",
        order_details: [
          {
            product_name: "Test Product",
            quantity: 1,
            amount: 6000,
          },
        ],
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    OrderDetailsWithoutBilling: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        order_details: [
          {
            product_name: "Test Product",
            quantity: 1,
            amount: 6000,
          },
        ],
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    OrderDetailsMultipleItems: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        order_details: [
          {
            product_name: "Test Product 1",
            quantity: 1,
            amount: 3000,
          },
          {
            product_name: "Test Product 2",
            quantity: 2,
            amount: 1500,
          },
        ],
        authentication_type: "no_three_ds",
        request_external_three_ds_authentication: true,
        three_ds_data: {
          authentication_cryptogram: {
            cavv: {
              authentication_cryptogram: "3q2+78r+ur7erb7vyv66vv////8=",
            },
          },
          ds_trans_id: "c4e59ceb-a382-4d6a-bc87-385d591fa09d",
          version: "2.1.0",
          eci: "AUTHENTICATED",
          transaction_status: "Y",
          exemption_indicator: "low_value",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    OrderDetailsMissingProductName: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        order_details: [
          {
            quantity: 1,
            amount: 6000,
          },
        ],
      },
      Response: {
        status: 400,
        body: {
          error: {
            error_type: "invalid_request",
            message:
              "Json deserialize error: missing field `product_name` at line 1 column 1755",
            code: "IR_06",
          },
          authentication_type: "no_three_ds",
        },
      },
    },
  },
  pm_list: {
    PmListResponse: {
      PmListNull: {
        payment_methods: [],
      },
      pmListDynamicFieldWithoutBilling: requiredFields,
      pmListDynamicFieldWithBilling: requiredFields,
      pmListDynamicFieldWithNames: requiredFields,
      pmListDynamicFieldWithEmail: requiredFields,
    },
  },
  webhook: {
    TransactionIdConfig: {
      // Defines how to locate and parse the payment reference ID from connector-specific webhook payloads
      path: "_embedded.authorizations.0.id",
      // Type of payment reference ID
      type: "string",
    },
    RefundIdConfig: {
      // Finix refund (REVERSAL) webhooks carry the connector refund ID in the transfers array
      path: "_embedded.transfers.0.id",
      type: "string",
    },
  },
};
