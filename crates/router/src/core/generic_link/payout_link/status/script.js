// @ts-check
/**
 * Trigger - init
 * Uses
 *  - Update document's icon
 */
function boot() {
  // @ts-ignore
  var payoutDetails = window.__PAYOUT_DETAILS;

  // Attach document icon
  if (typeof payoutDetails.logo === "string") {
    var link = document.createElement("link");
    link.rel = "icon";
    link.href = payoutDetails.logo;
    link.type = "image/x-icon";
    document.head.appendChild(link);
  }
  // Render status details
  renderStatusDetails(payoutDetails);
  // Redirect
  if (typeof payoutDetails.return_url === "string") {
    // Form query params
    var queryParams = {
      payout_id: payoutDetails.payout_id,
      status: payoutDetails.status,
    };
    var url = new URL(payoutDetails.return_url);
    var params = new URLSearchParams(url.search);
    // Attach query params to return_url
    for (var key in queryParams) {
      if (queryParams.hasOwnProperty(key)) {
        params.set(key, queryParams[key]);
      }
    }
    url.search = params.toString();
    redirectToEndUrl(url);
  }
}
boot();

/**
 * Trigger - on boot
 * Uses
 *  - Render merchant details
 *  - Render status
 */
function renderStatusDetails(payoutDetails) {
  var statusCardNode = document.getElementById("status-card");
  var merchantHeaderNode = document.getElementById("merchant-header");

  if (
    typeof payoutDetails.merchant_name === "string" &&
    merchantHeaderNode instanceof HTMLDivElement
  ) {
    var merchantNameNode = document.createElement("div");
    merchantNameNode.innerText = payoutDetails.merchant_name;
    merchantHeaderNode.appendChild(merchantNameNode);
  }
  if (
    typeof payoutDetails.logo === "string" &&
    merchantHeaderNode instanceof HTMLDivElement
  ) {
    var merchantLogoNode = document.createElement("img");
    merchantLogoNode.src = payoutDetails.logo;
    merchantHeaderNode.appendChild(merchantLogoNode);
  }
  var status = payoutDetails.status;
  var statusInfo = {
    statusImageSrc:
      "https://live.hyperswitch.io/payment-link-assets/success.png",
    statusText: "Payout Successful",
    statusMessage: "Your payout was made to selected payment method.",
  };
  switch (status) {
    case "success":
      break;
    case "initiated":
    case "pending":
      statusInfo.statusImageSrc =
        "https://live.hyperswitch.io/payment-link-assets/pending.png";
      statusInfo.statusText = "Payout Processing";
      statusInfo.statusMessage =
        "Your payout should be processed within 2-3 business days.";
      break;
    case "failed":
    case "cancelled":
    case "expired":
    case "reversed":
    case "ineligible":
    case "requires_creation":
    case "requires_confirmation":
    case "requires_payout_method_data":
    case "requires_fulfillment":
    case "requires_vendor_account_creation":
    default:
      statusInfo.statusImageSrc =
        "https://live.hyperswitch.io/payment-link-assets/failed.png";
      statusInfo.statusText = "Payout Failed";
      statusInfo.statusMessage =
        "Failed to process your payout. Please check with your provider for more details.";
      break;
  }

  var statusImageNode = document.createElement("img");
  statusImageNode.src = statusInfo.statusImageSrc;
  statusImageNode.id = "status-image";
  var statusTextNode = document.createElement("div");
  statusTextNode.innerText = statusInfo.statusText;
  statusTextNode.id = "status-text";
  var statusMsgNode = document.createElement("div");
  statusMsgNode.innerText = statusInfo.statusMessage;
  statusMsgNode.id = "status-message";

  // Append status info
  if (statusCardNode instanceof HTMLDivElement) {
    statusCardNode.appendChild(statusImageNode);
    statusCardNode.appendChild(statusTextNode);
    statusCardNode.appendChild(statusMsgNode);
  }

  var resourceInfo = {
    "Ref Id": payoutDetails.payout_id,
  };
  if (typeof payoutDetails.error_code === "string") {
    resourceInfo["Error Code"] = payoutDetails.error_code;
  }
  if (typeof payoutDetails.error_message === "string") {
    resourceInfo["Error Message"] = payoutDetails.error_message;
  }
  var resourceNode = document.createElement("div");
  resourceNode.id = "resource-info-container";
  for (var key in resourceInfo) {
    var infoNode = document.createElement("div");
    infoNode.id = "resource-info";
    var infoKeyNode = document.createElement("div");
    infoKeyNode.innerText = key;
    infoKeyNode.id = "info-key";
    var infoValNode = document.createElement("div");
    infoValNode.innerText = resourceInfo[key];
    infoValNode.id = "info-val";
    infoNode.appendChild(infoKeyNode);
    infoNode.appendChild(infoValNode);
    resourceNode.appendChild(infoNode);
  }

  // Append resource info
  if (statusCardNode instanceof HTMLDivElement) {
    statusCardNode.appendChild(resourceNode);
  }
}

/**
 * Trigger - if return_url was specified during payout link creation
 * Uses
 *  - Redirect to end url
 * @param {URL} returnUrl
 */
function redirectToEndUrl(returnUrl) {
  // Form redirect text
  var statusRedirectTextNode = document.getElementById("redirect-text");
  var timeout = 5,
    j = 0;
  for (var i = 0; i <= timeout; i++) {
    setTimeout(function () {
      var secondsLeft = timeout - j++;
      var innerText =
        secondsLeft === 0
          ? "Redirecting ..."
          : "Redirecting in " + secondsLeft + " seconds ...";
      if (statusRedirectTextNode instanceof HTMLDivElement) {
        statusRedirectTextNode.innerText = innerText;
      }
      if (secondsLeft === 0) {
        setTimeout(function () {
          window.location.href = returnUrl.toString();
        }, 1000);
      }
    }, i * 1000);
  }
}
