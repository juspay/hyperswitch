{{ payment_details_js_script }}

      function boot() {
        var paymentDetails = window.__PAYMENT_DETAILS;

        // Attach document icon
        if (paymentDetails.merchant_logo) {
          var link = document.createElement("link");
          link.rel = "icon";
          link.href = paymentDetails.merchant_logo;
          link.type = "image/x-icon";
          document.head.appendChild(link);
        }

        var statusDetails = {
          imageSource: "",
          message: "",
          status: "",
          items: [],
        };

        var paymentId = createItem("Ref Id", paymentDetails.payment_id);
        statusDetails.items.push(paymentId);

        // Decide screen to render
        switch (paymentDetails.payment_link_status) {
          case "expired": {
            statusDetails.imageSource = "https://i.imgur.com/UD8CEuY.png";
            statusDetails.status = "Payment Link Expired!";
            statusDetails.message = "This payment link is expired.";
            break;
          }

          default: {
            statusDetails.status = paymentDetails.intent_status;
            // Render status screen
            switch (paymentDetails.intent_status) {
              case "succeeded": {
                statusDetails.imageSource = "https://i.imgur.com/5BOmYVl.png";
                statusDetails.message =
                  "We have successfully received your payment";
                break;
              }

              case "processing": {
                statusDetails.imageSource = "https://i.imgur.com/Yb79Qt4.png";
                statusDetails.message =
                  "Sorry! Your payment is taking longer than expected. Please check back again in sometime.";
                statusDetails.status = "Payment Pending";
                break;
              }

              case "failed": {
                statusDetails.imageSource = "https://i.imgur.com/UD8CEuY.png";
                statusDetails.status = "Payment Failed!";
                var errorCodeNode = createItem(
                  "Error code",
                  paymentDetails.error_code
                );
                var errorMessageNode = createItem(
                  "Error message",
                  paymentDetails.error_message
                );
                // @ts-ignore
                statusDetails.items.push(errorMessageNode, errorCodeNode);
                break;
              }

              case "cancelled": {
                statusDetails.imageSource = "https://i.imgur.com/UD8CEuY.png";
                statusDetails.status = "Payment Cancelled";
                break;
              }

              case "requires_merchant_action": {
                statusDetails.imageSource = "https://i.imgur.com/Yb79Qt4.png";
                statusDetails.status = "Payment under review";
                break;
              }

              case "requires_capture": {
                statusDetails.imageSource = "https://i.imgur.com/Yb79Qt4.png";
                statusDetails.status = "Payment Pending";
                break;
              }

              case "partially_captured": {
                statusDetails.imageSource = "https://i.imgur.com/Yb79Qt4.png";
                statusDetails.message = "Partial payment was captured.";
                statusDetails.status = "Partial Payment Pending";
                break;
              }

              default:
                statusDetails.imageSource = "https://i.imgur.com/UD8CEuY.png";
                statusDetails.status = "Something went wrong";
                // Error details
                if (typeof paymentDetails.error === "object") {
                  var errorCodeNode = createItem(
                    "Error Code",
                    paymentDetails.error.code
                  );
                  var errorMessageNode = createItem(
                    "Error Message",
                    paymentDetails.error.message
                  );
                  // @ts-ignore
                  statusDetails.items.push(errorMessageNode, errorCodeNode);
                }
            }
          }
        }

        // Form header
        var hyperCheckoutImageNode = document.createElement("img");
        var hyperCheckoutAmountNode = document.createElement("div");

        hyperCheckoutImageNode.src = paymentDetails.merchant_logo;
        hyperCheckoutImageNode.className =
          "hyper-checkout-status-merchant-logo";
        hyperCheckoutAmountNode.innerText =
          paymentDetails.currency + " " + paymentDetails.amount;
        hyperCheckoutAmountNode.className = "hyper-checkout-status-amount";
        var hyperCheckoutHeaderNode = document.getElementById(
          "hyper-checkout-status-header"
        );
        if (hyperCheckoutHeaderNode instanceof HTMLDivElement) {
          hyperCheckoutHeaderNode.append(
            hyperCheckoutAmountNode,
            hyperCheckoutImageNode
          );
        }

        // Form and append items
        var hyperCheckoutStatusTextNode = document.createElement("div");
        hyperCheckoutStatusTextNode.innerText = statusDetails.status;
        hyperCheckoutStatusTextNode.className = "hyper-checkout-status-text";

        var merchantLogoNode = document.createElement("img");
        merchantLogoNode.src = statusDetails.imageSource;
        merchantLogoNode.className = "hyper-checkout-status-image";

        var hyperCheckoutStatusMessageNode = document.createElement("div");
        hyperCheckoutStatusMessageNode.innerText = statusDetails.message;

        var hyperCheckoutDetailsNode = document.createElement("div");
        hyperCheckoutDetailsNode.className = "hyper-checkout-status-details";
        if (hyperCheckoutDetailsNode instanceof HTMLDivElement) {
          hyperCheckoutDetailsNode.append(...statusDetails.items);
        }

        var hyperCheckoutContentNode = document.getElementById(
          "hyper-checkout-status-content"
        );
        if (hyperCheckoutContentNode instanceof HTMLDivElement) {
          hyperCheckoutContentNode.prepend(
            merchantLogoNode,
            hyperCheckoutStatusTextNode,
            hyperCheckoutDetailsNode
          );
        }
        var statusRedirectTextNode = document.getElementById(
            "hyper-checkout-status-redirect-message"
          );
          if (
            statusRedirectTextNode ||
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
                statusRedirectTextNode.innerText = innerText;
                if (secondsLeft === 0) {
                  // Form query params
                  var queryParams = {
                    payment_id: paymentDetails.payment_id,
                    status: paymentDetails.status,
                    payment_intent_client_secret: paymentDetails.client_secret,
                    amount: amount,
                    manual_retry_allowed: paymentDetails.manual_retry_allowed,
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


      function createItem(heading, value) {
        var itemNode = document.createElement("div");
        itemNode.className = "hyper-checkout-status-item";
        var headerNode = document.createElement("div");
        headerNode.className = "hyper-checkout-item-header";
        headerNode.innerText = heading;
        var valueNode = document.createElement("div");
        valueNode.classList.add("hyper-checkout-item-value");
        // valueNode.classList.add("ellipsis-container-2");
        valueNode.innerText = value;
        itemNode.append(headerNode);
        itemNode.append(valueNode);
        return itemNode;
      }