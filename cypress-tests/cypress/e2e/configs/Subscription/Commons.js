// Subscription Commons Configuration
// Default configuration for subscription tests
import { getCustomExchange } from "../Payment/Modifiers";

export const connectorDetails = {
  subscription_pm: {
    Create: getCustomExchange({
      Request: {
        customer_id: "",
        billing_processor_id: "",
        currency: "USD",
        amount: 1000,
        interval: "month",
        interval_count: 1,
        description: "Test subscription",
      },
      Response: {
        status: 200,
        body: {
          status: "active",
        },
      },
    }),
    CreateInvalidCustomer: getCustomExchange({
      Request: {
        customer_id: "cust_invalid_nonexistent",
        billing_processor_id: "",
        currency: "USD",
        amount: 1000,
        interval: "month",
        interval_count: 1,
        description: "Test subscription with invalid customer",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_00",
            message: "Customer does not exist",
          },
        },
      },
    }),
    CreateMissingFields: getCustomExchange({
      Request: {
        description: "Test subscription missing required fields",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_00",
            message: "Missing required field: customer_id",
          },
        },
      },
    }),
    Retrieve: getCustomExchange({
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "active",
        },
      },
    }),
    RetrieveCancelled: getCustomExchange({
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    }),
    Update: getCustomExchange({
      Request: {
        description: "Updated test subscription",
      },
      Response: {
        status: 200,
        body: {
          status: "active",
        },
      },
    }),
    Cancel: getCustomExchange({
      Request: {
        cancellation_reason: "requested_by_customer",
      },
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    }),
    Reactivate: getCustomExchange({
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "active",
        },
      },
    }),
  },
};
