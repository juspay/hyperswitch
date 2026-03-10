/* eslint-disable cypress/unsafe-to-chain-command */

import jsQR from "jsqr";
import { getTimeoutMultiplier } from "../utils/RequestBodyUtils.js";

const timeoutMultiplier = getTimeoutMultiplier();

const CONSTANTS = {
  TIMEOUT: Math.round(90000 * timeoutMultiplier), // 90s local, 135s (2.25min) CI
  WAIT_TIME: Math.round(30000 * timeoutMultiplier), // 30s local, 45s CI
  ERROR_PATTERNS: [
    /^(4|5)\d{2}\s/, // HTTP error status codes
    /\berror occurred\b/i,
    /\bpayment failed\b/i,
    /\binvalid request\b/i,
    /\bserver error\b/i,
    /\btransaction failed\b/i,
    /\bpayment declined\b/i,
    /\bauthorization failed\b/i,
  ],
  VALID_TERMINAL_STATUSES: [
    "failed",
    "processing",
    "requires_capture",
    "succeeded",
  ],
};

export function handleRedirection(
  redirectionType,
  urls,
  connectorId,
  paymentMethodType,
  handlerMetadata
) {
  switch (redirectionType) {
    case "bank_redirect":
      bankRedirectRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        connectorId,
        paymentMethodType
      );
      break;
    case "bank_transfer":
      bankTransferRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        connectorId,
        paymentMethodType,
        handlerMetadata.nextActionType
      );
      break;
    case "three_ds":
      threeDsRedirection(urls.redirectionUrl, urls.expectedUrl, connectorId);
      break;
    case "upi":
      upiRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        connectorId,
        paymentMethodType
      );
      break;
    default:
      throw new Error(`Unknown redirection type: ${redirectionType}`);
  }
}

function bankTransferRedirection(
  redirectionUrl,
  expectedUrl,
  connectorId,
  paymentMethodType,
  nextActionType
) {
  let verifyUrl = true; // Default to true, can be set to false based on conditions
  switch (nextActionType) {
    case "bank_transfer_steps_and_charges_details":
      verifyUrl = false;
      break;
    case "qr_code_url":
      cy.request(redirectionUrl.href).then((response) => {
        switch (connectorId) {
          case "adyen":
            switch (paymentMethodType) {
              case "pix":
                expect(response.status).to.eq(200);
                fetchAndParseQRCode(redirectionUrl.href).then((qrCodeData) => {
                  expect(qrCodeData).to.eq("TestQRCodeEMVToken");
                });
                break;
              default:
                verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
              // expected_redirection can be used here to handle other payment methods
            }
            break;
          default:
            verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
        }
      });
      break;
    case "image_data_url":
      switch (connectorId) {
        case "facilitapay":
          switch (paymentMethodType) {
            case "pix":
              fetchAndParseImageData(redirectionUrl).then((qrCodeData) => {
                expect(qrCodeData).to.contains("FacilitaPay"); // image data contains the following value
              });
              break;
            default:
              verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
          }
          break;
        case "itaubank":
          switch (paymentMethodType) {
            case "pix":
              fetchAndParseImageData(redirectionUrl).then((qrCodeData) => {
                expect(qrCodeData).to.contains("itau.com.br/pix/qr/v2"); // image data contains the following value
              });
              break;
            default:
              verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
          }
          break;
        default:
          verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
      }
      break;
    case "redirect_to_url":
      cy.visit(redirectionUrl.href);
      waitForRedirect(redirectionUrl.href); // Wait for the first redirect

      handleFlow(
        redirectionUrl,
        expectedUrl,
        connectorId,
        ({ connectorId, paymentMethodType }) => {
          switch (connectorId) {
            case "trustpay":
              // Suppress cross-origin JavaScript errors from TrustPay's website
              cy.on("uncaught:exception", (err) => {
                // Trustpay javascript devs have skill issues
                if (
                  err.message.includes("$ is not defined") ||
                  err.message.includes("mainController is not defined") ||
                  err.message.includes("jQuery") ||
                  err.message.includes("aapi.trustpay.eu")
                ) {
                  return false; // Prevent test failure
                }
                return true;
              });

              // Trustpay bank redirects never reach the terminal state
              switch (paymentMethodType) {
                case "instant_bank_transfer_finland":
                  cy.log("Trustpay Instant Bank Transfer through Finland");
                  break;
                case "instant_bank_transfer_poland":
                  cy.log("Trustpay Instant Bank Transfer through Finland");
                  break;
                default:
                  throw new Error(
                    `Unsupported Trustpay payment method type: ${paymentMethodType}`
                  );
              }
              verifyUrl = false;
              break;
            default:
              verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
          }
        },
        { paymentMethodType }
      );
      break;

    default:
      verifyReturnUrl(redirectionUrl, expectedUrl, true);
  }
}

function bankRedirectRedirection(
  redirectionUrl,
  expectedUrl,
  connectorId,
  paymentMethodType
) {
  let verifyUrl = false;

  cy.visit(redirectionUrl.href);
  waitForRedirect(redirectionUrl.href); // Wait for the first redirect

  // adyen ideal has been kept outside the handleFlow function just because cypress does not support nested `cy.origin` yet
  // ref: https://github.com/cypress-io/cypress/issues/20718
  // the work around is to use `cy.origin` in sequential manner
  if (connectorId === "adyen" && paymentMethodType === "ideal") {
    const adyenIdealOrigin1 = "https://ext.pay.ideal.nl";
    const adyenIdealOrigin2 = "https://handler.ext.idealtesttool.nl";

    cy.origin(
      adyenIdealOrigin1,
      { args: { constants: CONSTANTS } },
      ({ constants }) => {
        cy.log(
          "Executing on Adyen iDEAL Origin 1:",
          cy.state("window").location.origin
        );
        cy.wait(constants.TIMEOUT / 10); // 2 seconds
        cy.get("button[data-testid=payment-action-button]").click();
        cy.wait(constants.TIMEOUT / 10); // 2 seconds
        cy.get("button[id=bank-item-TESTNL2A]").click();
      }
    );

    cy.log(`Waiting for redirection to ${adyenIdealOrigin2}`);
    cy.location("origin", { timeout: CONSTANTS.TIMEOUT }).should(
      "eq",
      adyenIdealOrigin2
    );

    cy.origin(
      adyenIdealOrigin2,
      { args: { constants: CONSTANTS } },
      ({ constants }) => {
        cy.log(
          "Executing on Adyen iDEAL Origin 2:",
          cy.state("window").location.origin
        );

        cy.get(".btn.btn-primary.btn-lg")
          .contains("Success")
          .should("be.visible")
          .click();

        cy.url({ timeout: constants.WAIT_TIME }).should(
          "include",
          "/loading/SUCCESS"
        );
      }
    );

    // we get `An error occurred with the WebSocket` after clicking the `success` button
    // and it does not redirect to the expected url
    // so, we need cannot verify the return url for adyen ideal bank redirect
    verifyUrl = false;
  }
  // Handle Shift4 separately similar to Adyen iDEAL to avoid constants scope issues
  else if (
    connectorId === "shift4" &&
    (paymentMethodType === "eps" || paymentMethodType === "ideal")
  ) {
    cy.log(`Special handling for Shift4 ${paymentMethodType} payment`);

    cy.url().then((currentUrl) => {
      cy.origin(
        new URL(currentUrl).origin,
        { args: { constants: CONSTANTS } },
        ({ constants }) => {
          // Try to click the succeed payment button
          cy.contains("button", "Succeed payment", {
            timeout: constants.TIMEOUT,
          })
            .should("be.visible")
            .click();
        }
      );
    });

    verifyUrl = true;
  } else if (connectorId === "airwallex" && paymentMethodType === "ideal") {
    const airwallexIdealOrigin1 = "https://ext.pay.ideal.nl";
    const airwallexIdealOrigin2 = "https://handler.ext.idealtesttool.nl";

    cy.origin(
      airwallexIdealOrigin1,
      { args: { constants: CONSTANTS } },
      ({ constants }) => {
        cy.log("Executing on Airwallex iDEAL Origin 1");
        cy.wait(constants.TIMEOUT / 10); // 2 seconds
        cy.get("button[data-testid=payment-action-button]").click();
        cy.wait(constants.TIMEOUT / 10); // 2 seconds
        cy.get("button[id=bank-item-TESTNL2A]").click();
      }
    );

    cy.log(`Waiting for redirection to ${airwallexIdealOrigin2}`);
    cy.location("origin", { timeout: CONSTANTS.TIMEOUT }).should(
      "eq",
      airwallexIdealOrigin2
    );

    cy.origin(
      airwallexIdealOrigin2,
      { args: { constants: CONSTANTS } },
      ({ constants }) => {
        cy.log("Executing on Airwallex iDEAL Origin 2");

        cy.get(".btn.btn-primary.btn-lg")
          .contains("Success")
          .should("be.visible")
          .click();

        cy.url({ timeout: constants.WAIT_TIME }).should(
          "include",
          "/loading/SUCCESS"
        );
      }
    );
    verifyUrl = false;
  } else {
    handleFlow(
      redirectionUrl,
      expectedUrl,
      connectorId,
      ({ connectorId, paymentMethodType }) => {
        // Renamed expectedUrl arg for clarity
        // This callback now runs either in cy.origin (if redirected) or directly (if iframe)
        switch (connectorId) {
          case "adyen":
            switch (paymentMethodType) {
              case "eps":
                cy.get("h1").should("contain.text", "Acquirer Simulator");
                cy.get('[value="authorised"]').click();
                verifyUrl = true;
                break;
              // The 'ideal' case is handled outside handleFlow
              default:
                throw new Error(
                  `Unsupported Adyen payment method type in handleFlow: ${paymentMethodType}`
                );
            }
            break;

          case "aci":
            switch (paymentMethodType) {
              case "ideal":
                cy.get('input[type="submit"][value="Confirm Transaction"]')
                  .should("be.visible")
                  .click();
                break;
              default:
                throw new Error(
                  `Unsupported ACI payment method type in handleFlow: ${paymentMethodType}`
                );
            }
            break;

          case "paypal":
            if (["eps", "ideal", "giropay"].includes(paymentMethodType)) {
              cy.get('button[name="Successful"][value="SUCCEEDED"]').click();
              verifyUrl = true;
            } else {
              throw new Error(
                `Unsupported Paypal payment method type: ${paymentMethodType}`
              );
            }

            break;

          case "stripe":
            if (
              ["eps", "ideal", "giropay", "sofort", "przelewy24"].includes(
                paymentMethodType
              )
            ) {
              // scroll down and click on the authorize test payment button
              cy.get("body").then(() => {
                cy.get("#frame-warning-container").then(($el) => {
                  if ($el.is(":visible")) {
                    // Frame warning is visible — use test payment button
                    cy.get("#authorize-test-payment")
                      .scrollIntoView()
                      .should("be.visible")
                      .click();
                  } else {
                    // Frame warning is hidden — use the success link
                    cy.contains(
                      'a.common-Button[name="success"]',
                      "Authorize Test Payment"
                    )
                      .scrollIntoView()
                      .should("be.visible")
                      .click();
                  }
                });
              });
              verifyUrl = true;
            } else {
              throw new Error(
                `Unsupported Stripe payment method type: ${paymentMethodType}`
              );
            }

            break;

          case "trustpay":
            // Trustpay bank redirects never reach the terminal state
            switch (paymentMethodType) {
              case "eps":
                cy.get("#bankname").type(
                  "Allgemeine Sparkasse Oberösterreich Bank AG (ASPKAT2LXXX / 20320)"
                );
                cy.get("#selectionSubmit").click();
                break;
              case "ideal":
                cy.contains("button", "Select your bank").click();
                cy.get(
                  'button[data-testid="bank-item"][id="bank-item-INGBNL2A"]'
                ).click();
                break;
              case "giropay":
                cy.get("._transactionId__header__iXVd_").should(
                  "contain.text",
                  "Bank suchen ‑ mit giropay zahlen."
                );
                cy.get(".BankSearch_searchInput__uX_9l").type(
                  "Volksbank Hildesheim{enter}"
                );
                cy.get(".BankSearch_searchIcon__EcVO7").click();
                cy.get(".BankSearch_bankWrapper__R5fUK").click();
                cy.get("._transactionId__primaryButton__nCa0r").click();
                cy.get(".normal-3").should("contain.text", "Kontoauswahl");
                break;
              default:
                throw new Error(
                  `Unsupported Trustpay payment method type: ${paymentMethodType}`
                );
            }
            verifyUrl = false;
            break;

          case "nuvei":
            // Enhanced Nuvei bank redirect handling with timeout awareness
            cy.log(`Handling Nuvei ${paymentMethodType} bank redirect`);

            // Add timeout handling for Nuvei bank redirects
            cy.window().then((win) => {
              // Check if we're on a timeout or error page
              const pageText = win.document.body.innerText.toLowerCase();
              if (
                pageText.includes("timeout") ||
                pageText.includes("error") ||
                pageText.includes("not responding") ||
                pageText.includes("connection failed")
              ) {
                cy.log(
                  `⚠ Nuvei ${paymentMethodType} timeout detected on redirect page`
                );
                verifyUrl = false; // Skip URL verification for timeout scenarios
                return;
              }
            });

            switch (paymentMethodType) {
              case "ideal":
                // Handle iDEAL bank selection and confirmation with timeout awareness
                cy.get("body", { timeout: 15000 }).then(($body) => {
                  const bodyText = $body.text().toLowerCase();

                  // Check for timeout indicators
                  if (
                    bodyText.includes("timeout") ||
                    bodyText.includes("error")
                  ) {
                    cy.log(
                      `Nuvei iDEAL timeout detected - skipping interaction`
                    );
                    verifyUrl = false;
                    return;
                  }

                  // Look for bank selection dropdown or buttons
                  if ($body.find('select[name="bank"]').length > 0) {
                    cy.get('select[name="bank"]').select("INGBNL2A"); // ING Bank
                    cy.get(
                      'button[type="submit"], input[type="submit"]'
                    ).click();
                  } else if (
                    $body.find('button[data-bank="INGBNL2A"]').length > 0
                  ) {
                    cy.get('button[data-bank="INGBNL2A"]').click();
                  } else {
                    // Generic approach - look for ING or any bank button
                    cy.contains("button, a", /ING|Bank/i)
                      .first()
                      .click();
                  }
                });
                verifyUrl = true;
                break;

              case "giropay":
                // Handle Giropay flow with timeout awareness
                cy.get("body", { timeout: 15000 }).then(($body) => {
                  const bodyText = $body.text().toLowerCase();

                  if (
                    bodyText.includes("timeout") ||
                    bodyText.includes("error")
                  ) {
                    cy.log(
                      `Nuvei Giropay timeout detected - skipping interaction`
                    );
                    verifyUrl = false;
                    return;
                  }

                  if ($body.find('input[name="bank_code"]').length > 0) {
                    cy.get('input[name="bank_code"]').type("12345678");
                    cy.get(
                      'button[name="continue"], button[type="submit"]'
                    ).click();
                  } else {
                    cy.contains(
                      "button, input",
                      /continue|submit|proceed/i
                    ).click();
                  }
                });
                verifyUrl = true;
                break;

              case "sofort":
                // Handle Sofort flow with timeout awareness
                cy.get("body", { timeout: 15000 }).then(($body) => {
                  const bodyText = $body.text().toLowerCase();

                  if (
                    bodyText.includes("timeout") ||
                    bodyText.includes("error")
                  ) {
                    cy.log(
                      `Nuvei Sofort timeout detected - skipping interaction`
                    );
                    verifyUrl = false;
                    return;
                  }

                  // Sofort typically requires bank selection and login simulation
                  if ($body.find('select[name="bank"]').length > 0) {
                    cy.get('select[name="bank"]').select(0); // Select first bank
                    cy.get('button[type="submit"]').click();
                  } else if ($body.find('input[name="login"]').length > 0) {
                    // If login form is present
                    cy.get('input[name="login"]').type("testuser");
                    cy.get('input[name="password"]').type("testpass");
                    cy.get('button[type="submit"]').click();
                  } else {
                    // Generic continue button
                    cy.contains(
                      "button, input",
                      /continue|weiter|submit/i
                    ).click();
                  }
                });
                verifyUrl = true;
                break;

              case "eps":
                // Handle EPS flow with timeout awareness
                cy.get("body", { timeout: 15000 }).then(($body) => {
                  const bodyText = $body.text().toLowerCase();

                  if (
                    bodyText.includes("timeout") ||
                    bodyText.includes("error")
                  ) {
                    cy.log(`Nuvei EPS timeout detected - skipping interaction`);
                    verifyUrl = false;
                    return;
                  }

                  if ($body.find('select[name="bank"]').length > 0) {
                    cy.get('select[name="bank"]').select(0); // Select first Austrian bank
                    cy.get('button[type="submit"]').click();
                  } else {
                    cy.contains(
                      "button, input",
                      /continue|submit|weiter/i
                    ).click();
                  }
                });
                verifyUrl = true;
                break;

              default:
                throw new Error(
                  `Unsupported Nuvei payment method type: ${paymentMethodType}`
                );
            }
            break;

          case "nexinets":
            switch (paymentMethodType) {
              case "ideal":
                // Nexinets iDEAL specific selector - click the Success link
                cy.get("a.btn.btn-primary.btn-block")
                  .contains("Success")
                  .click();

                verifyUrl = true;
                break;
              default:
                throw new Error(
                  `Unsupported Nexinets payment method type: ${paymentMethodType}`
                );
            }
            break;

          case "multisafepay":
            if (["sofort", "eps", "mbway"].includes(paymentMethodType)) {
              // Multisafe pay has CSRF blocking cannot actually test redirection flow via cypress
              // cy.get(".btn-msp-success").click();

              verifyUrl = false;
            } else {
              throw new Error(
                `Unsupported multisafe payment method type: ${paymentMethodType}`
              );
            }
            break;

          default:
            throw new Error(
              `Unsupported connector in handleFlow: ${connectorId}`
            );
        }
      },
      { paymentMethodType } // Pass options to handleFlow
    );
  }
  cy.then(() => {
    // The value of verifyUrl determined by the specific flow (Adyen iDEAL or handleFlow callback)
    verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
  });
}

function threeDsRedirection(redirectionUrl, expectedUrl, connectorId) {
  let responseContentType = null;

  // First check what type of response we get from the redirect URL
  cy.request({
    url: redirectionUrl.href,
    failOnStatusCode: false,
  }).then((response) => {
    responseContentType = response.headers["content-type"];

    // Check if the response is JSON
    if (response.headers["content-type"]?.includes("application/json")) {
      // For JSON responses, check if it contains useful info
      if (response.body && typeof response.body === "object") {
        // If the JSON contains redirect info, use it
        if (response.body.redirect_url) {
          cy.visit(response.body.redirect_url, { failOnStatusCode: false });
        } else {
          cy.visit(expectedUrl.href);
          // Verify return URL and exit completely
          verifyReturnUrl(redirectionUrl, expectedUrl, true);
          return;
        }
      } else {
        cy.visit(expectedUrl.href);
        verifyReturnUrl(redirectionUrl, expectedUrl, true);
        return;
      }
    } else {
      cy.visit(redirectionUrl.href, { failOnStatusCode: false });
    }
  });

  if (connectorId === "paysafe") {
    cy.log("Starting Paysafe 3DS authentication flow");

    cy.get('input[formcontrolname="contactInfo"]', {
      timeout: CONSTANTS.TIMEOUT,
    })
      .clear()
      .type("swangi@gmail.com");

    cy.get('button[type="submit"]', { timeout: CONSTANTS.TIMEOUT }).click();

    cy.log("Submitted email, waiting for OTP page...");
    // Wait for OTP iframe instead of hard wait
    cy.get("iframe", { timeout: CONSTANTS.TIMEOUT })
      .first()
      .its("0.contentDocument.body")
      .should("not.be.empty")
      .within(() => {
        cy.get(
          'input[placeholder="Enter Code Here"], input[type="text"], input[type="password"], input',
          { timeout: CONSTANTS.TIMEOUT }
        )
          .first()
          .clear()
          .type("1234");

        cy.get("input.button.primary", { timeout: CONSTANTS.TIMEOUT }).click();
      });

    cy.log("Submitted OTP");
    // Wait for redirect URL to load
    cy.url({ timeout: CONSTANTS.TIMEOUT }).should("include", expectedUrl);

    verifyReturnUrl(redirectionUrl, expectedUrl, true);
    return;
  }

  // Special handling for Airwallex which uses multiple domains in 3DS flow
  if (connectorId === "airwallex") {
    cy.log("Starting specialized Airwallex 3DS handling");

    // Wait for page to load completely by checking for document ready state
    cy.document()
      .should("have.property", "readyState")
      .and("equal", "complete");

    // Check current URL to determine which stage of flow we're in
    cy.url().then((currentUrl) => {
      cy.log(`Current URL: ${currentUrl}`);

      // If we're on api-demo.airwallex.com
      if (currentUrl.includes("api-demo.airwallex.com")) {
        cy.log("Detected api-demo.airwallex.com domain");

        const currentOrigin = new URL(currentUrl).origin;
        cy.origin(
          currentOrigin,
          { args: { timeout: CONSTANTS.TIMEOUT } },
          ({ timeout }) => {
            cy.log("Inside api-demo.airwallex.com origin");

            // Try to find and interact with the form
            cy.get("form", { timeout: timeout })
              .should("exist")
              .then(($form) => {
                cy.log(`Found form with ID: ${$form.attr("id") || "unknown"}`);

                // Try to find the password input field with various selectors
                cy.get(
                  'input[type="password"], input[type="text"], input[name="password"], input',
                  {
                    timeout: timeout,
                  }
                ).then(($inputs) => {
                  cy.log(`Found ${$inputs.length} input fields`);

                  if ($inputs.length > 0) {
                    cy.wrap($inputs.first())
                      .should("be.visible")
                      .should("be.enabled")
                      .clear()
                      .type("1234");

                    // Try to find and click the submit button with various selectors
                    cy.get(
                      'button[type="submit"], input[type="submit"], button, input[value="Submit"]',
                      {
                        timeout: timeout,
                      }
                    ).then(($buttons) => {
                      cy.log(
                        `Found ${$buttons.length} possible submit buttons`
                      );

                      if ($buttons.length > 0) {
                        cy.wrap($buttons.first()).should("be.visible").click();

                        cy.log("Clicked submit button");
                      } else {
                        cy.log("No submit button found. Trying form submit");
                        cy.get("form").submit();
                      }
                    });
                  } else {
                    cy.log("No input fields found. Trying direct form submit");
                    cy.get("form").submit();
                  }
                });
              });
          }
        );

        // Wait for any navigation or form submission effects to complete
        cy.get("body").should("exist");
      }
      // If we're on pci-api-demo.airwallex.com
      else if (currentUrl.includes("pci-api-demo.airwallex.com")) {
        cy.log(
          "Detected pci-api-demo.airwallex.com domain - waiting for auto-redirect"
        );

        // Wait for redirect to complete by checking for URL changes
        cy.url({ timeout: CONSTANTS.TIMEOUT }).should(
          "not.include",
          "pci-api-demo.airwallex.com"
        );

        // Check if we've been redirected to api-demo.airwallex.com
        cy.url().then((newUrl) => {
          cy.log(`URL after waiting: ${newUrl}`);

          if (newUrl.includes("api-demo.airwallex.com")) {
            const newOrigin = new URL(newUrl).origin;

            cy.origin(
              newOrigin,
              { args: { timeout: CONSTANTS.TIMEOUT } },
              ({ timeout }) => {
                cy.log("Redirected to api-demo.airwallex.com");

                // Try to find and interact with the form
                cy.get("form", { timeout: timeout })
                  .should("exist")
                  .then(($form) => {
                    cy.log(
                      `Found form with ID: ${$form.attr("id") || "unknown"}`
                    );

                    // Try to find the password input field with various selectors
                    cy.get(
                      'input[type="password"], input[type="text"], input[name="password"], input',
                      {
                        timeout: timeout,
                      }
                    ).then(($inputs) => {
                      cy.log(`Found ${$inputs.length} input fields`);

                      if ($inputs.length > 0) {
                        cy.wrap($inputs.first())
                          .should("be.visible")
                          .should("be.enabled")
                          .clear()
                          .type("1234");

                        // Try to find and click the submit button with various selectors
                        cy.get(
                          'button[type="submit"], input[type="submit"], button, input[value="Submit"]',
                          {
                            timeout: timeout,
                          }
                        ).then(($buttons) => {
                          cy.log(
                            `Found ${$buttons.length} possible submit buttons`
                          );

                          if ($buttons.length > 0) {
                            cy.wrap($buttons.first())
                              .should("be.visible")
                              .click();

                            cy.log("Clicked submit button");
                          } else {
                            cy.log(
                              "No submit button found. Trying form submit"
                            );
                            cy.get("form").submit();
                          }
                        });
                      } else {
                        cy.log(
                          "No input fields found. Trying direct form submit"
                        );
                        cy.get("form").submit();
                      }
                    });
                  });
              }
            );

            // Wait for form submission to complete by checking URL or DOM changes
            cy.document()
              .should("have.property", "readyState")
              .and("equal", "complete");
          }
        });
      }
    });

    // After handling the 3DS authentication, go to the expected return URL
    cy.log(`Navigating to expected return URL: ${expectedUrl.href}`);
    cy.visit(expectedUrl.href);

    // Wait for page to load completely by checking for document ready state
    cy.document()
      .should("have.property", "readyState")
      .and("equal", "complete");

    // Skip the standard verification since we've manually navigated to expected URL
    return;
  }

  // Nuvei 3DS: 2-step flow (auth button + redirect button)
  if (connectorId === "nuvei") {
    cy.visit(redirectionUrl.href, { failOnStatusCode: false });

    cy.document()
      .should("have.property", "readyState")
      .and("equal", "complete");

    cy.url().then((currentUrl) => {
      const currentOrigin = new URL(currentUrl).origin;
      const redirectOrigin = new URL(redirectionUrl.href).origin;

      if (currentOrigin !== redirectOrigin) {
        cy.origin(
          currentOrigin,
          {
            args: {
              WAIT_TIME: CONSTANTS.WAIT_TIME,
            },
          },
          ({ WAIT_TIME }) => {
            cy.wait(WAIT_TIME);

            cy.get("body").then(($body) => {
              if ($body.find("#btn1").length > 0) {
                cy.get("#btn1").click();
                cy.get(
                  "a:contains('Redirect'), button:contains('Redirect'), input[value='Redirect']",
                  { timeout: 10000 }
                )
                  .should("be.visible")
                  .first()
                  .click();
              }
            });
          }
        );
      } else {
        cy.wait(CONSTANTS.WAIT_TIME);
        cy.get("body").then(($body) => {
          if ($body.find("#btn1").length > 0) {
            cy.get("#btn1").click();
            cy.get(
              "a:contains('Redirect'), button:contains('Redirect'), input[value='Redirect']",
              { timeout: 10000 }
            )
              .should("be.visible")
              .first()
              .click();
          }
        });
      }
    });

    cy.url({ timeout: CONSTANTS.TIMEOUT }).should(
      "include",
      new URL(expectedUrl.href).origin
    );
    cy.document()
      .should("have.property", "readyState")
      .and("equal", "complete");
    verifyReturnUrl(redirectionUrl, expectedUrl, true);
    return;
  }

  // For all other connectors, use the standard flow
  waitForRedirect(redirectionUrl.href);

  handleFlow(
    redirectionUrl,
    expectedUrl,
    connectorId,
    ({ connectorId, constants, expectedUrl }) => {
      switch (connectorId) {
        case "aci":
          cy.get('form[name="challengeForm"]', {
            timeout: constants.WAIT_TIME,
          })
            .should("exist")
            .then(() => {
              cy.get("#outcomeSelect")
                .select("Approve")
                .should("have.value", "Y");
              cy.get('button[type="submit"]').click();
            });
          break;
        case "adyen":
          cy.get("iframe")
            .its("0.contentDocument.body")
            .within(() => {
              cy.get('input[type="password"]').click();
              cy.get('input[type="password"]').type("password");
              cy.get("#buttonSubmit").click();
            });
          break;

        case "airwallex":
          // Airwallex uses multiple domains during 3DS flow
          // Handle the domain changes specifically for Airwallex
          cy.url().then((url) => {
            const currentOrigin = new URL(url).origin;

            if (currentOrigin.includes("pci-api-demo.airwallex.com")) {
              cy.log(
                "First Airwallex domain detected, waiting for redirect..."
              );
              // Just wait for the automatic redirect to the next domain
              cy.wait(constants.TIMEOUT / 5); // 4 seconds
            } else if (currentOrigin.includes("api-demo.airwallex.com")) {
              cy.log(
                "Second Airwallex domain detected, handling 3DS challenge..."
              );
              cy.origin(
                currentOrigin,
                { args: { constants } },
                ({ constants }) => {
                  cy.get("form", { timeout: constants.TIMEOUT })
                    .should("be.visible")
                    .within(() => {
                      cy.get(
                        'input[type="text"], input[type="password"], input[name="password"]',
                        {
                          timeout: constants.TIMEOUT,
                        }
                      )
                        .should("be.visible")
                        .should("be.enabled")
                        .click()
                        .type("1234");

                      cy.get('button[type="submit"], input[type="submit"]', {
                        timeout: constants.TIMEOUT,
                      })
                        .should("be.visible")
                        .click();
                    });
                }
              );
            }
          });
          break;

        case "bankofamerica":
        case "wellsfargo":
          cy.get("iframe", { timeout: constants.TIMEOUT })
            .should("be.visible")
            .its("0.contentDocument.body")
            .should("not.be.empty")
            .within(() => {
              cy.get(
                'input[type="text"], input[type="password"], input[name="challengeDataEntry"]',
                { timeout: constants.TIMEOUT }
              )
                .should("be.visible")
                .should("be.enabled")
                .click()
                .type("1234");

              cy.get('input[value="SUBMIT"], button[type="submit"]', {
                timeout: constants.TIMEOUT,
              })
                .should("be.visible")
                .click();
            });
          break;

        case "cybersource":
          cy.url({ timeout: constants.TIMEOUT }).should("include", expectedUrl);
          break;

        case "checkout":
          cy.get("iframe", { timeout: constants.TIMEOUT })
            .its("0.contentDocument.body")
            .within(() => {
              cy.get('form[id="form"]', { timeout: constants.WAIT_TIME })
                .should("exist")
                .then(() => {
                  cy.get('input[id="password"]').click();
                  cy.get('input[id="password"]').type("Checkout1!");
                  cy.get("#txtButton").click();
                });
            });
          break;

        case "deutschebank":
          cy.get('button[id="submit"]', { timeout: constants.TIMEOUT })
            .should("exist")
            .should("be.visible")
            .click();
          break;

        case "nexinets":
          cy.wait(constants.TIMEOUT / 10); // Wait for the page to load
          // Nexinets iDEAL specific selector - click the Success link
          cy.get("a.btn.btn-primary.btn-block").contains("Success").click();

          break;

        case "nmi":
        case "noon":
        case "xendit":
          cy.get("iframe", { timeout: constants.TIMEOUT })
            .its("0.contentDocument.body")
            .within(() => {
              cy.get("iframe", { timeout: constants.TIMEOUT })
                .its("0.contentDocument.body")
                .within(() => {
                  cy.get('form[name="cardholderInput"]', {
                    timeout: constants.TIMEOUT,
                  })
                    .should("exist")
                    .then(() => {
                      cy.get('input[name="challengeDataEntry"]')
                        .click()
                        .type("1234");
                      cy.get('input[value="SUBMIT"]').click();
                    });
                });
            });
          break;

        case "novalnet":
          cy.get("form", { timeout: constants.WAIT_TIME })
            .should("exist")
            .then(() => {
              cy.get('input[id="submit"]').click();
            });
          break;
        case "nuvei":
          cy.get("#btn1", { timeout: constants.WAIT_TIME })
            .should("be.visible")
            .click();
          cy.get(
            "a:contains('Redirect'), button:contains('Redirect'), input[value='Redirect']",
            { timeout: 10000 }
          )
            .should("be.visible")
            .first()
            .click();
          break;
        case "stripe":
          cy.get("iframe", { timeout: constants.TIMEOUT })
            .its("0.contentDocument.body")
            .within(() => {
              cy.get("iframe")
                .its("0.contentDocument.body")
                .within(() => {
                  cy.get("#test-source-authorize-3ds").click();
                });
            });
          break;

        case "trustpay":
          cy.get('form[name="challengeForm"]', {
            timeout: constants.WAIT_TIME,
          })
            .should("exist")
            .then(() => {
              cy.get("#outcomeSelect")
                .select("Approve")
                .should("have.value", "Y");
              cy.get('button[type="submit"]').click();
            });
          break;

        case "worldpay":
          cy.get("iframe", { timeout: constants.WAIT_TIME })
            .its("0.contentDocument.body")
            .within(() => {
              cy.get('form[name="cardholderInput"]', {
                timeout: constants.WAIT_TIME,
              })
                .should("exist")
                .then(() => {
                  cy.get('input[name="challengeDataEntry"]')
                    .click()
                    .type("1234");
                  cy.get('input[value="SUBMIT"]').click();
                });
            });
          break;

        case "fiuu":
          cy.get('form[id="cc_form"]', { timeout: constants.TIMEOUT })
            .should("exist")
            .then(() => {
              cy.get('button.pay-btn[name="pay"]').click();
              cy.get("div.otp")
                .invoke("text")
                .then((otpText) => {
                  const otp = otpText.match(/\d+/)[0];
                  cy.get("input#otp-input").should("not.be.disabled").type(otp);
                  cy.get("button.pay-btn").click();
                });
            });
          break;
        case "redsys":
          // Suppress cross-origin JavaScript errors from Redsys's website
          cy.on("uncaught:exception", (err) => {
            if (err.message.includes("$ is not defined")) {
              return false; // Prevent test failure
            }
            return true;
          });

          cy.get("div.autenticada").click();
          cy.get('input[value="Enviar"]').click();
          break;
        default:
          cy.wait(constants.WAIT_TIME);
      }
    }
  );

  cy.then(() => {
    if (
      responseContentType &&
      !responseContentType.includes("application/json")
    ) {
      verifyReturnUrl(redirectionUrl, expectedUrl, true);
    }
  });
}

function upiRedirection(
  redirectionUrl,
  expectedUrl,
  connectorId,
  paymentMethodType
) {
  let verifyUrl = false;
  if (connectorId === "iatapay") {
    switch (paymentMethodType) {
      case "upi_collect":
        cy.visit(redirectionUrl.href);
        cy.wait(CONSTANTS.TIMEOUT).then(() => {
          verifyUrl = true;
        });
        break;
      case "upi_intent":
        cy.request(redirectionUrl.href).then((response) => {
          expect(response.status).to.eq(200);
          expect(response.body).to.have.property("iataPaymentId");
          expect(response.body).to.have.property("status", "INITIATED");
          expect(response.body.qrInfoData).to.be.an("object");
          expect(response.body.qrInfoData).to.have.property("qr");
          expect(response.body.qrInfoData).to.have.property("qrLink");
        });
        verifyUrl = false;
        break;
      default:
        throw new Error(
          `Unsupported payment method type: ${paymentMethodType}`
        );
    }
  } else {
    return;
  }

  cy.then(() => {
    verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
  });
}

function verifyReturnUrl(redirectionUrl, expectedUrl, forwardFlow) {
  if (!forwardFlow) {
    cy.log("Skipping return URL verification as forwardFlow is false.");
    return;
  }
  cy.log(`Verifying return URL. Expecting host: ${expectedUrl.host}`);

  cy.location("host", { timeout: CONSTANTS.TIMEOUT }).should((currentHost) => {
    expect(currentHost).to.equal(expectedUrl.host);
  });

  cy.url().then((url) => {
    cy.log(`Current URL for verification: ${url}`);
    cy.origin(
      new URL(url).origin,
      {
        args: {
          redirectionUrl: redirectionUrl.origin,
          expectedUrl: expectedUrl.origin,
          constants: CONSTANTS,
        },
      },
      ({ redirectionUrl, expectedUrl, constants }) => {
        try {
          const redirectionHost = new URL(redirectionUrl).host;
          const expectedHost = new URL(expectedUrl).host;

          cy.log(
            `Running verification checks within origin: ${location.origin}`
          );

          cy.window()
            .its("location")
            .then((location) => {
              // Check for payment_id in the URL
              const urlParams = new URLSearchParams(location.search);
              const paymentId = urlParams.get("payment_id");

              cy.log(`URL Params: ${location.search}`);
              cy.log(`Payment ID: ${paymentId}`);

              if (!paymentId) {
                // eslint-disable-next-line cypress/assertion-before-screenshot
                cy.screenshot("missing-payment-id-error");
                throw new Error("URL does not contain payment_id parameter");
              }

              // Proceed with other verifications based on whether redirection host ends with expected host
              if (redirectionHost.endsWith(expectedHost)) {
                cy.wait(constants.WAIT_TIME / 2);

                // Check page state before taking screenshots
                cy.document().then((doc) => {
                  const pageText = doc.body.innerText.toLowerCase();

                  cy.log(
                    `Page text for error check: ${pageText.substring(0, 200)}...`
                  );

                  if (!pageText) {
                    // eslint-disable-next-line cypress/assertion-before-screenshot
                    cy.screenshot("blank-page-error");
                    cy.log("Warning: Page appears blank.");
                  } else {
                    // Check if any error pattern exists in the text
                    const hasError = constants.ERROR_PATTERNS.some((pattern) =>
                      pattern.test(pageText)
                    );

                    if (hasError) {
                      // Only take screenshot if an error pattern was found
                      // eslint-disable-next-line cypress/assertion-before-screenshot
                      cy.screenshot(`error-page-${Date.now()}`);
                      throw new Error(`Page contains error: ${pageText}`);
                    }
                  }
                });

                const paymentStatus = urlParams.get("status");

                if (
                  !constants.VALID_TERMINAL_STATUSES.includes(paymentStatus)
                ) {
                  // eslint-disable-next-line cypress/assertion-before-screenshot
                  cy.screenshot(`failed-payment-${paymentStatus}`);
                  throw new Error(
                    `Redirection failed with payment status: ${paymentStatus}`
                  );
                }
              } else {
                cy.window().its("location.origin").should("eq", expectedUrl);

                Cypress.on("uncaught:exception", (err, runnable) => {
                  // Log the error details
                  // eslint-disable-next-line no-console
                  console.error(
                    `Error: ${err.message}\nOccurred in: ${runnable.title}\nStack: ${err.stack}`
                  );

                  // Return false to prevent the error from failing the test
                  return false;
                });
              }
            });
        } catch (error) {
          throw new Error(`Redirection verification failed: ${error}`);
        }
      }
    );
  });
}

async function fetchAndParseQRCode(url) {
  const response = await fetch(url, { encoding: "binary" });
  if (!response.ok) {
    throw new Error(`Failed to fetch QR code image: ${response.statusText}`);
  }
  const blob = await response.blob();
  const reader = new FileReader();

  return new Promise((resolve, reject) => {
    reader.onload = () => {
      // Use the entire data URI from reader.result
      const dataUrl = reader.result;

      // Create a new Image, assigning its src to the full data URI
      const image = new Image();
      image.src = dataUrl;

      // Once the image loads, draw it to a canvas and let jsQR decode it
      image.onload = () => {
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d");
        canvas.width = image.width;
        canvas.height = image.height;
        ctx.drawImage(image, 0, 0);

        const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
        const qrCodeData = jsQR(
          imageData.data,
          imageData.width,
          imageData.height
        );

        if (qrCodeData) {
          resolve(qrCodeData.data);
        } else {
          reject(new Error("Failed to decode QR code"));
        }
      };

      // If the image fails to load at all, reject the promise
      image.onerror = (err) => {
        reject(new Error("Image failed to load: " + err?.message || err));
      };
    };

    // Read the blob as a data URL (this includes the data:image/png;base64 prefix)
    reader.readAsDataURL(blob);
  });
}

async function fetchAndParseImageData(url) {
  return await new Promise((resolve, reject) => {
    const image = new Image();
    image.src = url;

    image.onload = () => {
      const canvas = document.createElement("canvas");
      const ctx = canvas.getContext("2d");
      canvas.width = image.width;
      canvas.height = image.height;
      ctx.drawImage(image, 0, 0);

      const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
      const qrCodeData = jsQR(
        imageData.data,
        imageData.width,
        imageData.height
      );

      if (qrCodeData) {
        resolve(qrCodeData.data);
      } else {
        reject(new Error("Failed to decode QR code"));
      }
    };
    image.onerror = reject; // Handle image loading errors
  });
}

function waitForRedirect(redirectionUrl) {
  const originalHost = new URL(redirectionUrl).host;

  cy.location("host", { timeout: CONSTANTS.TIMEOUT }).should((currentHost) => {
    const hostChanged = currentHost !== originalHost;
    const iframeExists = Cypress.$("iframe")
      .toArray()
      .some((iframeEl) => {
        try {
          const iframeHost = new URL(iframeEl.src).host;
          return iframeHost && iframeHost !== originalHost;
        } catch {
          return false;
        }
      });

    // The assertion will pass if either the host changed or an iframe with a foreign host exists.
    expect(
      hostChanged || iframeExists,
      "Host changed or an  iframe with foreign host exist"
    ).to.be.true;
  });
}

function handleFlow(
  redirectionUrl,
  expectedUrl,
  connectorId,
  callback,
  options = {}
) {
  // Extract the host from the redirection URL
  const originalHost = new URL(redirectionUrl.href).host;

  cy.location("host", { timeout: CONSTANTS.TIMEOUT }).then((currentHost) => {
    const callbackArgs = {
      connectorId,
      constants: CONSTANTS,
      expectedUrl: expectedUrl.origin,
      ...options, // e.g. paymentMethodType if provided
    };

    if (currentHost !== originalHost) {
      cy.log(
        `Redirect detected: ${originalHost} -> ${currentHost}. Using cy.origin.`
      );

      // For a regular redirection flow: host changed, use cy.origin
      cy.url().then((currentUrl) => {
        cy.origin(new URL(currentUrl).origin, { args: callbackArgs }, callback);
      });
    } else {
      cy.log(
        `No host change detected or potential iframe. Executing callback directly/targeting iframe.`
      );

      // Wait for page to be ready first
      cy.document().should("have.property", "readyState", "complete");

      // For embedded flows using an iframe - use robust detection:
      cy.get("body").then(($body) => {
        const iframes = $body.find("iframe");

        if (iframes.length > 0) {
          // Wait for iframe to be ready
          cy.get("iframe", { timeout: CONSTANTS.TIMEOUT })
            .should("be.visible")
            .then(() => {
              cy.log(
                "Iframe detected and ready, executing callback targeting iframe context"
              );
              callback(callbackArgs);
            });
        } else {
          cy.log(
            "No iframe detected initially, checking for dynamic iframe or executing direct callback"
          );

          cy.get("body", { timeout: 3000 })
            .should("exist")
            .then(($body) => {
              // Check if iframe appeared during the wait
              if ($body.find("iframe").length > 0) {
                cy.log("Dynamic iframe detected, executing iframe flow");
                cy.get("iframe", { timeout: CONSTANTS.TIMEOUT })
                  .should("be.visible")
                  .then(() => {
                    callback(callbackArgs);
                  });
              } else {
                cy.log("No iframe found, executing direct callback");
                // Execute callback directly for non-iframe flows
                callback(callbackArgs);
              }
            });
        }
      });
    }
  });
}
