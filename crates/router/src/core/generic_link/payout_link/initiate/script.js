// @ts-check

// Top level checks
// @ts-ignore
var payoutDetails = window.__PAYOUT_DETAILS;
var isTestMode = payoutDetails.test_mode;

var isFramed = false;
try {
  isFramed = window.parent.location !== window.location;

  // If parent's window object is restricted, DOMException is
  // thrown which concludes that the webpage is iframed
} catch (err) {
  isFramed = true;
}

// Remove the script from DOM incase it's not iframed
if (!isTestMode && !isFramed) {
  function initializePayoutSDK() {
    var errMsg = "{{i18n_not_allowed}}";
    var contentElement = document.getElementById("payout-link");
    if (contentElement instanceof HTMLDivElement) {
      contentElement.innerHTML = errMsg;
    } else {
      document.body.innerHTML = errMsg;
    }
  }

  // webpage is iframed, good to load
} else {
  var hyper = null;
  var payoutWidget = null;
  var widgets = null;
  /**
   * Use - format date in "hh:mm AM/PM timezone MM DD, YYYY"
   * @param {Date} date
   **/
  function formatDate(date) {
    var months = [
      "{{i18n_january}}",
      "{{i18n_february}}",
      "{{i18n_march}}",
      "{{i18n_april}}",
      "{{i18n_may}}",
      "{{i18n_june}}",
      "{{i18n_july}}",
      "{{i18n_august}}",
      "{{i18n_september}}",
      "{{i18n_october}}",
      "{{i18n_november}}",
      "{{i18n_december}}",
    ];

    var hours = date.getHours();
    var minutes = date.getMinutes();
    // @ts-ignore
    minutes = minutes < 10 ? "0" + minutes : minutes;
    var suffix = hours > 11 ? "{{i18n_pm}}" : "{{i18n_am}}";
    hours = hours % 12;
    hours = hours ? hours : 12;
    var day = date.getDate();
    var month = months[date.getMonth()];
    var year = date.getUTCFullYear();

    // @ts-ignore
    var locale = navigator.language || navigator.userLanguage;
    var timezoneShorthand = date
      .toLocaleDateString(locale, {
        day: "2-digit",
        timeZoneName: "long",
      })
      .substring(4)
      .split(" ")
      .reduce(function (tz, c) {
        return tz + c.charAt(0).toUpperCase();
      }, "");

    var formatted =
      hours +
      ":" +
      minutes +
      " " +
      suffix +
      " " +
      timezoneShorthand +
      "  " +
      month +
      " " +
      day +
      ", " +
      year;
    return formatted;
  }

  /**
   * Trigger - init
   * Uses
   *  - Initialize SDK
   *  - Update document's icon
   */
  function boot() {
    // Initialize SDK
    // @ts-ignore
    if (window.Hyper) {
      initializePayoutSDK();
    }

    // @ts-ignore
    var payoutDetails = window.__PAYOUT_DETAILS;

    // Attach document icon
    if (payoutDetails.logo) {
      var link = document.createElement("link");
      link.rel = "icon";
      link.href = payoutDetails.logo;
      link.type = "image/x-icon";
      document.head.appendChild(link);
    }
  }
  boot();

  /**
   * Trigger - post downloading SDK
   * Uses
   *  - Initialize SDK
   *  - Create a payout widget
   *  - Mount it in DOM
   **/
  function initializePayoutSDK() {
    // @ts-ignore
    var payoutDetails = window.__PAYOUT_DETAILS;
    var clientSecret = payoutDetails.client_secret;
    var locale = payoutDetails.locale;
    var publishableKey = payoutDetails.publishable_key;
    var appearance = {
      variables: {
        colorPrimary: payoutDetails?.theme?.primary_color || "rgb(0, 109, 249)",
        fontFamily: "ui-sans-serif, system-ui, sans-serif, 'Apple Color Emoji', 'Segoe UI Emoji', 'Segoe UI Symbol', 'Noto Color Emoji'",
        fontSizeBase: "16px",
        colorText: "rgb(51, 65, 85)",
      },
    };
    // @ts-ignore
    hyper = window.Hyper(publishableKey, {
      isPreloadEnabled: false,
      shouldUseTopRedirection: isFramed,
    });
    widgets = hyper.widgets({
      appearance: appearance,
      clientSecret: clientSecret,
      locale: locale,
    });

    // Create payment method collect widget
    let sessionExpiry = formatDate(new Date(payoutDetails.session_expiry));
    var payoutOptions = {
      linkId: payoutDetails.payout_link_id,
      payoutId: payoutDetails.payout_id,
      customerId: payoutDetails.customer_id,
      theme: payoutDetails.theme,
      collectorName: payoutDetails.merchant_name,
      logo: payoutDetails.logo,
      enabledPaymentMethods: payoutDetails.enabled_payment_methods_with_required_fields,
      returnUrl: payoutDetails.return_url,
      sessionExpiry,
      amount: payoutDetails.amount,
      currency: payoutDetails.currency,
      flow: "PayoutLinkInitiate",
      formLayout: payoutDetails.form_layout,
    };
    payoutWidget = widgets.create("paymentMethodCollect", payoutOptions);

    // Mount
    if (payoutWidget !== null) {
      payoutWidget.mount("#payout-link");
    }
  }
}
