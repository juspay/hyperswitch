// @ts-check

var widgets = null;
var payoutWidget = null;
// @ts-ignore
var pub_key = window.__PAYOUT_DETAILS.pub_key;
var hyper = null;

/**
 * Trigger - init
 * Uses
 *  - Instantiate SDK
 */
function boot() {
  // Initialize SDK
  // @ts-ignore
  if (window.Hyper) {
    initializePayoutSDK();
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
function initializePayoutSDK() {
  // @ts-ignore
  var payoutDetails = window.__PAYOUT_DETAILS;
  var clientSecret = payoutDetails.client_secret;
  var appearance = {
    variables: {
      colorPrimary:
        payoutDetails?.theme?.primary_color || "rgb(0, 109, 249)",
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
  var payoutOptions = {
    linkId: payoutDetails.payout_link_id,
    payoutId: payoutDetails.payout_id,
    customerId: payoutDetails.customer_id,
    theme: payoutDetails.theme,
    collectorName: payoutDetails.collector_name,
    logo: payoutDetails.logo,
    enabledPaymentMethods: payoutDetails.enabled_payment_methods,
    returnUrl: payoutDetails.return_url,
    sessionExpiry: payoutDetails.session_expiry,
    amount: payoutDetails.amount,
    currency: payoutDetails.currency,
    flow: payoutDetails.flow,
  };
  payoutWidget = widgets.create(
    "paymentMethodCollect",
    payoutOptions
  );

  // Mount
  if (payoutWidget !== null) {
    payoutWidget.mount("#payout-link");
  }
}
