import { getCustomExchange } from "./Modifiers";

export const connectorDetails = {
  pay_later_pm: {
    Refund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    }),
  },
};
