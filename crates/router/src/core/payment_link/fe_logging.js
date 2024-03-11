function parseRoute(url) {
    const route = new URL(url).pathname;
    const regex =
      /^\/(?:redirect\/)?([^/]+)\/([^/]+)(?:\/redirect\/(?:response|complete)\/([^/]+))?(?:\/([^/]+))?$/;
    const matches = route.match(regex);
    const attemptIdExists = !(
      route.includes("response") || route.includes("complete")
    );
  
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
  
  function initiateLogListener (){
    window.addEventListener("error", (event) => {
      let url = window.location.href;
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
  }