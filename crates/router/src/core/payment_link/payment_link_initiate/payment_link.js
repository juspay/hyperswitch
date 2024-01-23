{{ payment_details_js_script }}

      window.state = {
        prevHeight: window.innerHeight,
        prevWidth: window.innerWidth,
        isMobileView: window.innerWidth <= 1400,
        currentScreen: "payment_link",
      };

      var widgets = null;
      var unifiedCheckout = null;
      var pub_key = window.__PAYMENT_DETAILS.pub_key;
      var hyper = null;

      // Boot functions
      function boot() {
        // Update HTML doc
        var paymentDetails = window.__PAYMENT_DETAILS;

        if (paymentDetails.merchant_name) {
          document.title =
            "Payment requested by " + paymentDetails.merchant_name;
        }

        if (paymentDetails.merchant_logo) {
          var link = document.createElement("link");
          link.rel = "icon";
          link.href = paymentDetails.merchant_logo;
          link.type = "image/x-icon";
          document.head.appendChild(link);
        }

        // Render UI
        renderPaymentDetails(paymentDetails);
        renderSDKHeader(paymentDetails);
        renderCart(paymentDetails);

        // Deal w loaders
        show("#sdk-spinner");
        hide("#page-spinner");
        hide("#unified-checkout");

        // Add event listeners
        initializeEventListeners(paymentDetails);

        // Initialize SDK
        if (window.Hyper) {
          initializeSDK();
        }

        // State specific functions
        if (window.state.isMobileView) {
          show("#hyper-footer");
          hide("#hyper-checkout-cart");
        } else {
          show("#hyper-checkout-cart");
        }
      }
      boot();

      function initializeEventListeners(paymentDetails) {
        var primaryColor = paymentDetails.theme;
        var lighterColor = adjustLightness(primaryColor, 1.4);
        var darkerColor = adjustLightness(primaryColor, 0.8);
        var contrastBWColor = invert(primaryColor, true);
        var contrastingTone =
          Array.isArray(a) && a.length > 4 ? darkerColor : lighterColor;
        var hyperCheckoutNode = document.getElementById(
          "hyper-checkout-payment"
        );
        var hyperCheckoutCartImageNode = document.getElementById(
          "hyper-checkout-cart-image"
        );
        var hyperCheckoutFooterNode = document.getElementById(
          "hyper-checkout-payment-footer"
        );
        var statusRedirectTextNode = document.getElementById(
          "hyper-checkout-status-redirect-message"
        );
        var submitButtonNode = document.getElementById("submit");
        var submitButtonLoaderNode = document.getElementById("submit-spinner");

        if (submitButtonLoaderNode instanceof HTMLSpanElement) {
          submitButtonLoaderNode.style.borderBottomColor = contrastingTone;
        }

        if (submitButtonNode instanceof HTMLButtonElement) {
          submitButtonNode.style.color = contrastBWColor;
        }

        if (hyperCheckoutCartImageNode instanceof HTMLDivElement) {
          hyperCheckoutCartImageNode.style.backgroundColor = contrastingTone;
        }

        if (window.innerWidth <= 1400) {
          statusRedirectTextNode.style.color = "#333333";
          hyperCheckoutNode.style.color = contrastBWColor;
          var a = lighterColor.match(/[fF]/gi);
          hyperCheckoutFooterNode.style.backgroundColor = contrastingTone;
        } else if (window.innerWidth > 1400) {
          statusRedirectTextNode.style.color = contrastBWColor;
          hyperCheckoutNode.style.color = "#333333";
          hyperCheckoutFooterNode.style.backgroundColor = "#F5F5F5";
        }

        window.addEventListener("resize", function (event) {
          var currentHeight = window.innerHeight;
          var currentWidth = window.innerWidth;
          if (currentWidth <= 1400 && window.state.prevWidth > 1400) {
            hide("#hyper-checkout-cart");
            if (window.state.currentScreen === "payment_link") {
              show("#hyper-footer");
            }
            try {
              statusRedirectTextNode.style.color = "#333333";
              hyperCheckoutNode.style.color = contrastBWColor;
              hyperCheckoutFooterNode.style.backgroundColor = lighterColor;
            } catch (error) {
              console.error(
                "Failed to fetch primary-color, using default",
                error
              );
            }
          } else if (currentWidth > 1400 && window.state.prevWidth <= 1400) {
            if (window.state.currentScreen === "payment_link") {
              hide("#hyper-footer");
            }
            show("#hyper-checkout-cart");
            try {
              statusRedirectTextNode.style.color = contrastBWColor;
              hyperCheckoutNode.style.color = "#333333";
              hyperCheckoutFooterNode.style.backgroundColor = "#F5F5F5";
            } catch (error) {
              console.error("Failed to revert back to default colors", error);
            }
          }

          window.state.prevHeight = currentHeight;
          window.state.prevWidth = currentWidth;
          window.state.isMobileView = currentWidth <= 1400;
        });
      }

      function showSDK(paymentDetails) {
        checkStatus(paymentDetails)
          .then(function (res) {
            if (res.showSdk) {
              show("#hyper-checkout-sdk");
              show("#hyper-checkout-details");
              show("#submit");
            } else {
              hide("#hyper-checkout-details");
              hide("#hyper-checkout-sdk");
              show("#hyper-checkout-status-canvas");
              hide("#hyper-footer");
              window.state.currentScreen = "status";
            }
            show("#unified-checkout");
            hide("#sdk-spinner");
          })
          .catch(function (err) {
            console.error("Failed to check status", err);
          });
      }

      function initializeSDK() {
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
        hyper = window.Hyper(pub_key, { isPreloadEnabled: false });
        widgets = hyper.widgets({
          appearance: appearance,
          clientSecret: client_secret,
        });
        var type = (paymentDetails.sdk_layout === "spaced_accordion" || paymentDetails.sdk_layout === "accordion")
              ? "accordion"
              : paymentDetails.sdk_layout;

        var unifiedCheckoutOptions = {
          layout: {
            type: type, //accordion , tabs, spaced accordion
            spacedAccordionItems: paymentDetails.sdk_layout === "spaced_accordion"
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
        showSDK(paymentDetails);
      }

      // Util functions
      function mountUnifiedCheckout(id) {
        if (unifiedCheckout !== null) {
          unifiedCheckout.mount(id);
        }
      }

      function handleSubmit(e) {
        var paymentDetails = window.__PAYMENT_DETAILS;

        // Update button loader
        hide("#submit-button-text");
        show("#submit-spinner");
        var submitButtonNode = document.getElementById("submit");
        submitButtonNode.disabled = true;
        submitButtonNode.classList.add("disabled");

        hyper
          .confirmPayment({
            widgets: widgets,
            confirmParams: {
              // Make sure to change this to your payment completion page
              return_url: paymentDetails.return_url,
            },
          })
          .then(function (result) {
            var error = result.error;
            if (error) {
              if (error.type === "validation_error") {
                showMessage(error.message);
              } else {
                showMessage("An unexpected error occurred.");
              }
            } else {
              // This point will only be reached if there is an immediate error occurring while confirming the payment. Otherwise, your customer will be redirected to your 'return_url'.
              // For some payment flows such as Sofort, iDEAL, your customer will be redirected to an intermediate page to complete authorization of the payment, and then redirected to the 'return_url'.
              hyper
                .retrievePaymentIntent(paymentDetails.client_secret)
                .then(function (result) {
                  var paymentIntent = result.paymentIntent;
                  if (paymentIntent && paymentIntent.status) {
                    hide("#hyper-checkout-sdk");
                    hide("#hyper-checkout-details");
                    show("#hyper-checkout-status-canvas");
                    showStatus(
                      paymentDetails.amount,
                      Object.assign(paymentDetails, paymentIntent)
                    );
                  }
                })
                .catch(function (error) {
                  console.error("Error retrieving payment_intent", error);
                });
            }
          })
          .catch(function (error) {
            console.error("Error confirming payment_intent", error);
          })
          .finally(() => {
            hide("#submit-spinner");
            show("#submit-button-text");
            submitButtonNode.disabled = false;
            submitButtonNode.classList.remove("disabled");
          });
      }

      // Fetches the payment status after payment submission
      function checkStatus(paymentDetails) {
        return new window.Promise(function (resolve, reject) {
          var res = {
            showSdk: true,
          };

          var clientSecret = new URLSearchParams(window.location.search).get(
            "payment_intent_client_secret"
          );

          // If clientSecret is not found in URL params, try to fetch from window context
          if (!clientSecret) {
            clientSecret = paymentDetails.client_secret;
          }

          // If clientSecret is not present, show status
          if (!clientSecret) {
            res.showSdk = false;
            showStatus(
              paymentDetails.amount,
              Object.assign(paymentDetails, {
                status: "",
                error: {
                  code: "NO_CLIENT_SECRET",
                  message: "client_secret not found",
                },
              })
            );
            return resolve(res);
          }
          hyper
            .retrievePaymentIntent(clientSecret)
            .then(function (response) {
              var paymentIntent = response.paymentIntent;
              // If paymentIntent was not found, show status
              if (!paymentIntent) {
                res.showSdk = false;
                showStatus(
                  paymentDetails.amount,
                  Object.assign(paymentDetails, {
                    status: "",
                    error: {
                      code: "NOT_FOUND",
                      message: "PaymentIntent was not found",
                    },
                  })
                );
                return resolve(res);
              }
              // Show SDK only if paymentIntent status has not been initiated
              switch (paymentIntent.status) {
                case "requires_confirmation":
                case "requires_payment_method":
                  return resolve(res);
              }
              showStatus(
                paymentDetails.amount,
                Object.assign(paymentDetails, paymentIntent)
              );
              res.showSdk = false;
              resolve(res);
            })
            .catch(function (error) {
              reject(error);
            });
        });
      }

      function show(id) {
        removeClass(id, "hidden");
      }
      function hide(id) {
        addClass(id, "hidden");
      }

      function showMessage(msg) {
        show("#payment-message");
        addText("#payment-message", msg);
      }

      function showStatus(amount, paymentDetails) {
        var status = paymentDetails.status;
        var statusDetails = {
          imageSource: "",
          message: null,
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
          case "succeeded":
            statusDetails.imageSource = "https://i.imgur.com/5BOmYVl.png";
            statusDetails.message =
              "We have successfully received your payment";
            statusDetails.status = "Paid successfully";
            statusDetails.amountText = new Date(
              paymentDetails.created
            ).toTimeString();
            break;

          case "processing":
            statusDetails.imageSource = "https://i.imgur.com/Yb79Qt4.png";
            statusDetails.message =
              "Sorry! Your payment is taking longer than expected. Please check back again in sometime.";
            statusDetails.status = "Payment Pending";
            break;

          case "failed":
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

          case "cancelled":
            statusDetails.imageSource = "https://i.imgur.com/UD8CEuY.png";
            statusDetails.status = "Payment Cancelled";
            break;

          case "requires_merchant_action":
            statusDetails.imageSource = "https://i.imgur.com/Yb79Qt4.png";
            statusDetails.status = "Payment under review";
            break;

          case "requires_capture":
            statusDetails.imageSource = "https://i.imgur.com/Yb79Qt4.png";
            statusDetails.status = "Payment Pending";
            break;

          case "partially_captured":
            statusDetails.imageSource = "https://i.imgur.com/Yb79Qt4.png";
            statusDetails.message = "Partial payment was captured.";
            statusDetails.status = "Partial Payment Pending";
            break;

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
            break;
        }

        // Form header items
        var amountNode = document.createElement("div");
        amountNode.className = "hyper-checkout-status-amount";
        amountNode.innerText = paymentDetails.currency + " " + amount;
        var merchantLogoNode = document.createElement("img");
        merchantLogoNode.className = "hyper-checkout-status-merchant-logo";
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
          if (statusDetails.message instanceof HTMLDivElement) {
            statusContentNode.append(statusMessageNode);
          }
          statusContentNode.append(statusDetailsNode);
        }

        // Form redirect text
        var statusRedirectTextNode = document.getElementById(
          "hyper-checkout-status-redirect-message"
        );
        if (
          statusRedirectTextNode &&
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
        valueNode.className = "hyper-checkout-item-value";
        valueNode.innerText = value;
        itemNode.append(headerNode);
        itemNode.append(valueNode);
        return itemNode;
      }

      function addText(id, msg) {
        var element = document.querySelector(id);
        element.innerText = msg;
      }

      function addClass(id, className) {
        var element = document.querySelector(id);
        element.classList.add(className);
      }

      function removeClass(id, className) {
        var element = document.querySelector(id);
        element.classList.remove(className);
      }

      function formatDate(date) {
        var months = [
          "January",
          "February",
          "March",
          "April",
          "May",
          "June",
          "July",
          "August",
          "September",
          "October",
          "November",
          "December",
        ];

        var hours = date.getHours();
        var minutes = date.getMinutes();
        minutes = minutes < 10 ? "0" + minutes : minutes;
        var suffix = hours > 11 ? "PM" : "AM";
        hours = hours % 12;
        hours = hours ? hours : 12;
        var day = date.getDate();
        var month = months[date.getMonth()];
        var year = date.getUTCFullYear();

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

      function renderPaymentDetails(paymentDetails) {
        // Create price node
        var priceNode = document.createElement("div");
        priceNode.className = "hyper-checkout-payment-price";
        priceNode.innerText =
          paymentDetails.currency + " " + paymentDetails.amount;

        // Create merchant name's node
        var merchantNameNode = document.createElement("div");
        merchantNameNode.className = "hyper-checkout-payment-merchant-name";
        merchantNameNode.innerText =
          "Requested by " + paymentDetails.merchant_name;

        // Create payment ID node
        var paymentIdNode = document.createElement("div");
        paymentIdNode.className = "hyper-checkout-payment-ref";
        paymentIdNode.innerText = "Ref Id: " + paymentDetails.payment_id;

        // Create merchant logo's node
        var merchantLogoNode = document.createElement("img");
        merchantLogoNode.src = paymentDetails.merchant_logo;

        // Create expiry node
        var paymentExpiryNode = document.createElement("div");
        paymentExpiryNode.className = "hyper-checkout-payment-footer-expiry";
        var expiryDate = new Date(paymentDetails.session_expiry);
        var formattedDate = formatDate(expiryDate);
        paymentExpiryNode.innerText = "Link expires on: " + formattedDate;

        // Append information to DOM
        var paymentContextNode = document.getElementById(
          "hyper-checkout-payment-context"
        );
        paymentContextNode.prepend(priceNode);
        var paymentMerchantDetails = document.getElementById(
          "hyper-checkout-payment-merchant-details"
        );
        paymentMerchantDetails.append(merchantNameNode);
        paymentMerchantDetails.append(paymentIdNode);
        var merchantImageNode = document.getElementById(
          "hyper-checkout-merchant-image"
        );
        merchantImageNode.prepend(merchantLogoNode);
        var footerNode = document.getElementById(
          "hyper-checkout-payment-footer"
        );
        footerNode.append(paymentExpiryNode);
      }

      function renderCart(paymentDetails) {
        var orderDetails = paymentDetails.order_details;

        // Cart items
        if (Array.isArray(orderDetails) && orderDetails.length > 0) {
          var cartNode = document.getElementById("hyper-checkout-cart");
          var cartItemsNode = document.getElementById(
            "hyper-checkout-cart-items"
          );
          var MAX_ITEMS_VISIBLE_AFTER_COLLAPSE =
            paymentDetails.max_items_visible_after_collapse;

          orderDetails.map(function (item, index) {
            if (index >= MAX_ITEMS_VISIBLE_AFTER_COLLAPSE) {
              return;
            }
            renderCartItem(
              item,
              paymentDetails,
              index !== 0 && index < MAX_ITEMS_VISIBLE_AFTER_COLLAPSE,
              cartItemsNode
            );
          });
          // Expand / collapse button
          var totalItems = orderDetails.length;
          if (totalItems > MAX_ITEMS_VISIBLE_AFTER_COLLAPSE) {
            var expandButtonNode = document.createElement("div");
            expandButtonNode.className = "hyper-checkout-cart-button";
            expandButtonNode.onclick = () => {
              handleCartView(paymentDetails);
            };
            var buttonImageNode = document.createElement("svg");
            buttonImageNode.id = "hyper-checkout-cart-button-arrow";
            buttonImageNode.innerHTML =
              document.getElementById("arrow-down").innerHTML;
            var buttonTextNode = document.createElement("span");
            buttonTextNode.id = "hyper-checkout-cart-button-text";
            var hiddenItemsCount =
              orderDetails.length - MAX_ITEMS_VISIBLE_AFTER_COLLAPSE;
            buttonTextNode.innerText = "Show More (" + hiddenItemsCount + ")";
            expandButtonNode.append(buttonTextNode, buttonImageNode);
            cartNode.insertBefore(expandButtonNode, cartNode.lastElementChild);
          }
        } else {
          hide("#hyper-checkout-cart-header");
          hide("#hyper-checkout-cart-items");
          hide("#hyper-checkout-cart-image");
          if (
            typeof paymentDetails.merchant_description === "string" &&
            paymentDetails.merchant_description.length > 0
          ) {
            show("#hyper-checkout-merchant-description");
            var merchantDescription = paymentDetails.merchant_description;
            document.getElementById(
              "hyper-checkout-merchant-description"
            ).innerText = merchantDescription;
          }
        }
      }

      function renderCartItem(
        item,
        paymentDetails,
        shouldAddDividerNode,
        cartItemsNode
      ) {
        // Wrappers
        var itemWrapperNode = document.createElement("div");
        itemWrapperNode.className = "hyper-checkout-cart-item";
        var nameAndQuantityWrapperNode = document.createElement("div");
        nameAndQuantityWrapperNode.className =
          "hyper-checkout-cart-product-details";
        // Image
        var productImageNode = document.createElement("img");
        productImageNode.className = "hyper-checkout-cart-product-image";
        productImageNode.src = item.product_img_link;
        // Product title
        var productNameNode = document.createElement("div");
        productNameNode.className = "hyper-checkout-card-item-name";
        productNameNode.innerText = item.product_name;
        // Product quantity
        var quantityNode = document.createElement("div");
        quantityNode.className = "hyper-checkout-card-item-quantity";
        quantityNode.innerText = "Qty: " + item.quantity;
        // Product price
        var priceNode = document.createElement("div");
        priceNode.className = "hyper-checkout-card-item-price";
        priceNode.innerText = paymentDetails.currency + " " + item.amount;
        // Append items
        nameAndQuantityWrapperNode.append(productNameNode, quantityNode);
        itemWrapperNode.append(
          productImageNode,
          nameAndQuantityWrapperNode,
          priceNode
        );
        if (shouldAddDividerNode) {
          var dividerNode = document.createElement("div");
          dividerNode.className = "hyper-checkout-cart-item-divider";
          cartItemsNode.append(dividerNode);
        }
        cartItemsNode.append(itemWrapperNode);
      }

      function handleCartView(paymentDetails) {
        var orderDetails = paymentDetails.order_details;
        var MAX_ITEMS_VISIBLE_AFTER_COLLAPSE =
          paymentDetails.max_items_visible_after_collapse;
        var itemsHTMLCollection = document.getElementsByClassName(
          "hyper-checkout-cart-item"
        );
        var dividerHTMLCollection = document.getElementsByClassName(
          "hyper-checkout-cart-item-divider"
        );
        var cartItems = [].slice.call(itemsHTMLCollection);
        var dividerItems = [].slice.call(dividerHTMLCollection);
        var isHidden = cartItems.length < orderDetails.length;
        var cartItemsNode = document.getElementById(
          "hyper-checkout-cart-items"
        );
        var cartButtonTextNode = document.getElementById(
          "hyper-checkout-cart-button-text"
        );
        var cartButtonImageNode = document.getElementById(
          "hyper-checkout-cart-button-arrow"
        );
        if (isHidden) {
          if (Array.isArray(orderDetails)) {
            orderDetails.map(function (item, index) {
              if (index < MAX_ITEMS_VISIBLE_AFTER_COLLAPSE) {
                return;
              }
              renderCartItem(
                item,
                paymentDetails,
                index >= MAX_ITEMS_VISIBLE_AFTER_COLLAPSE,
                cartItemsNode
              );
            });
          }
          cartItemsNode.style.maxHeight = cartItemsNode.scrollHeight + "px";
          cartItemsNode.style.height = cartItemsNode.scrollHeight + "px";
          cartButtonTextNode.innerText = "Show Less";
          cartButtonImageNode.innerHTML =
            document.getElementById("arrow-up").innerHTML;
        } else {
          cartItemsNode.style.maxHeight = "300px";
          cartItemsNode.style.height = "290px";
          cartItemsNode.scrollTo({ top: 0, behavior: "smooth" });
          setTimeout(function () {
            cartItems.map(function (item, index) {
              if (index < MAX_ITEMS_VISIBLE_AFTER_COLLAPSE) {
                return;
              }
              cartItemsNode.removeChild(item);
            });
            dividerItems.map(function (item, index) {
              if (index < MAX_ITEMS_VISIBLE_AFTER_COLLAPSE - 1) {
                return;
              }
              cartItemsNode.removeChild(item);
            });
          }, 300);
          setTimeout(function () {
            var hiddenItemsCount =
              orderDetails.length - MAX_ITEMS_VISIBLE_AFTER_COLLAPSE;
            cartButtonTextNode.innerText =
              "Show More (" + hiddenItemsCount + ")";
            cartButtonImageNode.innerHTML =
              document.getElementById("arrow-down").innerHTML;
          }, 250);
        }
      }

      function hideCartInMobileView() {
        window.history.back();
        var cartNode = document.getElementById("hyper-checkout-cart");
        cartNode.style.animation = "slide-to-right 0.3s linear";
        cartNode.style.right = "-582px";
        setTimeout(function () {
          hide("#hyper-checkout-cart");
        }, 300);
      }

      function viewCartInMobileView() {
        window.history.pushState("view-cart", "");
        var cartNode = document.getElementById("hyper-checkout-cart");
        cartNode.style.animation = "slide-from-right 0.3s linear";
        cartNode.style.right = "0px";
        show("#hyper-checkout-cart");
      }

      function renderSDKHeader(paymentDetails) {
        // SDK headers' items
        var sdkHeaderItemNode = document.createElement("div");
        sdkHeaderItemNode.className = "hyper-checkout-sdk-items";
        var sdkHeaderMerchantNameNode = document.createElement("div");
        sdkHeaderMerchantNameNode.className =
          "hyper-checkout-sdk-header-brand-name";
        sdkHeaderMerchantNameNode.innerText = paymentDetails.merchant_name;
        var sdkHeaderAmountNode = document.createElement("div");
        sdkHeaderAmountNode.className = "hyper-checkout-sdk-header-amount";
        sdkHeaderAmountNode.innerText =
          paymentDetails.currency + " " + paymentDetails.amount;
        sdkHeaderItemNode.append(sdkHeaderMerchantNameNode);
        sdkHeaderItemNode.append(sdkHeaderAmountNode);

        // Append to SDK header's node
        var sdkHeaderNode = document.getElementById(
          "hyper-checkout-sdk-header"
        );
        if (sdkHeaderNode instanceof HTMLDivElement) {
          sdkHeaderNode.append(sdkHeaderLogoNode);
          sdkHeaderNode.append(sdkHeaderItemNode);
        }
      }

      function adjustLightness(hexColor, factor) {
        // Convert hex to RGB
        var r = parseInt(hexColor.slice(1, 3), 16);
        var g = parseInt(hexColor.slice(3, 5), 16);
        var b = parseInt(hexColor.slice(5, 7), 16);

        // Convert RGB to HSL
        var hsl = rgbToHsl(r, g, b);

        // Adjust lightness
        hsl[2] = Math.max(0, Math.min(100, hsl[2] * factor));

        // Convert HSL back to RGB
        var rgb = hslToRgb(hsl[0], hsl[1], hsl[2]);

        // Convert RGB to hex
        var newHexColor = rgbToHex(rgb[0], rgb[1], rgb[2]);

        return newHexColor;
      }
      function rgbToHsl(r, g, b) {
        r /= 255;
        g /= 255;
        b /= 255;
        var max = Math.max(r, g, b),
          min = Math.min(r, g, b);
        var h,
          s,
          l = (max + min) / 2;

        if (max === min) {
          h = s = 0;
        } else {
          var d = max - min;
          s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
          switch (max) {
            case r:
              h = (g - b) / d + (g < b ? 6 : 0);
              break;
            case g:
              h = (b - r) / d + 2;
              break;
            case b:
              h = (r - g) / d + 4;
              break;
          }
          h /= 6;
        }

        return [h * 360, s * 100, l * 100];
      }
      function hslToRgb(h, s, l) {
        h /= 360;
        s /= 100;
        l /= 100;
        var r, g, b;

        if (s === 0) {
          r = g = b = l;
        } else {
          var hue2rgb = function (p, q, t) {
            if (t < 0) t += 1;
            if (t > 1) t -= 1;
            if (t < 1 / 6) return p + (q - p) * 6 * t;
            if (t < 1 / 2) return q;
            if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
            return p;
          };

          var q = l < 0.5 ? l * (1 + s) : l + s - l * s;
          var p = 2 * l - q;

          r = hue2rgb(p, q, h + 1 / 3);
          g = hue2rgb(p, q, h);
          b = hue2rgb(p, q, h - 1 / 3);
        }

        return [r * 255, g * 255, b * 255];
      }
      function rgbToHex(r, g, b) {
        var toHex = function (c) {
          var hex = Math.round(c).toString(16);
          return hex.length === 1 ? "0" + hex : hex;
        };
        return "#" + toHex(r) + toHex(g) + toHex(b);
      }

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
        if (!RE_HEX.test(hex))
          throw new Error('Invalid HEX color: "' + hex + '"');
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
        var options =
          bw === true ? DEFAULT_BW : Object.assign({}, DEFAULT_BW, bw);
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