/**
 * Connector Onboarding Config
 * Configuration for PayPal-specific connector onboarding endpoints
 */

export const connectorDetails = {
  ConnectorOnboarding: {
    ActionUrl: {
      Request: {
        connector_id: "paypal",
        merchant_id: null, // Will be filled from globalState
        onboarding_type: "oauth",
      },
      Response: {
        status: 200,
        body: {
          action_url: null, // Will be validated as non-null
          tracking_id: null, // Will be validated as non-null
        },
      },
    },
    Sync: {
      Request: {
        connector_id: "paypal",
        merchant_id: null,
      },
      Response: {
        status: 200,
        body: {
          status: null, // syncing, completed, or failed
          connector_id: "paypal",
        },
      },
    },
    ResetTrackingId: {
      Request: {
        connector_id: "paypal",
        merchant_id: null,
      },
      Response: {
        status: 200,
        body: {
          tracking_id: null, // Will be validated as new non-null value
          connector_id: "paypal",
        },
      },
    },
  },
};
