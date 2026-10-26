import { getCustomExchange } from "./Modifiers";
import { connectorDetails as commonDetails } from "./Commons";

const billingAddressGB = {
  address: {
    line1: "1 Principal Place",
    city: "London",
    zip: "EC2A 2FA",
    country: "GB",
    first_name: "John",
    last_name: "Doe",
  },
  email: "test@example.com",
  phone: {
    number: "7123456789",
    country_code: "+44",
  },
};

// Truelayer is served exclusively by the Unified Connector Service. For
// unsupported payment methods the UCS server returns its own message
// ("This feature is not implemented: ..." with the title-cased connector
// name) instead of the router-generated message used by Modifiers' default
// exchange. The commons defaults are reused for the unsupported methods, but
// with the not-implemented (IR_00) error message rewritten to the UCS
// variant - the connector-specific handling stays in this file rather than
// being hardcoded in the shared Modifiers.
const UCS_NOT_IMPLEMENTED_MESSAGE =
  "This feature is not implemented: Selected payment method through Truelayer is not implemented";

const withUcsNotImplementedMessage = (node) => {
  if (!node || typeof node !== "object" || Array.isArray(node)) {
    return node;
  }

  // Exchange object: only the Response may need the message rewrite; Request,
  // Configs and any nested exchanges are left untouched.
  if ("Response" in node) {
    const error = node.Response?.body?.error;

    return error?.code === "IR_00"
      ? {
          ...node,
          Response: {
            ...node.Response,
            body: {
              error: { ...error, message: UCS_NOT_IMPLEMENTED_MESSAGE },
            },
          },
        }
      : node;
  }

  // Payment-method category map: recurse into each entry.
  return Object.fromEntries(
    Object.entries(node).map(([key, value]) => [
      key,
      withUcsNotImplementedMessage(value),
    ])
  );
};

const ucsCommonDetails = withUcsNotImplementedMessage(commonDetails);

export const connectorDetails = {
  ...ucsCommonDetails,
  bank_redirect_pm: {
    ...ucsCommonDetails.bank_redirect_pm,
    // Required supported-method config for the Truelayer confirm flow. The CI
    // selection runs only this bank redirect flow for Truelayer.
    Truelayer: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "open_banking",
        payment_method_data: {
          bank_redirect: {
            open_banking: {},
          },
        },
        currency: "GBP", // Truelayer requires GBP for UK open banking payments.
        billing: billingAddressGB,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "open_banking",
          connector: "truelayer",
        },
      },
      Configs: {
        skipPaymentMethodStatusAssertion: true,
      },
    }),
    TruelayerNoBilling: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "open_banking",
        payment_method_data: {
          bank_redirect: {
            open_banking: {},
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            // UCS maps missing top-level billing to the shared IR_04 error.
            message: "Missing required param: billing",
            code: "IR_04",
          },
        },
      },
    }),
  },
};
