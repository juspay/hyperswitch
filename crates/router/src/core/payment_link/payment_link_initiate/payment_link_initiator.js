// @ts-check

/**
 * Trigger - post downloading SDK
 * Uses
 *  - Instantiate SDK
 *  - Create a payment widget
 *  - Decide whether or not to show SDK (based on status)
 **/
function initializeSDK() {
  // @ts-ignore
  var paymentDetails = window.__PAYMENT_DETAILS;
  var client_secret = paymentDetails.client_secret;
  var appearance = {
    variables: {
      colorPrimary: paymentDetails.theme || "rgb(0, 109, 249)",
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
  // @ts-ignore
  hyper = window.Hyper(pub_key, {
    isPreloadEnabled: false,
    // TODO: Remove in next deployment
    shouldUseTopRedirection: true,
    redirectionFlags: {
      shouldRemoveBeforeUnloadEvents: true,
      shouldUseTopRedirection: true,
    },
  });
  // @ts-ignore
  widgets = hyper.widgets({
    appearance: appearance,
    clientSecret: client_secret,
    locale: paymentDetails.locale,
  });
  var type =
    paymentDetails.sdk_layout === "spaced_accordion" ||
    paymentDetails.sdk_layout === "accordion"
      ? "accordion"
      : paymentDetails.sdk_layout;
  var hideCardNicknameField = paymentDetails.hide_card_nickname_field;
  var unifiedCheckoutOptions = {
    displaySavedPaymentMethodsCheckbox: false,
    displaySavedPaymentMethods: false,
    layout: {
      type: type, //accordion , tabs, spaced accordion
      spacedAccordionItems: paymentDetails.sdk_layout === "spaced_accordion",
    },
    branding: "never",
    wallets: {
      walletReturnUrl: paymentDetails.return_url,
      style: {
        theme: "dark",
        type: "default",
        height: 55,
      },
    },
    showCardFormByDefault: paymentDetails.show_card_form_by_default,
    hideCardNicknameField: hideCardNicknameField,
  };
  // @ts-ignore
  unifiedCheckout = widgets.create("payment", unifiedCheckoutOptions);
  // @ts-ignore
  mountUnifiedCheckout("#unified-checkout");
  // @ts-ignore
  showSDK(paymentDetails.display_sdk_only);

  let shimmer = document.getElementById("payment-details-shimmer");
  shimmer.classList.add("reduce-opacity");

  setTimeout(() => {
    document.body.removeChild(shimmer);
  }, 500);
}

/**
 * Use - redirect to /payment_link/status
 */
function redirectToStatus() {
  var paymentDetails = window.__PAYMENT_DETAILS;
  var arr = window.location.pathname.split("/");

  // NOTE - This code preserves '/api' in url for integ and sbx
  // e.g. url for integ/sbx - https://integ.hyperswitch.io/api/payment_link/merchant_1234/pay_1234?locale=en
  // e.g. url for others - https://abc.dev.com/payment_link/merchant_1234/pay_1234?locale=en
  var hasApiInPath = arr.includes("api");
  if (hasApiInPath) {
    arr.splice(0, 3);
    arr.unshift("api", "payment_link", "status");
  } else {
    arr.splice(0, 2);
    arr.unshift("payment_link", "status");
  }

  window.location.href =
    window.location.origin +
    "/" +
    arr.join("/") +
    "?locale=" +
    paymentDetails.locale;
}
