import https from "https";

export function registerFetchPaymentIntentTask(on) {
  on("task", {
    fetchPaymentIntent: ({ authApiKey, paymentIntentId, providerBaseUrl }) => {
      return new Promise((resolve, reject) => {
        const headers = {
          Authorization: `Bearer ${authApiKey}`,
        };
        const options = {
          hostname: providerBaseUrl,
          path: `/v1/payment_intents/${paymentIntentId}`,
          method: "GET",
          headers,
        };
        const req = https.request(options, (res) => {
          let data = "";
          res.on("data", (chunk) => {
            data += chunk;
          });
          res.on("end", () => {
            try {
              resolve({ status: res.statusCode, body: JSON.parse(data) });
            } catch {
              resolve({ status: res.statusCode, body: data });
            }
          });
        });
        req.on("error", reject);
        req.end();
      });
    },
  });
}
