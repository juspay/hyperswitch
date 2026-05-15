export const connectorDetails = {
  wallet_pm: {
    Capture: {
      Request: {
        amount_to_capture: 6000,
      },
    },
    Void: {
      Request: {},
      Configs: {
        TRIGGER_SKIP: true,
      },
    },
    Refund: {
      Request: {
        amount: 6000,
      },
    },
  },
};
