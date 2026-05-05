// Shared test data for Misc specs
// Pattern follows: ../../../fixtures/imports.js

export const connectorDetails = {
  pmCollectLinkCreate: {
    Request: {
      return_url: "https://example.com/return",
    },
    Response: {
      status: 200,
    },
  },

  pmCollectLinkRender: {
    Response: {
      status: 200,
    },
  },

  pmCollectLinkRenderNotFound: {
    Response: {
      status: 404,
      body: {
        error: {
          type: "invalid_request",
          code: "IR_37",
          message: "payment method collect link not found",
        },
      },
    },
  },
};
