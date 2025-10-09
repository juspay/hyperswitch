import { isoTimeTomorrow } from "../../../utils/RequestBodyUtils";
import { getCustomExchange } from "./Modifiers";

const billingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "California",
    zip: "94122",
    country: "US",
    first_name: "joseph",
    last_name: "Doe",
  },
  phone: {
    number: "9123456789",
    country_code: "+91",
  },
};

export const connectorDetails = {
  card_pm: {
    ZeroAuthPaymentIntent: {
      Request: {
        currency: "BRL",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
  },
  bank_transfer_pm: {
    Pix: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix",
        payment_method_data: {
          bank_transfer: {
            pix: {
              // since we pass the same cpf number, the connector customer id will be updated instead of new ones being created
              cpf: "86665623580",
              source_bank_account_id: "739d6b0a-e92a-40fd-9f58-6d4cdeb699bb",
              pix_qr_expiry: isoTimeTomorrow(), // 1 day expiration
            },
          },
        },
        billing: {
          ...billingAddress,
          address: {
            ...billingAddress.address,
            country: "BR",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
      ResponseCustom: {
        status: 200,
        body: {
          error_code: "Cancelled",
          error_reason: "Unable to generate Pix QRCode",
        },
      },
    }),
  },
};
