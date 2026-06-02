// Connector Onboarding Configuration
// Utility config for connector onboarding endpoints
// Endpoints: POST /connector_onboarding/{action_url|sync|reset_tracking_id}

import { getCustomExchange } from "../Modifiers.js";

export const connectorDetails = {
  ConnectorOnboarding: {
    // Get Action URL for connector onboarding
    ActionUrl: getCustomExchange({
      Request: {
        connector_id: "paypal",
      },
      Response: {
        status: 200,
        body: {
          status: "success",
          action_url: "https://www.paypal.com/us/merchantsignup/partner/onboardingentry",
          tracking_id: "TRACK_ID_FROM_RESPONSE",
        },
      },
    }),

    // Sync connector onboarding status
    Sync: getCustomExchange({
      Request: {
        connector_id: "paypal",
      },
      Response: {
        status: 200,
        body: {
          status: "syncing",
          connector_id: "paypal",
          merchant_id: "MERCHANT_ID_FROM_GLOBAL_STATE",
        },
      },
    }),

    // Reset tracking ID for onboarding session
    ResetTrackingId: getCustomExchange({
      Request: {
        connector_id: "paypal",
      },
      Response: {
        status: 200,
        body: {
          status: "success",
          tracking_id: "NEW_TRACKING_ID_FROM_RESPONSE",
          connector_id: "paypal",
          merchant_id: "MERCHANT_ID_FROM_GLOBAL_STATE",
        },
      },
    }),
  },
};

export default connectorDetails;
