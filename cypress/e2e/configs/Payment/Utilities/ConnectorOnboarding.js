/**
 * Connector Onboarding Config
 * Configuration for PayPal-specific connector onboarding endpoints
 */

export const connectorDetails = {
  ConnectorOnboarding: {
    ActionUrl: {
      Request: {
        connector: "paypal",
        return_url: "http://localhost:8080/onboarding/callback",
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
        connector: "paypal",
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
        connector: "paypal",
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
