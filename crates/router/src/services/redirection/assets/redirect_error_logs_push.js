function parseRoute(url) {
    const route = new URL(url).pathname;
    const regex = /^\/payments\/redirect\/([^/]+)\/([^/]+)\/([^/]+)$|^\/payments\/([^/]+)\/([^/]+)\/redirect\/response\/([^/]+)(?:\/([^/]+)\/?)?$|^\/payments\/([^/]+)\/([^/]+)\/redirect\/complete\/([^/]+)$/;
    const matches = route.match(regex);
    const attemptIdExists = !(
      route.includes("response") || route.includes("complete")
    );
    if (matches) {
      const [, paymentId, merchantId, attemptId, connector,credsIdentifier] =
        matches;
      return {
        paymentId,
        merchantId,
        attemptId: attemptIdExists ? attemptId : "",
        connector
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

  function getEnvRoute(url) {
    const route = new URL(url).hostname;
    return route === "api.hyperswitch.io" ? "https://api.hyperswitch.io/logs/redirection" : "https://sandbox.hyperswitch.io/logs/redirection";
}

  
async function postLog(log, urlToPost) {

    try {
        const response = await fetch(urlToPost, {
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
    let { paymentId, merchantId, attemptId, connector } = parseRoute(url);
    let urlToPost = getEnvRoute(url);
    let log = {
        message: event.message,
        url,
        paymentId,
        merchantId,
        attemptId,
        connector,
    };
    postLog(log, urlToPost);
});
  
window.addEventListener("message", (event) => {
    let url = window.location.href;
    let { paymentId, merchantId, attemptId, connector } = parseRoute(url);
    let urlToPost = getEnvRoute(url);
    let log = {
        message: event.data,
        url,
        paymentId,
        merchantId,
        attemptId,
        connector,
    };
    postLog(log, urlToPost);
});
    
    