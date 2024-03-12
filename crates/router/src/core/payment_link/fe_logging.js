function parseRoute(url) {
  const route = new URL(url).pathname;
  const regex =
    /^\/(?:redirect\/)?([^/]+)\/([^/]+)(?:\/redirect\/(?:response|complete)\/([^/]+))?(?:\/([^/]+))?$/;
  const matches = route.match(regex);
  const attemptIdExists = !(
    route.includes("response") || route.includes("complete")
  );
  console.log(matches);
  if (matches) {
    const [, paymentId, merchantId, connector, attemptId, credsIdentifier] =
      matches;
    return {
      paymentId,
      merchantId,
      attemptId: attemptIdExists ? attemptId : "",
      connector: connector || "",
    };
  } else {
    return {
      paymentId: "",
      merchantId: "",
      attemptId: "",
      connector: "",
    };
  }
}
  
async function postLog(log) {
  const url = "https://sandbox.hyperswitch.io/logs/redirection";
  console.log("sahkal postlog");
  try {
    const response = await fetch(url, {
      method: "POST",
      mode: "no-cors",
      body: JSON.stringify(log),
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json",
      },
    });
  } catch (err) {
    console.error(`Error in logging: ${err}`);
  }
}


window.addEventListener("error", (event) => {
  let url = window.location.href;
  console.log("sahkal incoming");
  let { paymentId, merchantId, attemptId, connector } = parseRoute(url);
  let log = {
    message: event.error.message,
    stack: event.error.stack,
    filename: event.filename,
    url,
    paymentId,
    merchantId,
    attemptId,
    connector,
  };
  postLog(log);
});
  