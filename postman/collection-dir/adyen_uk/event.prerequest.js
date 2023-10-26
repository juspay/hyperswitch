// Add appropriate profile_id for relevant requests
const path = pm.request.url.toString();
const isPostRequest = pm.request.method.toString() === "POST";
const isPaymentCreation = path.match(/\/payments$/) && isPostRequest;
const isPayoutCreation = path.match(/\/payouts\/create$/) && isPostRequest;

if (isPaymentCreation || isPayoutCreation) {
  try {
    const request = JSON.parse(pm.request.body.toJSON().raw);

    // Attach profile_id
    const profile_id = isPaymentCreation
      ? pm.collectionVariables.get("payment_profile_id")
      : pm.collectionVariables.get("payout_profile_id");
    request["profile_id"] = profile_id;

    // Attach routing
    const routing = { type: "single", data: "adyen" };
    request["routing"] = routing;

    let updatedRequest = {
      mode: "raw",
      raw: JSON.stringify(request),
      options: {
        raw: {
          language: "json",
        },
      },
    };
    pm.request.body.update(updatedRequest);
  } catch (error) {
    console.error("Failed to inject profile_id in the request");
    console.error(error);
  }
}
