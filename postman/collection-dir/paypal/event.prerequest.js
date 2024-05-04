const path = pm.request.url.toString();
const isPostRequest = pm.request.method.toString() === "POST";
const isPaymentCreation = path.match(/\/payments$/) && isPostRequest;

if (isPaymentCreation) {
  try {
    const request = JSON.parse(pm.request.body.toJSON().raw);

    // Attach routing
    const routing = { type: "single", data: "paypal" };
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
    console.error("Failed to inject routing in the request");
    console.error(error);
  }
}