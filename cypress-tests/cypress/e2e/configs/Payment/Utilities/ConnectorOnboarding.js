export const connectorDetails = {
  ConnectorOnboarding: {
    // action_url calls PayPal API — returns 200 with action_url in integ/sandbox (PayPal creds configured), or 422 IR_06 in dev/test (no PayPal creds)
    ActionUrl: {
      Request: {
        connector: "paypal",
        return_url: "https://example.com/callback",
        // connector_id filled at runtime from globalState.get("paypalConnectorId")
      },
      Response: {
        // Returns 200 with action_url in integ/sandbox (PayPal creds configured), or 422 IR_06 in dev/test (no PayPal creds)
        status: 422,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_06",
          },
        },
      },
    },
    // sync returns 200 with sync status in integ/sandbox, or 400 in dev/test (no PayPal connector integration)
    Sync: {
      Request: {
        connector: "paypal",
        // connector_id and profile_id filled at runtime from globalState
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
          },
        },
      },
    },
    // reset_tracking_id returns 200 empty body — writes to configs table, no PayPal call
    ResetTrackingId: {
      Request: {
        connector: "paypal",
        // connector_id filled at runtime from globalState.get("paypalConnectorId")
      },
      Response: {
        status: 200,
      },
    },
  },
};

export default connectorDetails;
