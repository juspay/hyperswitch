/* eslint-disable cypress/unsafe-to-chain-command */

import jsQR from "jsqr";

// Define constants for wait times
const CONSTANTS = {
  TIMEOUT: 20000, // 20 seconds
  WAIT_TIME: 10000, // 10 seconds
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

          case "mollie":
            cy.log(`Handling Mollie ${paymentMethodType} bank redirect`);
            
            switch (paymentMethodType) {
              case "ideal":
                cy.log("Handling Mollie iDEAL redirection");
                cy.get('body').then(($body) => {
                  const bodyText = $body.text();
                  
                  // Check for bank selection page
                  if (bodyText.includes("Select your bank") || bodyText.includes("Kies uw bank") || 
                      bodyText.includes("Choose your bank") || bodyText.includes("iDEAL")) {
                    cy.log("Found iDEAL bank selection page");
                    
                    // Look for ING bank option (common test bank)
                    cy.get('select, button, a, div[role="button"]').then(($elements) => {
                      const ingElement = $elements.filter((index, el) => {
                        const text = Cypress.$(el).text().toLowerCase();
                        const value = Cypress.$(el).val()?.toLowerCase() || '';
                        const optionText = Cypress.$(el).find('option').text().toLowerCase();
                        return text.includes('ing') || value.includes('ing') || 
                               text.includes('ingbnl2a') || value.includes('ingbnl2a') ||
                               optionText.includes('ing');
                      });
                      
                      if (ingElement.length > 0) {
                        cy.log("Selecting ING bank for iDEAL");
                        if (ingElement.is('select')) {
                          cy.wrap(ingElement).select('INGBNL2A');
                        } else {
                          cy.wrap(ingElement.first()).click();
                        }
                      } else {
                        // Fallback: select first available bank
                        cy.log("ING not found, selecting first available bank");
                        if ($body.find('select').length > 0) {
                          cy.get('select').first().select(1);
                        } else {
                          cy.get('button, a').first().click();
                        }
                      }
                    });
                    
                    // Submit the bank selection
                    cy.get('button[type="submit"], input[type="submit"]').then(($submitBtns) => {
                      if ($submitBtns.length > 0) {
                        cy.wrap($submitBtns.first()).click();
                      }
                    });
                  }
                  // Handle bank authentication page
                  else if (bodyText.includes("Continue") || bodyText.includes("Doorgaan") || 
                           bodyText.includes("Authorize") || bodyText.includes("Autoriseren")) {
                    cy.log("Found iDEAL authentication page");
                    cy.get('button, input[type="submit"]').then(($buttons) => {
                      const continueBtn = $buttons.filter((index, btn) => {
                        const text = Cypress.$(btn).text().toLowerCase();
                        return text.includes('continue') || text.includes('doorgaan') || 
                               text.includes('authorize') || text.includes('success');
                      });
                      
                      if (continueBtn.length > 0) {
                        cy.wrap(continueBtn.first()).click();
                      } else {
                        cy.wrap($buttons.first()).click();
                      }
                    });
                  }
                  // Fallback for any form submission
                  else if ($body.find('form').length > 0) {
                    cy.log("Found form, submitting for iDEAL");
                    cy.get('button[type="submit"], input[type="submit"]').first().click();
                  }
                });
                verifyUrl = true;
                break;
                
              case "bancontact_card":
                cy.log("Handling Mollie Bancontact redirection");
                cy.get('body').then(($body) => {
                  const bodyText = $body.text();
                  
                  // Check for Bancontact authentication page
                  if (bodyText.includes("Bancontact") || bodyText.includes("Card number") || 
                      bodyText.includes("Kaartnummer")) {
                    cy.log("Found Bancontact authentication page");
                    
                    // Handle card input if present
                    if ($body.find('input[type="text"]').length > 0) {
                      cy.get('input[type="text"]').first().type('1234567890123456');
                    }
                    
                    // Look for continue/submit buttons
                    cy.get('button, input[type="submit"]').then(($buttons) => {
                      const submitBtn = $buttons.filter((index, btn) => {
                        const text = Cypress.$(btn).text().toLowerCase();
                        return text.includes('continue') || text.includes('submit') || 
                               text.includes('doorgaan') || text.includes('bevestigen');
                      });
                      
                      if (submitBtn.length > 0) {
                        cy.wrap(submitBtn.first()).click();
                      } else {
                        cy.wrap($buttons.first()).click();
                      }
                    });
                  }
                  // Handle success/confirmation page
                  else if (bodyText.includes("Success") || bodyText.includes("Gelukt") || 
                           bodyText.includes("Approved") || bodyText.includes("Goedgekeurd")) {
                    cy.log("Found Bancontact success page");
                    cy.get('button, a').then(($elements) => {
                      if ($elements.length > 0) {
                        cy.wrap($elements.first()).click();
                      }
                    });
                  }
                  // Fallback
                  else if ($body.find('button, input[type="submit"]').length > 0) {
                    cy.log("Fallback: clicking first available button for Bancontact");
                    cy.get('button, input[type="submit"]').first().click();
                  }
                });
                verifyUrl = true;
                break;
                
              case "giropay":
                cy.log("Handling Mollie Giropay redirection");
                cy.get('body').then(($body) => {
                  const bodyText = $body.text();
                  
                  // Check for bank code input page
                  if (bodyText.includes("Bank code") || bodyText.includes("Bankleitzahl") || 
                      bodyText.includes("BLZ") || bodyText.includes("giropay")) {
                    cy.log("Found Giropay bank code input page");
                    
                    // Input bank code
                    if ($body.find('input[type="text"]').length > 0) {
                      cy.get('input[type="text"]').first().type('12345678');
                    }
                    
                    // Submit bank code
                    cy.get('button[type="submit"], input[type="submit"]').then(($submitBtns) => {
                      if ($submitBtns.length > 0) {
                        cy.wrap($submitBtns.first()).click();
                      }
                    });
                  }
                  // Handle bank selection page
                  else if (bodyText.includes("Select bank") || bodyText.includes("Bank auswählen")) {
                    cy.log("Found Giropay bank selection page");
                    cy.get('select, button').then(($elements) => {
                      if ($elements.filter('select').length > 0) {
                        cy.wrap($elements.filter('select').first()).select(1);
                      } else {
                        cy.wrap($elements.first()).click();
                      }
                    });
                  }
                  // Handle authentication page
                  else if (bodyText.includes("Continue") || bodyText.includes("Weiter") || 
                           bodyText.includes("Authorize") || bodyText.includes("Autorisieren")) {
                    cy.log("Found Giropay authentication page");
                    cy.get('button, input[type="submit"]').first().click();
                  }
                  // Fallback
                  else if ($body.find('form').length > 0) {
                    cy.log("Fallback: submitting form for Giropay");
                    cy.get('button[type="submit"], input[type="submit"]').first().click();
                  }
                });
                verifyUrl = true;
                break;
                
              case "eps":
                cy.log("Handling Mollie EPS redirection");
                cy.get('body').then(($body) => {
                  const bodyText = $body.text();
                  
                  // Check for Austrian bank selection
                  if (bodyText.includes("Select your bank") || bodyText.includes("Bank auswählen") || 
                      bodyText.includes("EPS") || bodyText.includes("Austria")) {
                    cy.log("Found EPS bank selection page");
                    
                    // Look for Austrian banks
                    cy.get('select, button, a').then(($elements) => {
                      const bankElement = $elements.filter((index, el) => {
                        const text = Cypress.$(el).text().toLowerCase();
                        const value = Cypress.$(el).val()?.toLowerCase() || '';
                        return text.includes('erste') || text.includes('raiffeisen') || 
                               text.includes('sparkasse') || value.includes('erste') ||
                               text.includes('bank austria');
                      });
                      
                      if (bankElement.length > 0) {
                        cy.log("Selecting Austrian bank for EPS");
                        if (bankElement.is('select')) {
                          cy.wrap(bankElement).select(1);
                        } else {
                          cy.wrap(bankElement.first()).click();
                        }
                      } else {
                        // Fallback: select first available option
                        cy.log("No specific Austrian bank found, selecting first option");
                        if ($body.find('select').length > 0) {
                          cy.get('select').first().select(1);
                        } else {
                          cy.get('button, a').first().click();
                        }
                      }
                    });
                    
                    // Submit selection
                    cy.get('button[type="submit"], input[type="submit"]').then(($submitBtns) => {
                      if ($submitBtns.length > 0) {
                        cy.wrap($submitBtns.first()).click();
                      }
                    });
                  }
                  // Handle authentication
                  else if (bodyText.includes("Continue") || bodyText.includes("Weiter")) {
                    cy.log("Found EPS authentication page");
                    cy.get('button, input[type="submit"]').first().click();
                  }
                  // Fallback
                  else if ($body.find('button, input[type="submit"]').length > 0) {
                    cy.log("Fallback: clicking button for EPS");
                    cy.get('button, input[type="submit"]').first().click();
                  }
                });
                verifyUrl = true;
                break;
                
              case "sofort":
                cy.log("Handling Mollie Sofort redirection");
                cy.get('body').then(($body) => {
                  const bodyText = $body.text();
                  
                  // Check for Sofort login page
                  if (bodyText.includes("Sofort") || bodyText.includes("Bank code") || 
                      bodyText.includes("Bankleitzahl") || bodyText.includes("Login")) {
                    cy.log("Found Sofort login page");
                    
                    // Handle bank code input
                    if ($body.find('input[name="BankCodeSearch"], input[placeholder*="Bank"]').length > 0) {
                      cy.get('input[name="BankCodeSearch"], input[placeholder*="Bank"]').first().type('88888888');
                    } else if ($body.find('input[type="text"]').length > 0) {
                      cy.get('input[type="text"]').first().type('88888888');
                    }
                    
                    // Submit bank code
                    cy.get('button[type="submit"], input[type="submit"]').then(($submitBtns) => {
                      if ($submitBtns.length > 0) {
                        cy.wrap($submitBtns.first()).click();
                      }
                    });
                  }
                  // Handle login credentials page
                  else if (bodyText.includes("User ID") || bodyText.includes("PIN") || 
                           bodyText.includes("Benutzerkennung")) {
                    cy.log("Found Sofort credentials page");
                    
                    // Fill in test credentials
                    if ($body.find('input[name="userid"], input[name="UserID"]').length > 0) {
                      cy.get('input[name="userid"], input[name="UserID"]').type('test');
                    }
                    if ($body.find('input[name="pin"], input[type="password"]').length > 0) {
                      cy.get('input[name="pin"], input[type="password"]').type('1234');
                    }
                    
                    cy.get('button[type="submit"], input[type="submit"]').first().click();
                  }
                  // Handle TAN/confirmation page
                  else if (bodyText.includes("TAN") || bodyText.includes("Confirm") || 
                           bodyText.includes("Bestätigen")) {
                    cy.log("Found Sofort TAN/confirmation page");
                    
                    if ($body.find('input[name="tan"]').length > 0) {
                      cy.get('input[name="tan"]').type('123456');
                    }
                    
                    cy.get('button, input[type="submit"]').then(($buttons) => {
                      const confirmBtn = $buttons.filter((index, btn) => {
                        const text = Cypress.$(btn).text().toLowerCase();
                        return text.includes('confirm') || text.includes('bestätigen') || 
                               text.includes('submit') || text.includes('weiter');
                      });
                      
                      if (confirmBtn.length > 0) {
                        cy.wrap(confirmBtn.first()).click();
                      } else {
                        cy.wrap($buttons.first()).click();
                      }
                    });
                  }
                  // Fallback
                  else if ($body.find('button, input[type="submit"]').length > 0) {
                    cy.log("Fallback: clicking button for Sofort");
                    cy.get('button, input[type="submit"]').first().click();
                  }
                });
                verifyUrl = true;
                break;
                
              case "przelewy24":
                cy.log("Handling Mollie Przelewy24 redirection");
                cy.get('body').then(($body) => {
                  const bodyText = $body.text();
                  
                  // Check for bank selection page
                  if (bodyText.includes("Wybierz bank") || bodyText.includes("Select bank") || 
                      bodyText.includes("Przelewy24") || bodyText.includes("P24")) {
                    cy.log("Found Przelewy24 bank selection page");
                    
                    // Look for popular Polish banks
                    cy.get('button, a, div[role="button"]').then(($elements) => {
                      const bankElement = $elements.filter((index, el) => {
                        const text = Cypress.$(el).text().toLowerCase();
                        return text.includes('pko') || text.includes('mbank') || 
                               text.includes('ing') || text.includes('millennium') ||
                               text.includes('santander') || text.includes('alior');
                      });
                      
                      if (bankElement.length > 0) {
                        cy.log("Selecting Polish bank for Przelewy24");
                        cy.wrap(bankElement.first()).click();
                      } else {
                        // Fallback: click first available bank
                        cy.log("No specific Polish bank found, selecting first option");
                        cy.get('button, a').first().click();
                      }
                    });
                  }
                  // Handle payment confirmation page
                  else if (bodyText.includes("Zapłać") || bodyText.includes("Pay") || 
                           bodyText.includes("Potwierdź") || bodyText.includes("Confirm")) {
                    cy.log("Found Przelewy24 payment confirmation page");
                    
                    cy.get('button, input[type="submit"]').then(($buttons) => {
                      const payBtn = $buttons.filter((index, btn) => {
                        const text = Cypress.$(btn).text().toLowerCase();
                        return text.includes('zapłać') || text.includes('pay') || 
                               text.includes('potwierdź') || text.includes('confirm');
                      });
                      
                      if (payBtn.length > 0) {
                        cy.wrap(payBtn.first()).click();
                      } else {
                        cy.wrap($buttons.first()).click();
                      }
                    });
                  }
                  // Handle success page
                  else if (bodyText.includes("Sukces") || bodyText.includes("Success") || 
                           bodyText.includes("Płatność zakończona")) {
                    cy.log("Found Przelewy24 success page");
                    cy.get('button, a').then(($elements) => {
                      if ($elements.length > 0) {
                        cy.wrap($elements.first()).click();
                      }
                    });
                  }
                  // Fallback
                  else if ($body.find('button, input[type="submit"]').length > 0) {
                    cy.log("Fallback: clicking button for Przelewy24");
                    cy.get('button, input[type="submit"]').first().click();
                  }
                });
                verifyUrl = true;
                break;
                
              default:
                throw new Error(`Unsupported Mollie payment method type: ${paymentMethodType}`);
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

    // extract the verifyUrl decision from within the handleFlow callback
    // since the callback runs asynchronously within cy.origin or directly,
    // we need a way to signal back if verification is needed.
    // we use a closure variable `verifyUrl` which is modified inside the callback.
    // this relies on cypress command queue ensuring the callback completes before cy.then runs.
  }
  cy.then(() => {
    // The value of verifyUrl determined by the specific flow (Adyen iDEAL or handleFlow callback)
    verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
  });
}

function threeDsRedirection(redirectionUrl, expectedUrl, connectorId) {
  cy.visit(redirectionUrl.href);

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

  // For all other connectors, use the standard flow
  waitForRedirect(redirectionUrl.href);

  handleFlow(
    redirectionUrl,
    expectedUrl,
    connectorId,
    ({ connectorId, constants, expectedUrl }) => {
      switch (connectorId) {
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
          cy.get("div.autenticada").click();
          cy.get('input[value="Enviar"]').click();
          break;

        case "mollie":
          cy.log("Handling Mollie payment redirection");
          cy.wait(constants.WAIT_TIME);
          
          cy.get("body").then(($body) => {
            const bodyText = $body.text();
            
            cy.log(`Mollie page content: ${bodyText.substring(0, 200)}...`);
            
            // Check for Mollie's payment completion page (English)
            if (bodyText.includes("Payment completed") || bodyText.includes("Payment successful")) {
              cy.log("Payment completed successfully - looking for continue button");
              // Look for continue/return button
              cy.get('button, a, input[type="button"], input[type="submit"]').then(($elements) => {
                const continueElement = $elements.filter((index, el) => {
                  const text = Cypress.$(el).text().toLowerCase();
                  const value = Cypress.$(el).val()?.toLowerCase() || '';
                  return text.includes('continue') || text.includes('return') || 
                         text.includes('back') || value.includes('continue') ||
                         text.includes('proceed') || text.includes('next');
                });
                
                if (continueElement.length > 0) {
                  cy.log("Clicking continue button");
                  cy.wrap(continueElement.first()).click();
                } else {
                  cy.log("No continue button found, clicking first available button");
                  cy.wrap($elements.first()).click();
                }
              });
            }
            // Check for Mollie's payment completion page (Dutch)
            else if (bodyText.includes("Betaling voltooid") || bodyText.includes("Betaling geslaagd")) {
              cy.log("Betaling voltooid - looking for terug/doorgaan button");
              cy.get('button, a, input[type="button"], input[type="submit"]').then(($elements) => {
                const continueElement = $elements.filter((index, el) => {
                  const text = Cypress.$(el).text().toLowerCase();
                  const value = Cypress.$(el).val()?.toLowerCase() || '';
                  return text.includes('terug') || text.includes('doorgaan') || 
                         text.includes('verder') || value.includes('doorgaan') ||
                         text.includes('continue') || text.includes('return');
                });
                
                if (continueElement.length > 0) {
                  cy.log("Clicking Dutch continue button");
                  cy.wrap(continueElement.first()).click();
                } else {
                  cy.log("No Dutch continue button found, clicking first available button");
                  cy.wrap($elements.first()).click();
                }
              });
            }
            // Check for payment method selection page
            else if (bodyText.includes("Select payment method") || bodyText.includes("Kies betaalmethode") || 
                     bodyText.includes("Choose your payment method") || bodyText.includes("Payment method")) {
              cy.log("On payment method selection page");
              // For card payments, look for card/credit card option
              cy.get('button, a, div[role="button"]').then(($elements) => {
                const cardElement = $elements.filter((index, el) => {
                  const text = Cypress.$(el).text().toLowerCase();
                  const ariaLabel = Cypress.$(el).attr('aria-label')?.toLowerCase() || '';
                  return text.includes('card') || text.includes('credit') || 
                         text.includes('kaart') || text.includes('creditcard') ||
                         ariaLabel.includes('card') || ariaLabel.includes('credit');
                });
                
                if (cardElement.length > 0) {
                  cy.log("Clicking card payment option");
                  cy.wrap(cardElement.first()).click();
                } else {
                  cy.log("No card option found, clicking first payment method");
                  cy.wrap($elements.first()).click();
                }
              });
            }
            // Check for PayPal redirection page
            else if (bodyText.includes("PayPal") || bodyText.includes("paypal") || 
                     bodyText.includes("Log in to your PayPal account") || bodyText.includes("Inloggen bij PayPal")) {
              cy.log("Found PayPal redirection page");
              
              // Handle PayPal login simulation
              if ($body.find('input[type="email"], input[name="login_email"]').length > 0) {
                cy.log("Found PayPal login form");
                cy.get('input[type="email"], input[name="login_email"]').first().type('test@example.com');
                
                if ($body.find('input[type="password"], input[name="login_password"]').length > 0) {
                  cy.get('input[type="password"], input[name="login_password"]').first().type('testpassword');
                }
                
                cy.get('button[type="submit"], input[type="submit"]').then(($submitBtns) => {
                  const loginBtn = $submitBtns.filter((index, btn) => {
                    const text = Cypress.$(btn).text().toLowerCase();
                    return text.includes('log in') || text.includes('sign in') || 
                           text.includes('inloggen') || text.includes('login');
                  });
                  
                  if (loginBtn.length > 0) {
                    cy.wrap(loginBtn.first()).click();
                  } else {
                    cy.wrap($submitBtns.first()).click();
                  }
                });
              }
              // Handle PayPal payment confirmation
              else if ($body.find('button, a').length > 0) {
                cy.get('button, a').then(($elements) => {
                  const payBtn = $elements.filter((index, el) => {
                    const text = Cypress.$(el).text().toLowerCase();
                    return text.includes('pay now') || text.includes('continue') || 
                           text.includes('agree') || text.includes('confirm') ||
                           text.includes('nu betalen') || text.includes('doorgaan') ||
                           text.includes('akkoord') || text.includes('bevestigen');
                  });
                  
                  if (payBtn.length > 0) {
                    cy.log("Clicking PayPal payment confirmation button");
                    cy.wrap(payBtn.first()).click();
                  } else {
                    cy.log("Clicking first available PayPal button");
                    cy.wrap($elements.first()).click();
                  }
                });
              }
            }
            // Check for Apple Pay redirection (if applicable)
            else if (bodyText.includes("Apple Pay") || bodyText.includes("Touch ID") || 
                     bodyText.includes("Face ID") || bodyText.includes("Authenticate")) {
              cy.log("Found Apple Pay authentication page");
              
              // Apple Pay in test environment usually auto-completes or has a simple confirmation
              cy.get('button, a').then(($elements) => {
                const confirmBtn = $elements.filter((index, el) => {
                  const text = Cypress.$(el).text().toLowerCase();
                  return text.includes('pay') || text.includes('confirm') || 
                         text.includes('authenticate') || text.includes('continue') ||
                         text.includes('betalen') || text.includes('bevestigen');
                });
                
                if (confirmBtn.length > 0) {
                  cy.log("Clicking Apple Pay confirmation button");
                  cy.wrap(confirmBtn.first()).click();
                } else {
                  cy.log("Clicking first available Apple Pay button");
                  cy.wrap($elements.first()).click();
                }
              });
            }
            // Check for 3DS challenge page
            else if (bodyText.includes("3D Secure") || bodyText.includes("3DS") || 
                     bodyText.includes("Authentication") || bodyText.includes("Verify")) {
              cy.log("Found 3DS challenge page");
              
              // Look for password input field
              if ($body.find('input[type="password"]').length > 0) {
                cy.log("Found password field for 3DS");
                cy.get('input[type="password"]').type("password");
                cy.get('button[type="submit"], input[type="submit"]').first().click();
              }
              // Look for any input field that might be for 3DS challenge
              else if ($body.find('input[type="text"]').length > 0) {
                cy.log("Found text input for 3DS challenge");
                cy.get('input[type="text"]').first().type("1234");
                cy.get('button[type="submit"], input[type="submit"]').first().click();
              }
              // Look for success/approve buttons
              else if ($body.find('button, input[type="button"]').length > 0) {
                cy.get('button, input[type="button"], input[type="submit"]').then(($buttons) => {
                  const successButton = $buttons.filter((index, btn) => {
                    const text = Cypress.$(btn).text().toLowerCase();
                    const value = Cypress.$(btn).val()?.toLowerCase() || '';
                    return text.includes('success') || text.includes('approve') || 
                           text.includes('authorize') || value.includes('success') ||
                           text.includes('confirm') || text.includes('submit');
                  });
                  
                  if (successButton.length > 0) {
                    cy.log("Clicking 3DS success button");
                    cy.wrap(successButton.first()).click();
                  } else {
                    cy.log("Clicking first available 3DS button");
                    cy.wrap($buttons.first()).click();
                  }
                });
              }
            }
            // Handle payment form submission
            else if ($body.find('form').length > 0 && 
                     ($body.find('input[type="submit"]').length > 0 || $body.find('button[type="submit"]').length > 0)) {
              cy.log("Found payment form, submitting");
              cy.get('input[type="submit"], button[type="submit"]').first().click();
            }
            // Handle iframe-based flows
            else if ($body.find('iframe').length > 0) {
              cy.log("Found iframe, attempting to interact with it");
              cy.get('iframe').first().its('0.contentDocument.body').within(() => {
                cy.get('body').then(($iframeBody) => {
                  const iframeText = $iframeBody.text();
                  
                  if (iframeText.includes('3DS') || iframeText.includes('Challenge') || 
                      iframeText.includes('Authentication')) {
                    // Handle 3DS within iframe
                    if ($iframeBody.find('input[type="password"]').length > 0) {
                      cy.get('input[type="password"]').type("password");
                      cy.get('button[type="submit"], input[type="submit"]').first().click();
                    } else if ($iframeBody.find('button, input[type="button"]').length > 0) {
                      cy.get('button, input[type="button"], input[type="submit"]').then(($buttons) => {
                        if ($buttons.length > 0) {
                          cy.log("Clicking first available button in iframe");
                          cy.wrap($buttons.first()).click();
                        }
                      });
                    }
                  }
                });
              });
            }
            // Fallback: look for any clickable element to proceed
            else {
              cy.log("Fallback: looking for any clickable element");
              cy.get('button, a, input[type="button"], input[type="submit"]').then(($elements) => {
                if ($elements.length > 0) {
                  cy.log("Clicking first available interactive element");
                  cy.wrap($elements.first()).click();
                } else {
                  cy.log("No interactive elements found, waiting for auto-redirect");
                  cy.wait(constants.WAIT_TIME / 2);
                }
              });
            }
          });
          break;

        default:
          cy.wait(constants.WAIT_TIME);
      }
    }
  );

  // Verify return URL after handling the specific connector
  verifyReturnUrl(redirectionUrl, expectedUrl, true);
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
    // For other connectors, nothing to do
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
    ).to.be.false;
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

      // For embedded flows using an iframe:
      cy.get("iframe", { timeout: CONSTANTS.TIMEOUT })
        .should("be.visible")
        .should("exist")
        .then((iframes) => {
          if (iframes.length === 0) {
            cy.log(
              "No host change and no iframe detected, executing callback directly."
            );

            throw new Error("No iframe found for embedded flow.");
          }
          // Execute the callback directly for the embedded flow
          cy.log(
            "Iframe detected, executing callback targeting iframe context (implicitly)."
          );
          callback(callbackArgs);
        });
    }
  });
}
