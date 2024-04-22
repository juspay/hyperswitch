// @ts-check

var widgets = null;
var paymentMethodCollect = null;
// @ts-ignore
var pub_key = window.__PM_COLLECT_DETAILS.pub_key;
var hyper = null;

/**
 * Trigger - init
 * Uses
 *  - Instantiate SDK
 */
function boot() {
  // @ts-ignore
  var paymentMethodCollectDetails = window.__PM_COLLECT_DETAILS;

  // Initialize SDK
  // @ts-ignore
  if (window.Hyper) {
    initializeCollectSDK();
  }
}
boot();

/**
 * Trigger - post downloading SDK
 * Uses
 *  - Instantiate SDK
 *  - Create a payment method collect widget
 *  - Mount it in DOM
 **/
function initializeCollectSDK() {
  // @ts-ignore
  var paymentMethodCollectDetails = window.__PM_COLLECT_DETAILS;
  var clientSecret = paymentMethodCollectDetails.client_secret;
  var appearance = {
    variables: {
      colorPrimary: paymentMethodCollectDetails?.theme?.primary_color || "rgb(0, 109, 249)",
      fontFamily: "Work Sans, sans-serif",
      fontSizeBase: "16px",
      colorText: "rgb(51, 65, 85)",
      colorTextSecondary: "#334155B3",
      colorPrimaryText: "rgb(51, 65, 85)",
      colorTextPlaceholder: "#33415550",
      borderColor: "#33415550",
      colorBackground: "rgb(255, 255, 255)",
    },
  };
  // Instantiate
  // @ts-ignore
  hyper = window.Hyper(pub_key, {
    isPreloadEnabled: false,
  });
  widgets = hyper.widgets({
    appearance: appearance,
    clientSecret: clientSecret,
  });

  // Create payment method collect widget
  var paymentMethodCollectOptions = {};
  paymentMethodCollect = widgets.create(
    "paymentMethodCollect",
    paymentMethodCollectOptions
  );

  // Mount
  if (paymentMethodCollect !== null) {
    paymentMethodCollect.mount("#payment-method-collect")
  }
}

