export const connectorDetails = {
  ConnectorOnboarding: {
    ActionUrl: {
      Request: {
        connector: "paypal",
        connector_id: "paypal",
        return_url: "https://example.com/callback",
      },
      Response: {
        status: 200,
        body: {
          paypal: {
            action_url: null,
          },
        },
      },
    },
    Sync: {
      Request: {
        connector: "paypal",
        connector_id: "paypal",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "Connector integration not found for merchant account and connector type",
          },
        },
      },
    },
    ResetTrackingId: {
      Request: {
        connector: "paypal",
        connector_id: "paypal",
      },
      Response: {
        status: 200,
        body: {
          message: "tracking_id updated successfully",
        },
      },
    },
  },
};

export default connectorDetails;
