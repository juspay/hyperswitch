// @ts-check

/**
 * UTIL FUNCTIONS
 */

/**
 * Ref - https://github.com/onury/invert-color/blob/master/lib/cjs/invert.js
 */
function padz(str, len) {
  if (len === void 0) {
    len = 2;
  }
  return (new Array(len).join("0") + str).slice(-len);
}
function hexToRgbArray(hex) {
  if (hex.slice(0, 1) === "#") hex = hex.slice(1);
  var RE_HEX = /^(?:[0-9a-f]{3}){1,2}$/i;
  if (!RE_HEX.test(hex)) throw new Error('Invalid HEX color: "' + hex + '"');
  if (hex.length === 3) {
    hex = hex[0] + hex[0] + hex[1] + hex[1] + hex[2] + hex[2];
  }
  return [
    parseInt(hex.slice(0, 2), 16),
    parseInt(hex.slice(2, 4), 16),
    parseInt(hex.slice(4, 6), 16),
  ];
}
function toRgbArray(c) {
  if (!c) throw new Error("Invalid color value");
  if (Array.isArray(c)) return c;
  return typeof c === "string" ? hexToRgbArray(c) : [c.r, c.g, c.b];
}
function getLuminance(c) {
  var i, x;
  var a = [];
  for (i = 0; i < c.length; i++) {
    x = c[i] / 255;
    a[i] = x <= 0.03928 ? x / 12.92 : Math.pow((x + 0.055) / 1.055, 2.4);
  }
  return 0.2126 * a[0] + 0.7152 * a[1] + 0.0722 * a[2];
}
function invertToBW(color, bw, asArr) {
  var DEFAULT_BW = {
    black: "#090302",
    white: "#FFFFFC",
    threshold: Math.sqrt(1.05 * 0.05) - 0.05,
  };
  var options = bw === true ? DEFAULT_BW : Object.assign({}, DEFAULT_BW, bw);
  return getLuminance(color) > options.threshold
    ? asArr
      ? hexToRgbArray(options.black)
      : options.black
    : asArr
    ? hexToRgbArray(options.white)
    : options.white;
}
function invert(color, bw) {
  if (bw === void 0) {
    bw = false;
  }
  color = toRgbArray(color);
  if (bw) return invertToBW(color, bw);
  return (
    "#" +
    color
      .map(function (c) {
        return padz((255 - c).toString(16));
      })
      .join("")
  );
}

/**
 * UTIL FUNCTIONS END HERE
 */

// @ts-ignore
{{ payment_details_js_script }}

// @ts-ignore
window.state = {
  prevHeight: window.innerHeight,
  prevWidth: window.innerWidth,
  isMobileView: window.innerWidth <= 1400,
};

/**
 * Trigger - init function invoked once the script tag is loaded
 * Use
 *  - Update document's title
 *  - Update document's icon
 *  - Render and populate document with payment details and cart
 *  - Initialize event listeners for updating UI on screen size changes
 *  - Initialize SDK
 **/
function boot() {
  // @ts-ignore
  var paymentDetails = window.__PAYMENT_DETAILS;

  // Attach document icon
  if (paymentDetails.merchant_logo) {
    var link = document.createElement("link");
    link.rel = "icon";
    link.href = paymentDetails.merchant_logo;
    link.type = "image/x-icon";
    document.head.appendChild(link);
  }

  // Render status details
  renderStatusDetails(paymentDetails);

  // Add event listeners
  initializeEventListeners(paymentDetails);
}

/**
 * Trigger - on boot
 * Uses
 *    - Render status details
 *      - Header - (amount, merchant name, merchant logo)
 *      - Body - status with image
 *      - Footer - payment details (id | error code and msg, if any)
 * @param {PaymentDetails} paymentDetails
 **/
function renderStatusDetails(paymentDetails) {
  var status = paymentDetails.status;

  var statusDetails = {
    imageSource: "",
    message: "",
    status: status,
    amountText: "",
    items: [],
  };

  // Payment details
  var paymentId = createItem("Ref Id", paymentDetails.payment_id);
  // @ts-ignore
  statusDetails.items.push(paymentId);

  // Status specific information
  switch (status) {
    case "expired":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/failed.png";
      statusDetails.status = "Payment Link Expired";
      statusDetails.message =
        "Sorry, this payment link has expired. Please use below reference for further investigation.";
      break;

    case "succeeded":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/success.png";
      statusDetails.message = "We have successfully received your payment";
      statusDetails.status = "Paid successfully";
      statusDetails.amountText = new Date(
        paymentDetails.created
      ).toTimeString();
      break;

    case "processing":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/pending.png";
      statusDetails.message =
        "Sorry! Your payment is taking longer than expected. Please check back again in sometime.";
      statusDetails.status = "Payment Pending";
      break;

    case "failed":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/failed.png";
      statusDetails.status = "Payment Failed!";
      var errorCodeNode = createItem("Error code", paymentDetails.error_code);
      var errorMessageNode = createItem(
        "Error message",
        paymentDetails.error_message
      );
      // @ts-ignore
      statusDetails.items.push(errorMessageNode, errorCodeNode);
      break;

    case "cancelled":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/failed.png";
      statusDetails.status = "Payment Cancelled";
      break;

    case "requires_merchant_action":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/pending.png";
      statusDetails.status = "Payment under review";
      break;

    case "requires_capture":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/success.png";
      statusDetails.message = "We have successfully received your payment";
      statusDetails.status = "Payment Success";
      break;

    case "partially_captured":
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/success.png";
      statusDetails.message = "Partial payment was captured.";
      statusDetails.status = "Payment Success";
      break;

    default:
      statusDetails.imageSource = "https://live.hyperswitch.io/payment-link-assets/failed.png";
      statusDetails.status = "Something went wrong";
      // Error details
      if (typeof paymentDetails.error === "object") {
        var errorCodeNode = createItem("Error Code", paymentDetails.error.code);
        var errorMessageNode = createItem(
          "Error Message",
          paymentDetails.error.message
        );
        // @ts-ignore
        statusDetails.items.push(errorMessageNode, errorCodeNode);
      }
      break;
  }

  // Form header items
  var amountNode = document.createElement("div");
  amountNode.className = "hyper-checkout-status-amount";
  amountNode.innerText = paymentDetails.currency + " " + paymentDetails.amount;
  var merchantLogoNode = document.createElement("img");
  merchantLogoNode.className = "hyper-checkout-status-merchant-logo";
  // @ts-ignore
  merchantLogoNode.src = window.__PAYMENT_DETAILS.merchant_logo;
  merchantLogoNode.alt = "";

  // Form content items
  var statusImageNode = document.createElement("img");
  statusImageNode.className = "hyper-checkout-status-image";
  statusImageNode.src = statusDetails.imageSource;
  var statusTextNode = document.createElement("div");
  statusTextNode.className = "hyper-checkout-status-text";
  statusTextNode.innerText = statusDetails.status;
  var statusMessageNode = document.createElement("div");
  statusMessageNode.className = "hyper-checkout-status-message";
  statusMessageNode.innerText = statusDetails.message;
  var statusDetailsNode = document.createElement("div");
  statusDetailsNode.className = "hyper-checkout-status-details";

  // Append items
  if (statusDetailsNode instanceof HTMLDivElement) {
    statusDetails.items.map(function (item) {
      statusDetailsNode.append(item);
    });
  }
  var statusHeaderNode = document.getElementById(
    "hyper-checkout-status-header"
  );
  if (statusHeaderNode instanceof HTMLDivElement) {
    statusHeaderNode.append(amountNode, merchantLogoNode);
  }
  var statusContentNode = document.getElementById(
    "hyper-checkout-status-content"
  );
  if (statusContentNode instanceof HTMLDivElement) {
    statusContentNode.append(statusImageNode, statusTextNode);
    if (statusMessageNode instanceof HTMLDivElement) {
      statusContentNode.append(statusMessageNode);
    }
    statusContentNode.append(statusDetailsNode);
  }

  if (paymentDetails.redirect === true) {
    // Form redirect text
    var statusRedirectTextNode = document.getElementById(
      "hyper-checkout-status-redirect-message"
    );
    if (
      statusRedirectTextNode instanceof HTMLDivElement &&
      typeof paymentDetails.return_url === "string"
    ) {
      var timeout = 5,
        j = 0;
      for (var i = 0; i <= timeout; i++) {
        setTimeout(function () {
          var secondsLeft = timeout - j++;
          var innerText =
            secondsLeft === 0
              ? "Redirecting ..."
              : "Redirecting in " + secondsLeft + " seconds ...";
          // @ts-ignore
          statusRedirectTextNode.innerText = innerText;
          if (secondsLeft === 0) {
            // Form query params
            var queryParams = {
              payment_id: paymentDetails.payment_id,
              status: paymentDetails.status,
            };
            var url = new URL(paymentDetails.return_url);
            var params = new URLSearchParams(url.search);
            // Attach query params to return_url
            for (var key in queryParams) {
              if (queryParams.hasOwnProperty(key)) {
                params.set(key, queryParams[key]);
              }
            }
            url.search = params.toString();
            setTimeout(function () {
              // Finally redirect
              window.location.href = url.toString();
            }, 1000);
          }
        }, i * 1000);
      }
    }
  }
}

/**
 * Use - create an item which is a key-value pair of some information related to a payment
 * @param {String} heading
 * @param {String} value
 **/
function createItem(heading, value) {
  var itemNode = document.createElement("div");
  itemNode.className = "hyper-checkout-status-item";
  var headerNode = document.createElement("div");
  headerNode.className = "hyper-checkout-item-header";
  headerNode.innerText = heading;
  var valueNode = document.createElement("div");
  valueNode.className = "hyper-checkout-item-value";
  valueNode.innerText = value;
  itemNode.append(headerNode);
  itemNode.append(valueNode);
  return itemNode;
}

/**
 * Use - add event listeners for changing UI on screen resize
 * @param {PaymentDetails} paymentDetails
 */
function initializeEventListeners(paymentDetails) {
  var primaryColor = paymentDetails.theme;
  var contrastBWColor = invert(primaryColor, true);
  var statusRedirectTextNode = document.getElementById(
    "hyper-checkout-status-redirect-message"
  );

  if (statusRedirectTextNode instanceof HTMLDivElement) {
    statusRedirectTextNode.style.color = contrastBWColor;
  }

    // @ts-ignore
    window.state.prevHeight = currentHeight;
    // @ts-ignore
    window.state.prevWidth = currentWidth;
    // @ts-ignore
    window.state.isMobileView = currentWidth <= 1400;
  };

