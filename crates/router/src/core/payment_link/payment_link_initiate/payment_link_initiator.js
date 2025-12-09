// @ts-nocheck

const toCamel = (s) => {
  return s.replace(/([-_][a-z])/gi, ($1) => {
    return $1.toUpperCase().replace("-", "").replace("_", "");
  });
};

const iisObject = function (obj) {
  return obj === Object(obj) && !Array.isArray(obj) && typeof obj !== "function";
};

const keysToCamel = function (obj) {
  if (iisObject(obj)) {
    const n = {};

    Object.keys(obj).forEach((k) => {
      n[toCamel(k)] = keysToCamel(obj[k]);
    });

    return n;
  } else if (Array.isArray(obj)) {
    return obj.map((i) => {
      return keysToCamel(i);
    });
  }

  return obj;
};

/**
 * Trigger - post downloading SDK
 * Uses
 *  - Instantiate SDK
 *  - Create a payment widget
 *  - Decide whether or not to show SDK (based on status)
 **/
function initializeSDK() {
  // @ts-ignore
  var encodedPaymentDetails = window.__PAYMENT_DETAILS;
  var decoded = decodeURIComponent(encodedPaymentDetails);
  var paymentDetails = keysToCamel(JSON.parse(decoded));
  var clientSecret = paymentDetails.clientSecret;
  var sdkUiRules = paymentDetails.sdkUiRules;
  var labelType = paymentDetails.paymentFormLabelType;
  var colorIconCardCvcError = paymentDetails.colorIconCardCvcError;
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
  if (isObject(sdkUiRules)) {
    appearance.rules = sdkUiRules;
  }
  if (labelType !== null && typeof labelType === "string") {
    appearance.labels = labelType;
  }
  if (colorIconCardCvcError !== null && typeof colorIconCardCvcError === "string") {
    appearance.variables.colorIconCardCvcError = colorIconCardCvcError;
  }
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
    clientSecret: clientSecret,
    locale: paymentDetails.locale,
  });
  var type =
    paymentDetails.sdkLayout === "spaced_accordion" ||
    paymentDetails.sdkLayout === "accordion"
      ? "accordion"
      : paymentDetails.sdkLayout;
  var hideCardNicknameField = paymentDetails.hideCardNicknameField;
  var unifiedCheckoutOptions = {
    displaySavedPaymentMethodsCheckbox: false,
    displaySavedPaymentMethods: false,
    layout: {
      type: type, //accordion , tabs, spaced accordion
      spacedAccordionItems: paymentDetails.sdkLayout === "spaced_accordion",
    },
    branding: "never",
    wallets: {
      walletReturnUrl: paymentDetails.returnUrl,
      style: {
        theme: "dark",
        type: "default",
        height: 55,
      },
    },
    showCardFormByDefault: paymentDetails.showCardFormByDefault,
    hideCardNicknameField: hideCardNicknameField,
    customMessageForCardTerms: paymentDetails.customMessageForCardTerms,
    paymentMethodsConfig: paymentDetails.customMessageForPaymentMethodTypes,
  };
  var showCardTerms = paymentDetails.showCardTerms;
  if (showCardTerms !== null && typeof showCardTerms === "string") {
    unifiedCheckoutOptions.terms = {
      card: showCardTerms
    };
  }
  var paymentMethodsHeaderText = paymentDetails.paymentFormHeaderText;
  if (paymentMethodsHeaderText !== null && typeof paymentMethodsHeaderText === "string") {
    unifiedCheckoutOptions.paymentMethodsHeaderText = paymentMethodsHeaderText;
  }
  unifiedCheckout = widgets.create("payment", unifiedCheckoutOptions);
  mountUnifiedCheckout("#unified-checkout");
  showSDK(paymentDetails.displaySdkOnly, paymentDetails.enableButtonOnlyOnFormReady);

  let shimmer = document.getElementById("payment-details-shimmer");
  shimmer.classList.add("reduce-opacity");

  setTimeout(() => {
    document.body.removeChild(shimmer);
  }, 500);
}

/**
 * Use - redirect to /payment_link/status
 */
function redirectToStatus(paymentDetails) {
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
