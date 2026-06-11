export const connectorDetails = {
  ConnectorOnboarding: {
    // action_url calls PayPal API — returns IR_06 in test env (no PayPal credentials configured)
    ActionUrl: {
      Request: {
        connector: "paypal",
        return_url: "https://example.com/callback",
        // connector_id filled at runtime from globalState.get("paypalConnectorId")
      },
      Response: {
        // In test env: 422 IR_06 (Client Authentication failed — no PayPal creds in config)
        status: 422,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_06",
          },
        },
      },
    },
    // sync always returns 400 in test env (no real PayPal connector integration)
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
