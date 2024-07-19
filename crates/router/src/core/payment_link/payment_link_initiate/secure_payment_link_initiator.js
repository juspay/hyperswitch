// @ts-check

// Top level checks
var isFramed = false;
try {
  isFramed = window.parent.location !== window.location;

  // If parent's window object is restricted, DOMException is
  // thrown which concludes that the webpage is iframed
} catch (err) {
  isFramed = true;
}

if (!isFramed) {
  function initializeSDK() {
    var errMsg = "You are not allowed to view this content.";
    var contentElement = document.getElementById("payout-link");
    if (contentElement instanceof HTMLDivElement) {
      contentElement.innerHTML = errMsg;
    } else {
      document.body.innerHTML = errMsg;
    }
  }
} else {
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
    });
    widgets = hyper.widgets({
      appearance: appearance,
      clientSecret: client_secret,
    });
    var type =
      paymentDetails.sdk_layout === "spaced_accordion" ||
      paymentDetails.sdk_layout === "accordion"
        ? "accordion"
        : paymentDetails.sdk_layout;

    var enableSavedPaymentMethod = paymentDetails.enabled_saved_payment_method;
    var unifiedCheckoutOptions = {
      displaySavedPaymentMethodsCheckbox: enableSavedPaymentMethod,
      displaySavedPaymentMethods: enableSavedPaymentMethod,
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
    };
    unifiedCheckout = widgets.create("payment", unifiedCheckoutOptions);
    mountUnifiedCheckout("#unified-checkout");
    showSDK(paymentDetails.display_sdk_only);

    let shimmer = document.getElementById("payment-details-shimmer");
    shimmer.classList.add("reduce-opacity");

    setTimeout(() => {
      document.body.removeChild(shimmer);
    }, 500);
  }
}
