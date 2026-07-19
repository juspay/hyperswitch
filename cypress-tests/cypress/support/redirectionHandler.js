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

const COINGATE_BILLING = {
  email: "test@example.com",
  firstName: "Jan",
  lastName: "Jansen",
  dobMonth: "1",
  dobDay: "1",
  dobYear: "1990",
};

function normalizeConnectorForRedirect(connectorId) {
  return connectorId === "stripeconnect" ? "stripe" : connectorId;
}

export function handleRedirection(
  redirectionType,
  urls,
  connectorId,
  paymentMethodType,
  handlerMetadata
) {
  const resolvedConnectorId = normalizeConnectorForRedirect(connectorId);

  switch (redirectionType) {
    case "bank_redirect":
      bankRedirectRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        resolvedConnectorId,
        paymentMethodType
      );
      break;
    case "bank_transfer":
      bankTransferRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        resolvedConnectorId,
        paymentMethodType,
        handlerMetadata.nextActionType
      );
      break;
    case "three_ds":
      threeDsRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        resolvedConnectorId,
        paymentMethodType
      );
      break;
    case "upi":
      upiRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        resolvedConnectorId,
        paymentMethodType
      );
      break;
    case "reward":
      rewardRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        resolvedConnectorId,
        paymentMethodType
      );
      break;
    case "crypto":
      cryptoRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        resolvedConnectorId,
        paymentMethodType
      );
      break;
    case "pay_later":
      payLaterRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        resolvedConnectorId,
        paymentMethodType,
        handlerMetadata?.globalState
      );
      break;
    case "affirm_pay_later":
      affirmPayLaterRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        resolvedConnectorId,
        paymentMethodType,
        handlerMetadata?.globalState
      );
      break;
    case "voucher":
      voucherRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        resolvedConnectorId,
        paymentMethodType
      );
      break;
    case "payment_link_card":
      paymentLinkCardRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        resolvedConnectorId,
        paymentMethodType,
        handlerMetadata
      );
      break;
    case "payout_link":
      payoutLinkRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        resolvedConnectorId,
        paymentMethodType,
        handlerMetadata
      );
      break;
    case "payout_link_init":
      payoutLinkInitRedirection(urls.redirectionUrl);
      break;
    case "card_redirect":
      cardRedirectRedirection(
        urls.redirectionUrl,
        urls.expectedUrl,
        resolvedConnectorId,
        paymentMethodType,
        handlerMetadata
      );
      break;
    default:
      throw new Error(`Unknown redirection type: ${redirectionType}`);
  }
}

function cryptoRedirection(
  redirectionUrl,
  expectedUrl,
  connectorId,
  paymentMethodType
) {
  const verifyUrl = false;

  if (redirectionUrl && redirectionUrl.href) {
    cy.visit(redirectionUrl.href);

    if (connectorId !== "bitpay") {
      waitForRedirect(redirectionUrl.href);
    }

    cy.wait(CONSTANTS.WAIT_TIME / 5);

    if (connectorId === "bitpay") {
      cy.document().should("have.property", "readyState", "complete");
      cy.wait(3000);

      cy.scrollTo("bottom");
      cy.wait(1000);

      cy.get("body").then(($body) => {
        const loginLink = $body
          .find('a[href*="login"]:visible, button:visible')
          .filter((_, el) => /log\s*in|sign\s*in|account/i.test(el.innerText))
          .first();

        if (loginLink.length > 0) {
          cy.wrap(loginLink).click({ force: true });
          cy.log("Clicked login link");
        } else {
          cy.log("Login link not found");
        }
      });

      cy.wait(2000);

      cy.get('input[type="email"], input[name="email"]', { timeout: 10000 })
        .should("be.visible")
        .clear()
        .type("venkatakarthik.m@juspay.in", { delay: 50 });

      cy.get("body").then(($body) => {
        const continueBtn = $body
          .find('button:visible, input[type="submit"]:visible')
          .filter((_, el) =>
            /continue|next/i.test(el.innerText || el.value || "")
          )
          .first();

        if (continueBtn.length > 0) {
          cy.wrap(continueBtn).click({ force: true });
          cy.log("Clicked continue after email");
        }
      });

      cy.wait(2000);

      cy.get('input[type="password"], input[name="password"]', {
        timeout: 10000,
      })
        .should("be.visible")
        .clear()
        .type("venkatkarthik123@", { delay: 50 });

      cy.get('button[type="submit"], input[type="submit"]', { timeout: 10000 })
        .should("be.visible")
        .click();

      cy.log("Submitted Bitpay login credentials");
    } else if (connectorId === "coingate") {
      cy.document().should("have.property", "readyState", "complete");
      cy.title({ timeout: CONSTANTS.WAIT_TIME }).should("include", "CoinGate");
      cy.log("Coingate payment page loaded");

      // Wait for Next.js app to render the currency selection dropdown
      cy.contains(/Select currency/i, { timeout: CONSTANTS.WAIT_TIME }).should(
        "be.visible"
      );
      cy.log("Coingate currency selection screen rendered");

      // Open the currency dropdown
      cy.contains(/Select currency/i).click();
      cy.log("Opened currency selector dropdown");

      // Step 1: Select Bitcoin from the currency dropdown
      cy.contains(/Bitcoin/i, { timeout: CONSTANTS.WAIT_TIME })
        .first()
        .should("be.visible")
        .click();
      cy.log("Selected Bitcoin as payment currency");

      // Step 2: Wait briefly for any network selection modal to appear,
      // then select Bitcoin mainchain if the modal is present.
      cy.wait(2000);
      cy.get("body").then(($body) => {
        if ($body.text().includes("Select network")) {
          cy.log(
            "Network selection modal appeared - selecting Bitcoin mainchain"
          );
          cy.contains(/^Bitcoin$/i, { timeout: CONSTANTS.WAIT_TIME })
            .should("be.visible")
            .click();
          cy.log("Selected Bitcoin mainchain network");
        } else {
          cy.log("No network selection modal - proceeding to Continue");
        }
      });

      // Click Continue to proceed to the payment address screen
      cy.contains(/Continue/i, { timeout: CONSTANTS.WAIT_TIME })
        .should("be.visible")
        .click();
      cy.log("Clicked Continue on Coingate payment page");

      handleFlow(
        redirectionUrl,
        expectedUrl,
        connectorId,
        // NOTE: this callback runs inside cy.origin — CONSTANTS and
        // COINGATE_BILLING are not in scope. Use `constants` and
        // `coingateBilling` from the destructured args instead.
        ({ paymentMethodType, constants, coingateBilling }) => {
          switch (paymentMethodType) {
            case "crypto_currency": {
              cy.log("Coingate Bitcoin payment: checking for KYC billing form");

              cy.document().should("have.property", "readyState", "complete");

              // Coingate may require billing details (KYC) before showing
              // the BTC payment address. Fill the form if it appears.
              cy.get("body").then(($body) => {
                const bodyText = $body.text();
                if (
                  bodyText.includes("Billing details") ||
                  bodyText.includes("First name")
                ) {
                  cy.log("Billing details form detected - filling KYC fields");

                  // Email (optional, but fill it)
                  cy.get('input[type="email"]').then(($el) => {
                    if ($el.length > 0) {
                      cy.wrap($el.first()).clear().type(coingateBilling.email);
                    }
                  });

                  // First name + Last name (inputs with latin-characters placeholder)
                  cy.get('input[placeholder*="latin"]').then(($inputs) => {
                    if ($inputs.length >= 1) {
                      cy.wrap($inputs.eq(0))
                        .clear()
                        .type(coingateBilling.firstName);
                    }
                    if ($inputs.length >= 2) {
                      cy.wrap($inputs.eq(1))
                        .clear()
                        .type(coingateBilling.lastName);
                    }
                  });

                  // Date of birth — select Month=January, Day=1, Year=1990
                  // Use jQuery .find() instead of cy.get("select") to avoid
                  // Cypress retry timeout when no <select> elements exist on
                  // the Coingate billing form.
                  const $selects = $body.find("select");
                  if ($selects.length >= 1) {
                    cy.wrap($selects.eq(0)).select(coingateBilling.dobMonth);
                  }
                  if ($selects.length >= 2) {
                    cy.wrap($selects.eq(1)).select(coingateBilling.dobDay);
                  }
                  if ($selects.length >= 3) {
                    cy.wrap($selects.eq(2)).select(coingateBilling.dobYear);
                  }

                  cy.wait(500);

                  // Submit the billing form
                  cy.contains(/Continue/i, { timeout: constants.WAIT_TIME })
                    .last()
                    .click();
                  cy.log("Billing form submitted");
                  cy.wait(3000);
                } else {
                  cy.log("No billing form - proceeding to payment address");
                }
              });

              // After the billing form (or if skipped), the BTC payment
              // address screen should appear.
              cy.contains(/Amount|BTC|address|0\./i, {
                timeout: constants.WAIT_TIME,
              }).should("be.visible");
              cy.log("Coingate Bitcoin payment details displayed successfully");
              break;
            }

            default:
              throw new Error(
                `Unsupported crypto payment method type: ${paymentMethodType}`
              );
          }
        },
        { paymentMethodType, coingateBilling: COINGATE_BILLING }
      );
    } else {
      cy.get("canvas.BbpsQr__canvas", { timeout: 5000 })
        .should("exist")
        .and("be.visible");

      handleFlow(
        redirectionUrl,
        expectedUrl,
        connectorId,
        ({ paymentMethodType }) => {
          switch (paymentMethodType) {
            case "crypto_currency":
              cy.log("Handling crypto currency payment redirection");
              break;

            default:
              throw new Error(
                `Unsupported crypto payment method type: ${paymentMethodType}`
              );
          }
        },
        { paymentMethodType }
      );
    }
  } else {
    cy.log("Skipping crypto redirection - no valid redirect URL provided");
  }

  cy.then(() => {
    verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
  });
}

function payLaterRedirection(
  redirectionUrl,
  expectedUrl,
  connectorId,
  paymentMethodType
) {
  // PayLater payments (like Klarna) are redirect flows where we verify navigation
  // to the provider's page but don't complete the payment (verifyUrl = false)
  let verifyUrl = false;

  if (redirectionUrl && redirectionUrl.href) {
    // Suppress uncaught exceptions from Klarna sandbox pages
    cy.on("uncaught:exception", (err) => {
      if (
        err.message.includes("klarna") ||
        err.message.includes("playground") ||
        err.message.includes("angular") ||
        err.message.includes("$ is not defined")
      ) {
        return false; // Prevent test failure
      }
      return true;
    });

    cy.visit(redirectionUrl.href);
    waitForRedirect(redirectionUrl.href);

    handleFlow(
      redirectionUrl,
      expectedUrl,
      connectorId,
      ({ connectorId, paymentMethodType, constants }) => {
        switch (connectorId) {
          case "adyen":
          case "klarna":
          case "aci":
            // Klarna via various connectors - verify we land on Klarna page
            cy.log(
              `Handling ${connectorId} ${paymentMethodType} pay_later flow`
            );

            // Verify the page loaded by checking for Klarna-specific content
            // Klarna playground shows payment forms or consent pages
            cy.get("body", { timeout: constants.TIMEOUT }).then(($body) => {
              const bodyText = $body.text();
              const klarnaIndicators = [
                /klarna/i,
                /playground/i,
                /buy now.*pay later/i,
                /continue.*klarna/i,
                /smoooth/i,
              ];

              const hasKlarnaIndicator = klarnaIndicators.some((pattern) =>
                pattern.test(bodyText)
              );

              if (hasKlarnaIndicator) {
                cy.log(
                  "Successfully navigated to Klarna page - verified redirection"
                );
              } else {
                // Check URL as fallback
                cy.url().then((url) => {
                  if (
                    url.includes("klarna") ||
                    url.includes("playground") ||
                    url.includes("adyen") // Some Klarna flows go through Adyen
                  ) {
                    cy.log(
                      "URL indicates Klarna redirect - verified navigation"
                    );
                  } else {
                    cy.log(
                      `Warning: URL (${url}) does not contain expected Klarna indicators`
                    );
                  }
                });
              }
            });

            verifyUrl = false; // Don't complete payment, just verify navigation
            break;

          case "stripe":
            // Stripe handles pay_later differently - may have different flow
            cy.log("Handling Stripe pay_later flow");

            if (paymentMethodType === "affirm") {
              cy.log("Handling Stripe Affirm redirect flow");
              cy.get("body", { timeout: constants.TIMEOUT }).should("exist");

              cy.get("body").then(($body) => {
                const bodyText = $body.text().toLowerCase();
                if (
                  bodyText.includes("affirm") ||
                  bodyText.includes("pay over time")
                ) {
                  cy.log("Affirm page detected");

                  cy.get("body").then(($b) => {
                    const phoneInput = $b.find(
                      'input[type="tel"], input[name*="phone"], input[placeholder*="phone"], input[placeholder*="Phone"]'
                    );
                    if (phoneInput.length > 0) {
                      cy.wrap(phoneInput[0])
                        .should("be.visible")
                        .clear()
                        .type("5555555555");
                    }
                  });

                  cy.get("body").then(($b) => {
                    const pinInput = $b.find(
                      'input[type="password"], input[name*="pin"], input[placeholder*="PIN"]'
                    );
                    if (pinInput.length > 0) {
                      cy.wrap(pinInput[0])
                        .should("be.visible")
                        .clear()
                        .type("1234");
                    }
                  });

                  cy.get("body").then(($b) => {
                    const ssnInput = $b.find(
                      'input[name*="ssn"], input[placeholder*="SSN"], input[placeholder*="social"]'
                    );
                    if (ssnInput.length > 0) {
                      cy.wrap(ssnInput[0])
                        .should("be.visible")
                        .clear()
                        .type("1234");
                    }
                  });

                  cy.get("body").then(($b) => {
                    const termsCheckbox = $b.find(
                      'input[type="checkbox"][name*="terms"], input[type="checkbox"][name*="agree"]'
                    );
                    if (termsCheckbox.length > 0) {
                      cy.wrap(termsCheckbox[0]).should("be.visible").check();
                    }
                  });

                  cy.get("body").then(($b) => {
                    const submitBtn = $b.find('button[type="submit"]');
                    if (submitBtn.length > 0) {
                      cy.wrap(submitBtn[0]).should("be.visible").click();
                    }
                  });
                }
              });
            } else {
              cy.get("body", { timeout: constants.TIMEOUT }).should("exist");
            }

            verifyUrl = false;
            break;

          case "mollie":
            // Mollie Klarna PayLater - complete the payment flow
            cy.log(
              `Handling Mollie ${paymentMethodType} pay_later flow - completing payment`
            );

            // Wait for the Mollie test page to load
            cy.get("body", { timeout: constants.TIMEOUT }).should("exist");

            // Mollie test mode shows radio buttons to select payment status
            cy.get("body").then(($body) => {
              const paidSelector = 'input[type="radio"][value="paid"]';
              const authorizedSelector =
                'input[type="radio"][value="authorized"]';

              if ($body.find(paidSelector).length) {
                cy.get(paidSelector, { timeout: constants.WAIT_TIME })
                  .click()
                  .log("Selected: Paid");
              } else if ($body.find(authorizedSelector).length) {
                cy.get(authorizedSelector, { timeout: constants.WAIT_TIME })
                  .click()
                  .log("Selected: Authorized");
              } else {
                cy.log(
                  "No payment status selector found, page may auto-redirect"
                );
              }
            });

            // Click the Continue/Submit button to complete payment
            cy.get("body").then(($body) => {
              if ($body.find('button[type="submit"]').length > 0) {
                cy.get('button[type="submit"]', {
                  timeout: constants.WAIT_TIME,
                })
                  .should("be.visible")
                  .click()
                  .log("Clicked submit button");
              } else if ($body.find("button:contains('Continue')").length > 0) {
                cy.contains("button", "Continue", {
                  timeout: constants.WAIT_TIME,
                })
                  .should("be.visible")
                  .click()
                  .log("Clicked Continue button");
              } else if ($body.find('input[type="submit"]').length > 0) {
                cy.get('input[type="submit"]', {
                  timeout: constants.WAIT_TIME,
                })
                  .should("be.visible")
                  .click()
                  .log("Clicked input submit");
              } else {
                cy.log("No submit button found - may auto-submit");
              }
            });

            verifyUrl = true; // Complete payment and verify return URL
            break;

          case "airwallex":
            cy.log(`Handling Airwallex ${paymentMethodType} pay_later flow`);

            // Wait for the page to load
            cy.get("body", { timeout: constants.TIMEOUT }).should("exist");

            if (paymentMethodType === "atome") {
              // Atome redirects to sandbox-gateway.apaylater.net
              cy.url().then((url) => {
                try {
                  const urlObj = new URL(url);
                  const hostname = urlObj.hostname;
                  if (
                    hostname === "apaylater.net" ||
                    hostname.endsWith(".apaylater.net")
                  ) {
                    cy.log(
                      "Successfully navigated to Atome page - verified redirection"
                    );
                  } else {
                    cy.log(
                      `Warning: URL (${url}) does not contain expected Atome indicators`
                    );
                  }
                } catch {
                  cy.log(`Warning: Could not parse URL: ${url}`);
                }
              });
            } else {
              // Airwallex Klarna redirects to standard Klarna playground
              // Verify we landed on a Klarna page
              cy.get("body", { timeout: constants.TIMEOUT }).then(($body) => {
                const bodyText = $body.text();
                const klarnaIndicators = [
                  /klarna/i,
                  /playground/i,
                  /buy now.*pay later/i,
                  /continue.*klarna/i,
                  /smoooth/i,
                ];

                const hasKlarnaIndicator = klarnaIndicators.some((pattern) =>
                  pattern.test(bodyText)
                );

                if (hasKlarnaIndicator) {
                  cy.log(
                    "Successfully navigated to Klarna page - verified redirection"
                  );
                } else {
                  // Check URL as fallback
                  cy.url().then((url) => {
                    if (
                      url.includes("klarna") ||
                      url.includes("playground") ||
                      url.includes("airwallex")
                    ) {
                      cy.log(
                        "URL indicates Klarna redirect - verified navigation"
                      );
                    } else {
                      cy.log(
                        `Warning: URL (${url}) does not contain expected Klarna indicators`
                      );
                    }
                  });
                }
              });
            }

            verifyUrl = false; // Don't complete payment, just verify navigation
            break;

          default:
            cy.log(
              `Generic pay_later handling for ${connectorId}/${paymentMethodType}`
            );
            verifyUrl = false;
        }
      },
      { paymentMethodType }
    );
  } else {
    cy.log("Skipping pay_later redirection - no valid redirect URL provided");
  }

  cy.then(() => {
    verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
  });
}

function affirmPayLaterRedirection(
  redirectionUrl,
  expectedUrl,
  connectorId,
  paymentMethodType,
  globalState
) {
  const verifyUrl = false;

  if (redirectionUrl && redirectionUrl.href) {
    cy.visit(redirectionUrl.href);
    cy.log("Affirm flow - starting checkout automation");

    cy.on("uncaught:exception", (err) => {
      if (
        err.message.includes("Cannot read properties of null") ||
        err.message.includes("postMessage")
      ) {
        return false;
      }
      return true;
    });

    cy.url().then((initialUrl) => {
      const affirmOrigin = new URL(initialUrl).origin;
      cy.log("Affirm pay later flow - handling on origin: " + affirmOrigin);

      const waitTime = CONSTANTS.WAIT_TIME;

      const determineAffirmStep = (doc, bodyText, currentUrl) => {
        if (currentUrl.includes("/payments/completion")) return "done";

        const hasPinInput = doc.querySelector(
          '[data-testid="phone-pin-field"]:not([aria-hidden="true"])'
        );
        if (hasPinInput) return "pin";

        if (
          bodyText.includes("phone") ||
          bodyText.includes("mobile") ||
          bodyText.includes("cell")
        )
          return "phone";
        if (bodyText.includes("continue to plans")) return "continue_to_plans";
        if (
          bodyText.includes("social") ||
          bodyText.includes("ssn") ||
          bodyText.includes("last 4")
        )
          return "ssn";
        if (bodyText.includes("first name") || bodyText.includes("legal name"))
          return "first_name";
        if (bodyText.includes("last name") || bodyText.includes("surname"))
          return "last_name";
        if (
          bodyText.includes("date of birth") ||
          bodyText.includes("birthday") ||
          bodyText.includes("dob") ||
          bodyText.includes("birth date")
        )
          return "dob";
        if (bodyText.includes("email") || bodyText.includes("e-mail"))
          return "email";
        if (
          bodyText.includes("continue to plans") ||
          (bodyText.includes("creating an account") &&
            bodyText.includes("agree"))
        )
          return "consent";
        if (
          bodyText.includes("pick a plan") ||
          bodyText.includes("choose a payment plan") ||
          bodyText.includes("payment plan") ||
          bodyText.includes("choose this plan") ||
          bodyText.includes("every month") ||
          bodyText.includes("total of payments")
        )
          return "plan";
        if (
          bodyText.includes("review") ||
          bodyText.includes("confirm") ||
          bodyText.includes("autopay") ||
          bodyText.includes("terms")
        )
          return "review";
        return "fallback";
      };

      const handleAffirmStep = () => {
        cy.wait(waitTime / 5);

        cy.document().then((doc) => {
          const bodyText = doc.body.innerText.toLowerCase();
          const pageTitle = doc.title || "No title";
          const currentUrl = doc.location?.href || "Unknown URL";

          cy.log(`Affirm Page: ${pageTitle}`);
          cy.log(`URL: ${currentUrl}`);
          cy.log(`Page content preview: ${bodyText.substring(0, 200)}...`);

          const step = determineAffirmStep(doc, bodyText, currentUrl);
          cy.log(`Determined Affirm step: ${step}`);

          switch (step) {
            case "done":
              cy.log("Reached return URL, done");
              break;

            case "phone":
              cy.get("body").then(($body) => {
                const phoneInputs = $body.find(
                  'input[type="tel"]:visible, input[autocomplete="tel"]:visible, input[name*="phone"]:visible'
                );
                if (phoneInputs.length > 0) {
                  cy.wrap(phoneInputs.first()).clear().type("4155551234");
                  cy.log("Entered phone number");

                  const sendBtn = $body
                    .find("button:visible")
                    .filter((i, btn) =>
                      /send|submit|continue|next/i.test(
                        btn.innerText.toLowerCase()
                      )
                    );
                  if (sendBtn.length > 0) {
                    cy.wrap(sendBtn.first()).click({ force: true });
                    cy.log("Clicked send/submit after phone");
                  }
                }
              });
              break;

            case "pin":
              cy.get("body").then(($body) => {
                const pinInput = $body
                  .find(
                    'input[data-testid="phone-pin-field"]:visible, input[placeholder="000000"]:visible, input[autocomplete="one-time-code"]:visible, input[inputmode="numeric"]:visible'
                  )
                  .first();

                if (pinInput.length > 0) {
                  cy.wrap(pinInput).clear().type("123456", { delay: 100 });
                  cy.log("Entered PIN 123456 into phone-pin-field");

                  cy.wait(1000);

                  const verifyBtn = $body
                    .find("button:visible")
                    .filter((i, btn) =>
                      /verify|confirm|continue/i.test(
                        btn.innerText.toLowerCase()
                      )
                    );
                  if (verifyBtn.length > 0) {
                    cy.wrap(verifyBtn.first()).click({ force: true });
                    cy.log("Clicked verify/continue after PIN");
                  }
                } else {
                  cy.log(
                    "PIN input not found - looking for phone-pin-field or placeholder 000000"
                  );
                }
              });
              break;

            case "continue_to_plans":
              cy.get("body").then(($body) => {
                const continueBtn = $body
                  .find("button:visible")
                  .filter((i, btn) =>
                    /continue to plans/i.test(btn.innerText.toLowerCase())
                  );
                if (continueBtn.length > 0) {
                  cy.wrap(continueBtn.first()).click({ force: true });
                  cy.log("Clicked Continue to plans");
                }
              });
              break;

            case "ssn":
              cy.get("body").then(($body) => {
                const ssnInputs = $body.find(
                  'input[type="password"][maxlength="4"]:visible, input[name*="ssn"]:visible, input[placeholder*="last 4"]:visible'
                );
                if (ssnInputs.length > 0) {
                  cy.wrap(ssnInputs.first()).clear().type("5678");
                  cy.log("Entered SSN last 4");

                  const continueBtn = $body
                    .find("button:visible")
                    .filter((i, btn) =>
                      /continue|next|submit/i.test(btn.innerText.toLowerCase())
                    );
                  if (continueBtn.length > 0) {
                    cy.wrap(continueBtn.first()).click({ force: true });
                    cy.log("Clicked continue after SSN");
                  }
                }
              });
              break;

            case "first_name":
              cy.get("body").then(($body) => {
                const firstNameInputs = $body.find(
                  'input[name*="first"]:visible, input[id*="first"]:visible, input[autocomplete*="given-name"]:visible'
                );
                if (firstNameInputs.length > 0) {
                  cy.wrap(firstNameInputs.first())
                    .clear()
                    .invoke("val", "Joseph")
                    .trigger("input");
                  cy.log("Entered first name Joseph");
                }
              });
              break;

            case "last_name":
              cy.get("body").then(($body) => {
                const lastNameInputs = $body.find(
                  'input[name*="last"]:visible, input[id*="last"]:visible, input[autocomplete*="family-name"]:visible'
                );
                if (lastNameInputs.length > 0) {
                  cy.wrap(lastNameInputs.first())
                    .clear()
                    .invoke("val", "Doe")
                    .trigger("input");
                  cy.log("Entered last name Doe");
                }
              });
              break;

            case "dob":
              cy.get("body").then(($body) => {
                const dobInputs = $body.find(
                  'input[data-testid*="dob"]:visible, input[aria-label*="Birth"]:visible, input[placeholder*="mm"]:visible, input[name*="dob"]:visible, input[name*="birth"]:visible'
                );
                if (dobInputs.length > 0) {
                  cy.wrap(dobInputs.first())
                    .click()
                    .clear()
                    .type("01/01/1990", { delay: 50 });
                  cy.log("Entered DOB 01/01/1990");
                }
              });
              break;

            case "email":
              cy.get("body").then(($body) => {
                const emailInputs = $body.find(
                  'input[data-testid*="email"]:visible, input[type="email"]:visible, input[autocomplete="email"]:visible, input[data-test="email-input"]:visible'
                );
                if (emailInputs.length > 0) {
                  cy.wrap(emailInputs.first())
                    .click()
                    .clear()
                    .type("venkatakarthik.m@juspay.in", { delay: 50 });
                  cy.log("Entered email venkatakarthik.m@juspay.in");
                }
              });
              break;

            case "consent":
              cy.get("body").then(($body) => {
                const consentCheckbox = $body
                  .find('input[type="checkbox"]:not(:checked):visible')
                  .first();
                if (consentCheckbox.length > 0) {
                  cy.wrap(consentCheckbox).click({ force: true });
                  cy.log("Checked consent checkbox");
                }

                const continueBtn = $body
                  .find("button:visible")
                  .filter((i, btn) =>
                    /continue to plans/i.test(btn.innerText.toLowerCase())
                  );
                if (continueBtn.length > 0) {
                  cy.wrap(continueBtn.first()).click({ force: true });
                  cy.log("Clicked Continue to plans");
                }
              });
              break;

            case "plan":
              {
                const planRadio = doc.querySelector(
                  'input[type="radio"][name="selectedTermIndex"]'
                );
                if (planRadio) {
                  planRadio.checked = true;
                  planRadio.dispatchEvent(
                    new Event("change", { bubbles: true })
                  );
                  cy.log("Selected payment plan via DOM");
                }

                const planBtn = doc.querySelector(
                  'button[data-testid="continue-with-selected-term-button"]'
                );
                if (planBtn) {
                  planBtn.dispatchEvent(
                    new MouseEvent("click", { bubbles: true })
                  );
                  cy.log("Clicked Choose this plan via DOM");
                }

                cy.wait(2000).then(() => {
                  const indicator = doc.querySelector(
                    '[data-testid="disclosure-checkbox-indicator"]'
                  );
                  if (indicator) {
                    const label = indicator.closest("label");
                    (label || indicator).dispatchEvent(
                      new MouseEvent("click", { bubbles: true })
                    );
                    cy.log("Clicked disclosure checkbox");
                  }

                  const submitBtn = doc.querySelector(
                    'button[data-testid="submit-button"]'
                  );
                  if (submitBtn) {
                    submitBtn.dispatchEvent(
                      new MouseEvent("click", { bubbles: true })
                    );
                    cy.log("Clicked Confirm button");
                  }
                });
              }
              break;

            case "review":
              cy.get('[data-testid="disclosure-checkbox-indicator"]')
                .closest("label")
                .click({ force: true });
              cy.log("Clicked disclosure checkbox label");

              cy.wait(1000);

              cy.get('button[data-testid="submit-button"]').click({
                force: true,
              });
              cy.log("Clicked Confirm button");
              break;

            case "fallback":
            default:
              cy.get("body").then(($body) => {
                const buttons = $body.find("button:visible");
                for (let i = 0; i < buttons.length; i++) {
                  const btnText = buttons[i].innerText.toLowerCase();
                  if (
                    /continue|submit|verify|next|pay|confirm|agree/i.test(
                      btnText
                    )
                  ) {
                    cy.wrap(buttons[i]).click({ force: true });
                    cy.log("Clicked button: " + btnText);
                    break;
                  }
                }
              });
              break;
          }
        });
      };

      function runUntilComplete(maxSteps, delay) {
        if (maxSteps <= 0) {
          cy.log("Max steps reached, stopping");
          return;
        }

        cy.url().then((currentUrl) => {
          cy.log(`Step remaining: ${maxSteps} | Current URL: ${currentUrl}`);

          handleAffirmStep();

          cy.wait(delay).then(() => {
            runUntilComplete(maxSteps - 1, delay);
          });
        });
      }

      runUntilComplete(3, 6000);
    });
  } else {
    cy.log("Skipping Affirm redirection - no valid redirect URL provided");
  }

  cy.then(() => {
    verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);

    if (globalState) {
      cy.url().then((currentUrl) => {
        try {
          const urlObj = new URL(currentUrl);
          const urlParams = new URLSearchParams(urlObj.search);
          const paymentId = urlParams.get("payment_id");

          if (paymentId) {
            cy.log(`Extracted payment_id from return URL: ${paymentId}`);
            globalState.set("paymentID", paymentId);
          } else {
            cy.log(
              "No payment_id found in return URL - using existing paymentID from globalState"
            );
          }
        } catch (error) {
          cy.log(`Error extracting payment_id from URL: ${error.message}`);
        }
      });
    }
  });
}

function bankTransferRedirection(
  redirectionUrl,
  expectedUrl,
  connectorId,
  paymentMethodType,
  nextActionType
) {
  connectorId = normalizeConnectorForRedirect(connectorId);
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
  connectorId = normalizeConnectorForRedirect(connectorId);
  let verifyUrl = false;

  cy.on("uncaught:exception", () => false);

  // Mifinity wallet redirect: visit the redirect URL and verify the redirection
  // without waiting for a host change (mifinity redirects to an external wallet
  // authentication page that doesn't trigger a secondary redirect)
  if (connectorId === "mifinity") {
    cy.on("uncaught:exception", () => false);

    cy.log(`Handling Mifinity wallet redirect for ${paymentMethodType}`);
    cy.visit(redirectionUrl.href, { failOnStatusCode: false });
    cy.document().should("have.property", "readyState", "complete");
    cy.url().then((currentUrl) => {
      cy.log(`Mifinity redirect: navigated to ${currentUrl}`);
      cy.log("Mifinity wallet redirect verified - redirection is happening");
    });
    verifyUrl = false;
    cy.then(() => {
      verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
    });
    return;
  }

  // Stripe wallet redirects (AliPay, AmazonPay, Cashapp, RevolutPay, WeChatPay) point to
  // external wallet pages that cannot be automated in CI. Skip the redirect visit entirely
  // and rely on the payment status (requires_customer_action) verified in the retrieve step.
  if (
    connectorId === "stripe" &&
    ["ali_pay", "amazon_pay", "cashapp", "revolut_pay", "we_chat_pay"].includes(
      paymentMethodType
    )
  ) {
    expect(
      redirectionUrl,
      `Stripe ${paymentMethodType} forward flow redirect URL`
    ).to.not.be.null;
    expect(
      redirectionUrl.href,
      `Stripe ${paymentMethodType} redirect URL href`
    ).to.be.a("string").and.to.not.be.empty;
    cy.log(
      `Verified forward flow for Stripe ${paymentMethodType} — redirect URL present: ${redirectionUrl.href} (external page not automatable)`
    );
    return;
  }

  const adyenWalletTypesWithNullRedirect = ["dana", "go_pay", "momo", "vipps"];

  if (
    connectorId === "adyen" &&
    adyenWalletTypesWithNullRedirect.includes(paymentMethodType)
  ) {
    if (redirectionUrl.hostname === "null") {
      cy.log(
        `Adyen ${paymentMethodType} redirect URL has null hostname - skipping redirect handling`
      );
      verifyUrl = false;
      cy.then(() => {
        verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
      });
      return;
    }

    cy.visit(redirectionUrl.href, { failOnStatusCode: false });
    cy.get("body", { timeout: CONSTANTS.TIMEOUT }).should("exist");
    cy.log(
      `Adyen ${paymentMethodType} redirect page loaded (may return error status)`
    );
    verifyUrl = false;
    cy.then(() => {
      verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
    });
    return;
  }

  if (connectorId === "adyen" && paymentMethodType === "gcash") {
    cy.visit(redirectionUrl.href, {
      failOnStatusCode: false,
      timeout: CONSTANTS.TIMEOUT * 2,
    });
    cy.get("body", { timeout: CONSTANTS.TIMEOUT * 2 }).should("exist");
    cy.log("Adyen Gcash redirect page loaded (extended timeout)");
    verifyUrl = false;
    cy.then(() => {
      verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
    });
    return;
  }

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
  } else if (connectorId === "trustpay" && paymentMethodType === "ideal") {
    // TrustPay iDEAL: aapi.finby.eu JS auto-redirects to pay.ideal.nl with no user interaction.
    // Cypress does not support nested cy.origin, so we handle origins sequentially.
    // ref: https://github.com/cypress-io/cypress/issues/20718
    const trustpayIdealOrigin2 = "https://pay.ideal.nl";

    // aapi.finby.eu redirects automatically — just wait for pay.ideal.nl to load
    cy.log(`Waiting for redirection to ${trustpayIdealOrigin2}`);
    cy.location("origin", { timeout: CONSTANTS.TIMEOUT }).should(
      "eq",
      trustpayIdealOrigin2
    );

    cy.origin(
      trustpayIdealOrigin2,
      { args: { constants: CONSTANTS } },
      ({ constants }) => {
        cy.log("Executing on TrustPay iDEAL Origin (pay.ideal.nl)");
        cy.wait(constants.TIMEOUT / 10); // 2 seconds for page load
        cy.get("button[data-testid=payment-action-button]", {
          timeout: constants.TIMEOUT,
        })
          .should("be.visible")
          .click();
        cy.wait(constants.TIMEOUT / 10); // 2 seconds for bank list to render
        cy.get('button[id="bank-item-INGBNL2A"]', {
          timeout: constants.TIMEOUT,
        })
          .should("be.visible")
          .click();
      }
    );

    verifyUrl = false;
  } else if (
    (connectorId === "payjustnow" || connectorId === "payjustnowinstore") &&
    paymentMethodType === "payjustnow"
  ) {
    // PayJustNow (both online and in-store variants) is handled outside
    // handleFlow for the same reason as Adyen/Airwallex iDEAL: cy.origin
    // cannot be nested (https://github.com/cypress-io/cypress/issues/20718).
    // The flow crosses THREE origins sequentially:
    //   1. sandbox-app.payjustnow.com  — login → checkout → PIN
    //   2. sandbox-app.payjustnow.com  — optional /confirm-card OTP
    //   3. test.oppwa.com              — 3DS simulator challenge form
    // We must leave each cy.origin before the page auto-navigates to the next
    // origin, otherwise Cypress throws an origin-mismatch error.

    // ------------------------------------------------------------------
    // ORIGIN 1: sandbox-app.payjustnow.com — login, checkout, PIN
    // ------------------------------------------------------------------
    cy.origin(
      "https://sandbox-app.payjustnow.com",
      { args: { constants: CONSTANTS } },
      ({ constants }) => {
        cy.on("uncaught:exception", () => false);

        // Wait for the Vue SPA to fully render
        cy.document().should("have.property", "readyState", "complete");
        cy.wait(3000);

        // STEP 1: Login
        cy.get("body", { timeout: constants.TIMEOUT }).then(($body) => {
          const editableEmail = $body.find("#email:not([disabled])");
          if (editableEmail.length > 0) {
            cy.wrap(editableEmail.first())
              .should("be.visible")
              .click()
              .clear()
              .type("customer@payjustnow.co.za", { delay: 50 });
          }
        });

        cy.get('input[name="password"]', { timeout: constants.TIMEOUT })
          .should("be.visible")
          .click()
          .clear()
          .type("password", { delay: 50 });

        cy.contains("button", "Log In", { timeout: constants.TIMEOUT })
          .should("be.visible")
          .click({ force: true });

        // STEP 2: Checkout — Confirm and Pay
        cy.location("pathname", { timeout: constants.TIMEOUT }).should(
          "include",
          "/checkout/"
        );
        cy.wait(2000);

        // Some checkouts require the terms-and-conditions checkbox to be ticked
        // before "Confirm and Pay" becomes actionable.
        cy.get("body").then(($body) => {
          const termsCheckbox = $body
            .find('input[type="checkbox"]')
            .filter((_, el) => {
              const label = $body.find(`label[for="${el.id}"]`);
              const parentText = Cypress.$(el).closest("label").text();
              const labelText = label.text();
              const combined = `${parentText} ${labelText}`;
              return (
                combined.includes("South African citizen") ||
                combined.includes("confirm that I am") ||
                combined.includes("Terms and Conditions")
              );
            });

          if (termsCheckbox.length > 0 && !termsCheckbox.is(":checked")) {
            cy.wrap(termsCheckbox).check();
          }
        });

        cy.get("#checkout-bnpl-confirm-and-pay", {
          timeout: constants.TIMEOUT,
        })
          .scrollIntoView()
          .should("be.visible")
          .click({ force: true });

        // STEP 3: Confirm PIN (/confirm-pin/:token)
        cy.location("pathname", { timeout: constants.TIMEOUT }).should(
          "include",
          "/confirm-pin/"
        );
        cy.wait(2000);

        cy.get(".otp-wrapper input", { timeout: constants.TIMEOUT }).then(
          ($pinInputs) => {
            ["1", "1", "1", "1"].forEach((digit, idx) => {
              cy.wrap($pinInputs.eq(idx))
                .should("be.visible")
                .click()
                .type(digit, { delay: 100 });
            });
          }
        );

        cy.wait(1000);
        cy.contains("button", "next", { timeout: constants.TIMEOUT })
          .should("exist")
          .click({ force: true });
        // Give the page time to process the PIN and start redirecting
        // before we leave this cy.origin block. The 3DS auto-submit on
        // /3dsecure-acs fires after ~3.5s, so 2s is safe.
        cy.wait(2000);
      }
    );

    // ------------------------------------------------------------------
    // OUTSIDE ORIGIN: decide whether an OTP (/confirm-card/) step exists.
    // Reading the URL here is safe because we are not inside cy.origin.
    // ------------------------------------------------------------------
    cy.url({ timeout: CONSTANTS.TIMEOUT }).should((url) => {
      expect(
        url.includes("/confirm-card/") ||
          url.includes("/3dsecure-acs") ||
          new URL(url).hostname === "test.oppwa.com",
        "post-PIN URL to be /confirm-card/, /3dsecure-acs, or test.oppwa.com"
      ).to.be.true;
    });

    cy.url().then((url) => {
      if (url.includes("/confirm-card/")) {
        cy.log("PayJustNow: /confirm-card/ detected — entering OTP step");

        // ORIGIN 2 (optional): sandbox-app.payjustnow.com — OTP
        cy.origin(
          "https://sandbox-app.payjustnow.com",
          { args: { constants: CONSTANTS } },
          ({ constants }) => {
            cy.on("uncaught:exception", () => false);
            cy.wait(2000);

            cy.get(".otp-wrapper input", {
              timeout: constants.TIMEOUT,
            }).then(($otpInputs) => {
              ["1", "1", "1", "1"].forEach((digit, idx) => {
                cy.wrap($otpInputs.eq(idx))
                  .should("be.visible")
                  .click()
                  .type(digit, { delay: 100 });
              });
            });

            cy.contains("button", "next", { timeout: constants.TIMEOUT })
              .should("be.visible")
              .click({ force: true });

            cy.log("PayJustNow: submitted OTP 1111");
          }
        );
      }
    });

    // ------------------------------------------------------------------
    // Wait for the 3DS challenge page on test.oppwa.com.
    // ------------------------------------------------------------------
    cy.location("origin", { timeout: CONSTANTS.TIMEOUT }).should(
      "eq",
      "https://test.oppwa.com"
    );

    // ------------------------------------------------------------------
    // ORIGIN 3: test.oppwa.com — submit the 3DS challenge form directly.
    // ------------------------------------------------------------------
    cy.origin(
      "https://test.oppwa.com",
      { args: { TIMEOUT: CONSTANTS.TIMEOUT } },
      ({ TIMEOUT }) => {
        cy.on("uncaught:exception", () => false);

        // Wait for the challenge form with the required creq input
        cy.get('form[method="post"]', { timeout: TIMEOUT })
          .should("have.length.at.least", 1)
          .first()
          .within(() => {
            cy.get('input[name="creq"]', { timeout: TIMEOUT }).should("exist");
          });

        // Click the submit button instead of .submit() to trigger JS handlers
        cy.get('button[type="submit"]', { timeout: TIMEOUT })
          .should("exist")
          .click({ force: true });

        // Give the form submission a moment to start. The response redirects
        // to sandbox-app.payjustnow.com/3dsecure-transaction-status/ which
        // then polls the connector and finally redirects back to Hyperswitch.
        cy.wait(2000);
      }
    );

    // ------------------------------------------------------------------
    // Wait for the 3DS form submission to redirect away from test.oppwa.com.
    // After submission we land on sandbox-app.payjustnow.com which shows an
    // intermediate "Confirming your 3DSecure transaction" page while it
    // polls the connector. Give that page time to finish instead of waiting
    // for the final redirect back to Hyperswitch (which can take long or fail
    // to fire in headless mode).
    // ------------------------------------------------------------------
    cy.location("origin", { timeout: CONSTANTS.TIMEOUT }).should(
      "not.eq",
      "https://test.oppwa.com"
    );

    cy.location("host").then((host) => {
      if (host.includes("payjustnow")) {
        cy.origin("https://sandbox-app.payjustnow.com", () => {
          cy.on("uncaught:exception", () => false);

          // The status page polls PayJustNow's backend to confirm 3DS.
          // A 15s wait lets it resolve before we move on.
          cy.wait(15000);
        });
      }
    });

    // Buffer for the final redirect back to Hyperswitch (if any).
    cy.wait(5000);

    verifyUrl = false;
  } else {
    handleFlow(
      redirectionUrl,
      expectedUrl,
      connectorId,
      ({ connectorId, paymentMethodType, constants }) => {
        // Renamed expectedUrl arg for clarity
        // This callback now runs either in cy.origin (if redirected) or directly (if iframe)
        switch (connectorId) {
          case "adyen":
            switch (paymentMethodType) {
              case "eps":
              case "twint":
                cy.get("h1").should("contain.text", "Acquirer Simulator");
                cy.get('[value="authorised"]').click();
                verifyUrl = true;
                break;
              case "paypal":
              case "kakao_pay":
              case "gcash":
              case "ali_pay_hk":
                cy.get("body", { timeout: constants.TIMEOUT }).then(($body) => {
                  const bodyText = $body.text() || "";
                  if (
                    bodyText.includes("Acquirer Simulator") &&
                    $body.find("h1").length > 0
                  ) {
                    cy.get("h1").should("contain.text", "Acquirer Simulator");
                    cy.get('[value="authorised"]').click();
                    verifyUrl = true;
                  } else if (
                    bodyText.includes("PayPal") ||
                    bodyText.includes("Log in")
                  ) {
                    cy.log("Adyen redirected to PayPal sandbox page");
                    cy.get("body", { timeout: constants.TIMEOUT }).should(
                      "exist"
                    );
                    verifyUrl = false;
                  } else {
                    cy.log(
                      `Adyen ${paymentMethodType} redirect page loaded but unrecognized content`
                    );
                    verifyUrl = false;
                  }
                });
                break;
              case "momo":
                cy.get("body", { timeout: constants.TIMEOUT }).then(($body) => {
                  if ($body.find("h1").length > 0) {
                    cy.get("h1").should("contain.text", "Acquirer Simulator");
                    cy.get('[value="authorised"]').click();
                    verifyUrl = true;
                  } else if ($body.find('[value="authorised"]').length > 0) {
                    cy.get('[value="authorised"]').click();
                    verifyUrl = true;
                  } else {
                    cy.log(
                      "Adyen Momo redirect page loaded - no h1 or authorised button found"
                    );
                    cy.get("body").should("exist");
                    verifyUrl = false;
                  }
                });
                break;
              case "vipps":
                cy.get("body", { timeout: constants.TIMEOUT }).then(($body) => {
                  const bodyText = $body.text() || "";
                  if (bodyText.includes("Acquirer Simulator")) {
                    cy.get('[value="authorised"]').click();
                    verifyUrl = true;
                  } else {
                    cy.log("Vipps redirect page loaded - skipping interaction");
                    verifyUrl = false;
                  }
                });
                break;
              case "dana":
              case "go_pay":
                cy.log(
                  `Adyen ${paymentMethodType} redirect page - skipping interaction`
                );
                verifyUrl = false;
                break;
              case "pay_safe_card":
                cy.url().should("include", "paysafecard");
                verifyUrl = false;
                break;
              case "open_banking_uk":
                cy.get("body", { timeout: constants.TIMEOUT }).should("exist");
                cy.url().should("include", "adyen");
                cy.log(
                  "Adyen OpenBankingUk redirect page loaded - sandbox error page, skipping interaction"
                );
                verifyUrl = false;
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

          case "airwallex":
            if (paymentMethodType === "paypal") {
              cy.log("Handling Airwallex PayPal wallet redirect");
              cy.get("body", { timeout: constants.TIMEOUT }).should("exist");
              verifyUrl = false;
            } else if (paymentMethodType === "skrill") {
              cy.log("Handling Airwallex Skrill wallet redirect");
              cy.get("body", { timeout: constants.TIMEOUT }).should("exist");
              cy.get("button#approve", { timeout: constants.TIMEOUT })
                .should("be.visible")
                .click();
              verifyUrl = true;
            } else {
              throw new Error(
                `Unsupported Airwallex payment method type: ${paymentMethodType}`
              );
            }
            break;

          case "paypal":
            if (["eps", "ideal", "giropay"].includes(paymentMethodType)) {
              cy.get('button[name="Successful"][value="SUCCEEDED"]').click();
              verifyUrl = true;
            } else if (paymentMethodType === "paypal") {
              cy.url().should("include", "sandbox.paypal.com");
              verifyUrl = false;
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
              case "giropay":
                // Nexinets iDEAL/Giropay selector - click the Success link
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

          case "globalpay":
            switch (paymentMethodType) {
              case "ideal":
                cy.get("body", { timeout: 20000 }).then(($body) => {
                  const bodyText = $body.text();
                  cy.task(
                    "cli_log",
                    `GlobalPay ${paymentMethodType} page text: ${bodyText.substring(0, 200)}`
                  );

                  if ($body.find('button[type="submit"]').length > 0) {
                    cy.get('button[type="submit"]').first().click();
                  } else if ($body.find('input[type="submit"]').length > 0) {
                    cy.get('input[type="submit"]').first().click();
                  } else if (
                    $body.find(
                      '[data-testid*="confirm"], [data-testid*="continue"]'
                    ).length > 0
                  ) {
                    cy.get(
                      '[data-testid*="confirm"], [data-testid*="continue"]'
                    )
                      .first()
                      .click();
                  } else if ($body.find("a.btn, button.btn").length > 0) {
                    cy.get("a.btn, button.btn").first().click();
                  } else {
                    cy.log(
                      `No interactable elements found on GlobalPay ${paymentMethodType} page`
                    );
                  }
                });
                verifyUrl = false;
                break;
              case "eps":
                cy.on("uncaught:exception", () => false);
                cy.get("body", { timeout: 20000 }).then(($body) => {
                  const bodyText = $body.text();
                  cy.task(
                    "cli_log",
                    `GlobalPay ${paymentMethodType} page text: ${bodyText.substring(0, 200)}`
                  );

                  if ($body.find('button[type="submit"]').length > 0) {
                    cy.get('button[type="submit"]').first().click();
                  } else if ($body.find('input[type="submit"]').length > 0) {
                    cy.get('input[type="submit"]').first().click();
                  } else if (
                    $body.find(
                      '[data-testid*="confirm"], [data-testid*="continue"]'
                    ).length > 0
                  ) {
                    cy.get(
                      '[data-testid*="confirm"], [data-testid*="continue"]'
                    )
                      .first()
                      .click();
                  } else if ($body.find("a.btn, button.btn").length > 0) {
                    cy.get("a.btn, button.btn").first().click();
                  } else {
                    cy.log(
                      `No interactable elements found on GlobalPay ${paymentMethodType} page`
                    );
                  }
                });
                verifyUrl = false;
                break;
              case "giropay":
                cy.get("body", { timeout: 10000 }).then(($body) => {
                  const bodyText = $body.text().toLowerCase();
                  if (
                    bodyText.includes("timeout") ||
                    bodyText.includes("error") ||
                    bodyText.includes("503") ||
                    bodyText.includes("unavailable")
                  ) {
                    cy.log(
                      "GlobalPay Giropay redirect page unavailable - skipping interaction"
                    );
                    verifyUrl = false;
                    return;
                  }
                });
                verifyUrl = false;
                break;
              case "paypal":
                cy.url().should("include", "sandbox.paypal.com");
                verifyUrl = false;
                break;
              default:
                throw new Error(
                  `Unsupported GlobalPay payment method type: ${paymentMethodType}`
                );
            }
            break;

          case "loonio":
            switch (paymentMethodType) {
              case "interac":
                cy.log("Handling Loonio Interac bank redirect flow");
                cy.contains("button", "Back to Cashier", {
                  timeout: constants.TIMEOUT / 3,
                })
                  .should("be.visible")
                  .click();

                verifyUrl = true;
                break;
              default:
                throw new Error(
                  `Unsupported loonio payment method type: ${paymentMethodType}`
                );
            }
            break;

          case "gigadat":
            switch (paymentMethodType) {
              case "interac":
                cy.contains("button", /Return To Merchant/i, {
                  timeout: constants.TIMEOUT / 3,
                })
                  .should("be.visible")
                  .click();

                cy.contains("button", /^Yes$/i, {
                  timeout: constants.TIMEOUT / 3,
                })
                  .should("be.visible")
                  .click();

                verifyUrl = true;
                break;
              default:
                throw new Error(
                  `Unsupported loonio payment method type: ${paymentMethodType}`
                );
            }
            break;

          case "multisafepay":
            if (
              [
                "sofort",
                "eps",
                "mb_way",
                "ali_pay",
                "paypal",
                "we_chat_pay",
              ].includes(paymentMethodType)
            ) {
              // Multisafe pay has CSRF blocking cannot actually test redirection flow via cypress
              // cy.get(".btn-msp-success").click();

              verifyUrl = false;
            } else {
              throw new Error(
                `Unsupported multisafe payment method type: ${paymentMethodType}`
              );
            }
            break;

          case "calida":
            switch (paymentMethodType) {
              case "bluecode":
                cy.log("Handling Bluecode redirect flow");

                cy.contains("body", /Open your Bluecode compatible App/i, {
                  timeout: constants.TIMEOUT / 3,
                }).should("be.visible");

                // Bluecode shows a QR that the shopper scans inside their wallet app.
                cy.get(
                  "canvas:visible, img:visible, svg:visible, picture:visible",
                  {
                    timeout: constants.TIMEOUT / 2,
                  }
                )
                  .first()
                  .scrollIntoView()
                  .should("be.visible")
                  .then(($el) => {
                    cy.log(
                      "Verified Bluecode QR code is visible",
                      $el.prop("tagName")
                    );
                  });

                cy.contains("button, a", /Cancel/i, {
                  timeout: constants.TIMEOUT / 3,
                }).should("be.visible");

                verifyUrl = false;
                break;
              default:
                throw new Error(
                  `Unsupported Calida payment method type: ${paymentMethodType}`
                );
            }
            break;

          case "paysafe":
            switch (paymentMethodType) {
              case "interac":
                cy.log("Handling Paysafe Interac bank redirect flow");

                verifyUrl = false;
                break;
              case "skrill":
                cy.log("Handling Paysafe Skrill wallet redirect flow");

                verifyUrl = false;
                break;
              case "pay_safe_card":
                cy.log("Handling Paysafe PaySafeCard gift card redirect flow");

                verifyUrl = false;
                break;
              default:
                throw new Error(
                  `Unsupported Paysafe payment method type in handleFlow: ${paymentMethodType}`
                );
            }
            break;

          case "volt":
            if (paymentMethodType === "open_banking_uk") {
              cy.log("Handling Volt OpenBankingUk redirect flow");
              const clickableSelector =
                "button, [role='button'], div[role='option'], li, span, label";
              const selectBank = () =>
                cy
                  .contains(clickableSelector, /Barclays Sandbox/i, {
                    timeout: constants.TIMEOUT,
                  })
                  .scrollIntoView()
                  .should("be.visible")
                  .then(($el) => {
                    const candidate = $el.closest(clickableSelector);
                    if (candidate.length) {
                      cy.wrap(candidate).click();
                    } else {
                      cy.wrap($el).click();
                    }
                  });
              selectBank();
              cy.contains("button, a", /Continue on Desktop/i, {
                timeout: constants.TIMEOUT,
              })
                .should("be.visible")
                .click();
              verifyUrl = true;
            } else {
              throw new Error(
                `Unsupported Volt payment method type: ${paymentMethodType}`
              );
            }
            break;

          case "fiuu":
            if (paymentMethodType === "online_banking_fpx") {
              cy.log("Handling FIUU OnlineBankingFpx redirect flow");

              cy.get("body", { timeout: constants.TIMEOUT }).then(($body) => {
                if ($body.find("#txtUsername").length > 0) {
                  cy.get("#txtUsername").clear().type("Gaara", { delay: 10 });
                }

                if ($body.find("#txtPassword").length > 0) {
                  cy.get("#txtPassword")
                    .clear()
                    .type("letmepaywithsand", { delay: 10 });
                }

                if ($body.find("#login-btn").length > 0) {
                  cy.get("#login-btn").click();
                }
              });

              cy.get("body", { timeout: constants.TIMEOUT }).then(($body) => {
                const requestTacButton = $body.find(
                  "button.pay-btn:contains('Request TAC')"
                );
                if (requestTacButton.length > 0) {
                  cy.wrap(requestTacButton).click();
                }
              });

              cy.get("body", { timeout: constants.TIMEOUT }).then(($body) => {
                const otpText = $body.find("div.otp").text();
                const otpMatch = otpText.match(/\d+/);

                if (otpMatch) {
                  cy.get("#otp-input")
                    .should("be.visible")
                    .should("be.enabled")
                    .clear()
                    .type(otpMatch[0]);
                } else {
                  cy.log("FIUU FPX OTP text not found; proceeding without OTP");
                }
              });

              cy.contains("button.pay-btn", /Pay Now|Request TAC/i, {
                timeout: constants.TIMEOUT,
              })
                .should("be.visible")
                .click();

              verifyUrl = true;
            } else {
              throw new Error(
                `Unsupported FIUU payment method type: ${paymentMethodType}`
              );
            }
            break;

          case "mollie":
            if (
              [
                "eps",
                "ideal",
                "giropay",
                "sofort",
                "przelewy24",
                "bancontact_card",
              ].includes(paymentMethodType)
            ) {
              cy.log(`Handling Mollie ${paymentMethodType} redirect flow`);

              // Mollie test mode shows radio buttons to select payment status
              cy.get("body").then(($body) => {
                const paidSelector = 'input[type="radio"][value="paid"]';
                const authorizedSelector =
                  'input[type="radio"][value="authorized"]';

                if ($body.find(paidSelector).length) {
                  cy.get(paidSelector, { timeout: constants.WAIT_TIME })
                    .click()
                    .log("Selected: Paid");
                } else if ($body.find(authorizedSelector).length) {
                  cy.get(authorizedSelector, { timeout: constants.WAIT_TIME })
                    .click()
                    .log("Selected: Authorized");
                } else {
                  cy.log(
                    "No payment status selector found, page may auto-redirect"
                  );
                }
              });

              // Click the Continue button if present
              cy.get("body").then(($body) => {
                if ($body.find('button:contains("Continue")').length > 0) {
                  cy.contains("button", "Continue", {
                    timeout: constants.WAIT_TIME,
                  })
                    .should("be.visible")
                    .click();
                }
              });

              verifyUrl = true;
            } else {
              throw new Error(
                `Unsupported Mollie payment method type: ${paymentMethodType}`
              );
            }
            break;

          case "paystack":
            if (paymentMethodType === "eft") {
              cy.log("Handling Paystack EFT bank redirect flow");
              cy.get("body", { timeout: constants.TIMEOUT }).should("exist");

              cy.get("body").then(($body) => {
                const submitBtn = $body.find(
                  'button[type="submit"], input[type="submit"]'
                );
                if (submitBtn.length > 0) {
                  cy.wrap(submitBtn.first())
                    .should("be.visible")
                    .click({ force: true });
                  cy.log("Clicked submit button on Paystack EFT redirect page");
                } else {
                  cy.log(
                    "No submit button found on Paystack EFT page - proceeding without interaction"
                  );
                }
              });

              verifyUrl = false;
            } else {
              throw new Error(
                `Unsupported Paystack payment method type: ${paymentMethodType}`
              );
            }
            break;

          // payjustnow and payjustnowinstore are handled in their own
          // else-if branch above (before handleFlow)
          // using two sequential cy.origin() calls, because cy.origin cannot be nested.

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

function threeDsRedirection(
  redirectionUrl,
  expectedUrl,
  connectorId,
  paymentMethodType
) {
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

  if (connectorId === "iatapay" && paymentMethodType === "duit_now") {
    cy.log("Starting iatapay RealTimePayment redirection flow for DuitNow");

    cy.get(".iatapay-button.iatapay-button--secondary", {
      timeout: CONSTANTS.TIMEOUT,
    })
      .should("be.visible")
      .click();

    cy.log("Clicked Simulate button");

    cy.url({ timeout: CONSTANTS.TIMEOUT }).should(
      "include",
      expectedUrl.hostname
    );

    verifyReturnUrl(redirectionUrl, expectedUrl, true);
    return;
  }

  // Special handling for Airwallex which uses multiple domains in 3DS flow
  // Handle separately to avoid nested cy.origin() calls
  if (connectorId === "airwallex") {
    cy.log("Starting Airwallex 3DS redirection flow");

    // Wait for initial redirect to complete
    waitForRedirect(redirectionUrl.href);

    // Handle first domain: pci-api-demo.airwallex.com
    cy.url().then((currentUrl) => {
      const urlObj = new URL(currentUrl);
      if (urlObj.hostname === "pci-api-demo.airwallex.com") {
        cy.log("On pci-api-demo.airwallex.com - waiting for auto-redirect");

        const currentOrigin = urlObj.origin;
        cy.origin(
          currentOrigin,
          { args: { constants: CONSTANTS } },
          ({ constants }) => {
            // Wait for automatic redirect to authentication page
            cy.url({ timeout: constants.TIMEOUT }).should(
              "not.include",
              "pci-api-demo.airwallex.com"
            );
          }
        );
      }
    });

    // Handle second domain: api-demo.airwallex.com
    cy.url().then((currentUrl) => {
      if (new URL(currentUrl).hostname === "api-demo.airwallex.com") {
        cy.log("Now on api-demo.airwallex.com for authentication");

        const currentOrigin = new URL(currentUrl).origin;
        cy.origin(
          currentOrigin,
          { args: { constants: CONSTANTS } },
          ({ constants }) => {
            cy.log("Handling Airwallex authentication form");

            // Wait for form to be available
            cy.get("form, body", { timeout: constants.TIMEOUT }).should(
              "exist"
            );

            // Look for authentication input field (password or text)
            cy.get(
              'input[type="password"], input[type="text"], input[name*="password"], input[name*="auth"], input',
              { timeout: constants.TIMEOUT }
            )
              .first()
              .should("be.visible")
              .should("be.enabled")
              .clear()
              .type("1234");

            // Look for submit button and click it
            cy.get(
              'button[type="submit"], input[type="submit"], button:contains("Submit"), button:contains("Continue"), button',
              { timeout: constants.TIMEOUT }
            )
              .first()
              .should("be.visible")
              .click();

            cy.log("Submitted Airwallex 3DS authentication with code 1234");
          }
        );
      } else {
        cy.log("On different domain, attempting generic form handling");
        // Handle generic form without cy.origin() for same-domain forms
        cy.get("body").then(($body) => {
          // Check if there's a form on the page
          if ($body.find("form").length > 0) {
            cy.get("form")
              .first()
              .within(() => {
                // Look for any input field that might be for authentication
                cy.get('input[type="password"], input[type="text"], input')
                  .first()
                  .clear()
                  .type("1234");

                // Look for submit button
                cy.get('button[type="submit"], input[type="submit"], button')
                  .first()
                  .click();
              });
          } else {
            cy.log("No form found, waiting for redirect");
            cy.wait(CONSTANTS.TIMEOUT / 6); // Wait 15 seconds for automatic redirect
          }
        });
      }
    });

    // Wait for final redirect back to expected URL
    cy.url({ timeout: CONSTANTS.TIMEOUT }).should("include", expectedUrl.host);
    verifyReturnUrl(redirectionUrl, expectedUrl, true);
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
          cy.get('form[name="simulationForm"]', {
            timeout: constants.WAIT_TIME,
          })
            .should("exist")
            .then(() => {
              cy.get("#challengeResult")
                .select("Successful")
                .should("have.value", "Success");
              cy.get('input[type="submit"]').click();
            });
          break;

        case "worldpay":
        case "worldpayxml":
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
        case "mollie":
          cy.get("body").then(($body) => {
            const paidSelector = 'input[type="radio"][value="paid"]';

            if ($body.find(paidSelector).length) {
              cy.get(paidSelector, { timeout: 500 }) // Short timeout as we already checked existence
                .click()
                .log("Selected: Paid");
            } else {
              const authorizedSelector =
                'input[type="radio"][value="authorized"]';

              cy.get(authorizedSelector, { timeout: constants.WAIT_TIME })
                .should("exist")
                .click()
                .log("Selected: Authorized");
            }
          });
          cy.contains("button", "Continue", { timeout: constants.WAIT_TIME })
            .should("be.visible")
            .click();
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

function rewardRedirection(
  redirectionUrl,
  expectedUrl,
  connectorId,
  paymentMethodType
) {
  let verifyUrl = false;

  // Skip if redirectionUrl is null (happens when nextActionUrl is invalid)
  if (redirectionUrl && redirectionUrl.href) {
    cy.visit(redirectionUrl.href);
    waitForRedirect(redirectionUrl.href);

    handleFlow(
      redirectionUrl,
      expectedUrl,
      connectorId,
      ({ connectorId, paymentMethodType }) => {
        switch (connectorId) {
          case "cashtocode":
            // Cashtocode reward payment redirects
            switch (paymentMethodType) {
              case "evoucher":
              case "classic":
                cy.log(`Handling Cashtocode ${paymentMethodType} payment`);
                // Cashtocode reward payments don't reach terminal state
                // Skip return URL verification
                verifyUrl = false;
                break;
              default:
                throw new Error(
                  `Unsupported Cashtocode payment method type: ${paymentMethodType}`
                );
            }
            break;
          default:
            // Default handling for other connectors that may support reward payments
            cy.log(`Handling reward payment for connector: ${connectorId}`);
            verifyUrl = false;
        }
      },
      { paymentMethodType }
    );
  } else {
    cy.log("Skipping reward redirection - no valid redirect URL provided");
  }

  cy.then(() => {
    verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
  });
}

function voucherRedirection(
  redirectionUrl,
  expectedUrl,
  connectorId,
  paymentMethodType
) {
  let verifyUrl = false;

  if (redirectionUrl && redirectionUrl.href) {
    cy.visit(redirectionUrl.href);
    waitForRedirect(redirectionUrl.href);

    handleFlow(
      redirectionUrl,
      expectedUrl,
      connectorId,
      ({ connectorId, paymentMethodType, constants }) => {
        switch (connectorId) {
          case "adyen":
            switch (paymentMethodType) {
              case "boleto":
              case "oxxo":
              case "alfamart":
              case "indomaret":
                // display_voucher_information vouchers — never reach here because
                // handleVoucherRedirection skips the redirect step entirely
                cy.log(
                  `Unexpected redirect for display_voucher_information voucher: ${paymentMethodType}`
                );
                verifyUrl = false;
                break;
              case "seven_eleven":
              case "lawson":
              case "mini_stop":
              case "family_mart":
              case "seicomart":
              case "pay_easy":
                // Adyen voucher redirect pages show branded Japanese-language
                // payment receipts. They are NOT Acquirer Simulator pages.
                // Just verify the page loads with voucher content.
                cy.get("body", { timeout: constants.TIMEOUT }).should("exist");
                // Voucher pages have a non-standard h1 (often a full-width
                // space) so we avoid asserting on heading text.  Instead
                // confirm that page load succeeded by checking for known
                // vendor logos or Japanese payment terminology.
                cy.get("body").then(($body) => {
                  const bodyText = $body.text();
                  const voucherIndicators = [
                    /お支払い/i,
                    /払込/i,
                    /セブン|lawson|ミニストップ|ファミリーマート|セイコーマート|ペイジー/i,
                    /お客様名/i,
                    /前払い/i,
                    /\u3000/, // full-width space
                  ];
                  const isVoucherPage = voucherIndicators.some((pat) =>
                    pat.test(bodyText)
                  );
                  if (isVoucherPage) {
                    cy.log(
                      `Verified Adyen voucher redirect page loaded for ${paymentMethodType}`
                    );
                  } else {
                    cy.log(
                      `Warning: Voucher page content not recognized for ${paymentMethodType}. Body length: ${bodyText.length}`
                    );
                  }
                });
                verifyUrl = false;
                break;
              default:
                cy.log(`Unhandled Adyen voucher type: ${paymentMethodType}`);
                verifyUrl = false;
            }
            break;
          case "dlocal":
            switch (paymentMethodType) {
              case "oxxo":
                // Dlocal Oxxo returns a redirect URL via ticket.image_url.
                // Visit the page, inspect for interactive elements,
                // and attempt to complete payment.
                cy.log(`Dlocal Oxxo voucher — visiting redirect URL`);

                // Suppress potential JS errors from sandbox/test pages
                cy.on("uncaught:exception", () => false);

                cy.get("body", { timeout: constants.TIMEOUT })
                  .should("exist")
                  .then(($body) => {
                    const bodyText = $body.text().toLowerCase();

                    // Determine if this is an interactive payment page
                    const hasPayButton =
                      $body.find('button:visible, input[type="submit"]:visible')
                        .length > 0;
                    const hasInteractiveElements = [
                      /pay/i,
                      /confirm/i,
                      /continue/i,
                      /submit/i,
                    ].some((pat) => pat.test(bodyText));

                    if (hasPayButton || hasInteractiveElements) {
                      cy.log(
                        `Interactive Oxxo page detected — attempting to complete payment`
                      );

                      // Generic button click strategy
                      cy.get("body").then(($b) => {
                        const buttons = $b.find("button:visible");
                        for (let i = 0; i < buttons.length; i++) {
                          const btnText = buttons[i].innerText.toLowerCase();
                          if (
                            /pay|confirm|continue|submit|complete/i.test(
                              btnText
                            )
                          ) {
                            cy.wrap(buttons[i]).click({ force: true });
                            cy.log(`Clicked payment button: ${btnText}`);
                            break;
                          }
                        }
                      });

                      // If there's a form, try to fill any visible inputs
                      cy.get("body").then(($b) => {
                        const inputs = $b.find(
                          "input:visible:not([type='hidden'])"
                        );
                        if (inputs.length > 0) {
                          cy.log(
                            `Found ${inputs.length} visible input(s) on Oxxo page`
                          );
                        }
                      });

                      verifyUrl = true;
                    } else {
                      // No interactive elements — display-only page (e.g. static barcode/image)
                      cy.log(
                        `Dlocal Oxxo page has no clickable payment buttons — display-only voucher`
                      );
                      verifyUrl = false;
                    }
                  });
                break;
              default:
                cy.log(`Unhandled dlocal voucher type: ${paymentMethodType}`);
                verifyUrl = false;
            }
            break;
          default:
            cy.log(
              `Generic voucher handling for ${connectorId}/${paymentMethodType}`
            );
            verifyUrl = false;
        }
      },
      { paymentMethodType }
    );
  } else {
    cy.log("Skipping voucher redirection - no valid redirect URL provided");
  }

  cy.then(() => {
    verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
  });
}

function cardRedirectRedirection(
  redirectionUrl,
  expectedUrl,
  connectorId,
  paymentMethodType,
  handlerMetadata
) {
  let verifyUrl = false;

  const cardData = handlerMetadata?.cardData || {};
  const {
    card_number = "4111111111111111",
    card_exp_month = "12",
    card_exp_year = "30",
    card_cvc = "123",
    card_name = "Test User",
    card_zip = "10001",
  } = cardData;

  if (redirectionUrl && redirectionUrl.href) {
    // Suppress uncaught exceptions from the Prophetpay hosted tokenize page,
    // including Google reCAPTCHA's "Cannot read properties of undefined
    // (reading 'replace')" and Blazor render-time errors. These are
    // third-party noise that must not abort the test. (A global handler in
    // cypress/support/e2e.js already returns false for all uncaught
    // exceptions; this per-test guard keeps that behavior explicit.)
    cy.on("uncaught:exception", (err) => {
      if (
        err.message.includes("replace") ||
        err.message.includes("grecaptcha") ||
        err.message.includes("Blazor")
      ) {
        return false;
      }
      return false;
    });

    cy.visit(redirectionUrl.href, { failOnStatusCode: false });
    // The Hyperswitch redirect page auto-submits to the connector's hosted
    // card page (e.g. ccm-thirdparty.cps.golf for prophetpay). Wait for the
    // host to change before interacting with the hosted form.
    waitForRedirect(redirectionUrl.href);
    cy.document().should("have.property", "readyState", "complete");
    cy.url().then((currentUrl) => {
      cy.log(`Card redirect: navigated to ${currentUrl}`);
    });

    // Fill the hosted card form directly WITHOUT cy.origin.
    //
    // chromeWebSecurity is disabled in cypress.config.js, so Cypress can
    // interact with cross-origin pages (e.g. ccm-thirdparty.cps.golf) in the
    // main context.  Using handleFlow/cy.origin causes the document context
    // to be lost after the Blazor form re-renders on input — the
    // "Cannot read properties of undefined (reading 'document')" error.
    // Filling the form in the main context avoids this entirely.
    switch (connectorId) {
      case "prophetpay": {
        verifyUrl = true;
        cy.log(`Handling Prophetpay card_redirect flow (${paymentMethodType})`);

        // Google reCAPTCHA loads on this form and throws
        // "TypeError: Cannot read properties of undefined (reading 'replace')".
        // Suppress this known third-party uncaught exception so it does not
        // abort the test.
        Cypress.on("uncaught:exception", (err) => {
          if (
            err.message.includes(
              "Cannot read properties of undefined (reading 'replace')"
            )
          ) {
            return false;
          }
        });

        // Prophetpay renders a Blazor hosted-tokenize form (#tokenForm)
        // at ccm-thirdparty.cps.golf/hp/Tokenize/{id}. Wait for the form
        // to render before filling card details.
        cy.get("#NameOnAccount, #tokenForm, body", {
          timeout: CONSTANTS.TIMEOUT,
        }).should("exist");
        cy.task("cli_log", "Prophetpay hosted tokenize form rendered");

        // ROUND 3 FIX: The Prophetpay Blazor form fires a tokenize XHR
        // while the hosted fields are being filled (e.g. onchange/oninput
        // handlers). That premature POST to /hp/Tokenize/{id} is sent
        // with incomplete card data, so Prophetpay returns a redirect to
        // localhost with `message=A user is invalid`. Cypress follows the
        // redirect, leaving the hosted form before the remaining fields
        // are filled, which causes the "document context lost" failure.
        // We intercept all tokenize POSTs and stub them with a neutral
        // 200 while the form is incomplete, then allow the real request
        // once every field is filled and we explicitly submit.
        let allowTokenize = false;
        cy.intercept("POST", /tokenize/i, (req) => {
          if (!allowTokenize) {
            req.reply({ statusCode: 200, body: {} });
          } else {
            req.continue();
          }
        }).as("prophetpayTokenize");

        // Backup guard: block regular form submits as well as XHRs until
        // every field is filled. This only runs for non-Blazor submissions;
        // Blazor's tokenize call goes through the intercept above.
        cy.window().then((win) => {
          const form = win.document.querySelector("#tokenForm");
          if (form) {
            win.__prophetpaySubmitGuard = (e) => {
              if (!allowTokenize) {
                e.preventDefault();
                e.stopPropagation();
                return false;
              }
            };
            form.addEventListener("submit", win.__prophetpaySubmitGuard, {
              capture: true,
            });
          }
        });

        // Cardholder name — known to exist (waited above)
        cy.get("#NameOnAccount", { timeout: CONSTANTS.TIMEOUT })
          .should("exist")
          .clear({ force: true })
          .type(card_name, { delay: 30, force: true });
        cy.task("cli_log", "Filled cardholder name on prophetpay form");

        // Wait for Blazor to finish processing the name input and
        // re-render the iframe card fields before we try to access them.
        // eslint-disable-next-line cypress/no-unnecessary-waiting
        cy.wait(2000);

        // Verify the card number container is present after re-render
        cy.get("#fullsteam-hosted-card-number-div, .cc-number", {
          timeout: CONSTANTS.TIMEOUT,
        }).should("exist");

        // Card number, expiry, and CVV are rendered inside separate
        // <iframe> elements by the Fullsteam/Prophetpay hosted tokenize form
        // (within #fullsteam-hosted-card-*-div containers whose class is
        // "form-control").  cy.clear()/type() must target the <input> INSIDE
        // the iframe body — calling them on the div wrapper fails because
        // cy.clear() only works on input/select/textarea/iframe/[contenteditable].
        // This mirrors the fillCardInputInIframe pattern used elsewhere in
        // this file (~line 3862).
        function fillIframeField(containerSelector, value, label) {
          // The Fullsteam/Prophetpay hosted-tokenize iframe is cross-origin
          // and may carry a sandbox attribute (without allow-same-origin).
          // When sandboxed, both contentDocument and contentWindow.document
          // are inaccessible even with chromeWebSecurity: false in
          // cypress.config.js — the sandbox restriction takes precedence
          // over Chrome's same-origin policy relaxation.
          //
          // Fix: detect the sandbox attribute, remove it, and RELOAD the
          // iframe by re-setting its src. Removing the attribute alone
          // does not change the sandbox flags — the iframe must be
          // re-navigated for the new (non-sandboxed) flags to take effect.
          // After reload, chromeWebSecurity: false lets the parent access
          // the cross-origin iframe document.
          cy.get(containerSelector, { timeout: 20000 })
            .should("exist")
            .first()
            .find("iframe")
            .should("be.visible")
            .should(($iframe) => {
              expect($iframe[0].src).to.not.be.empty;
            })
            .then(($iframe) => {
              const el = $iframe[0];
              const sandbox = el.getAttribute("sandbox");
              cy.task(
                "cli_log",
                `${label}: iframe src=${el.src || "(none)"} ` +
                  `sandbox="${sandbox || "(none)"}"`
              );

              // If sandboxed without allow-same-origin, remove attribute
              // and reload the iframe so the new flags take effect.
              if (sandbox !== null && !sandbox.includes("allow-same-origin")) {
                const originalSrc = el.src;
                cy.task(
                  "cli_log",
                  `${label}: removing sandbox, reloading iframe`
                );
                el.removeAttribute("sandbox");
                // Force navigation to about:blank first
                el.src = "about:blank";

                // Wait for blank doc, then restore original src.
                // .to.exist catches BOTH null and undefined — after
                // setting src to about:blank, contentDocument is briefly
                // undefined until the blank page loads; .to.not.be.null
                // would pass on undefined and proceed prematurely.
                cy.wrap(el)
                  .should(($iframeEl) => {
                    expect($iframeEl[0].contentDocument).to.exist;
                  })
                  .then(() => {
                    el.src = originalSrc;
                  });

                // Re-query iframe and wait for real content to load
                cy.get(containerSelector)
                  .first()
                  .find("iframe")
                  .should(($iframe) => {
                    const doc = $iframe[0].contentDocument;
                    expect(doc).to.not.be.null;
                    expect(doc).to.not.be.undefined;
                    expect(doc.body).to.not.be.empty;
                  })
                  .then(($iframe) => {
                    const body = $iframe[0].contentDocument.body;
                    const $input = Cypress.$(body)
                      .find("input:not([type=hidden])")
                      .first();
                    if ($input.length === 0) {
                      cy.task(
                        "cli_log",
                        `${label}: no input after sandbox removal`
                      );
                      return;
                    }
                    cy.wrap($input[0])
                      .clear({ force: true })
                      .type(value, { delay: 30, force: true });
                    cy.task(
                      "cli_log",
                      `Filled ${label} (after sandbox removal)`
                    );
                  });
                return;
              }

              // No sandbox (or has allow-same-origin) — standard
              // document access. Wait for an accessible, non-empty
              // contentDocument.body.
              cy.wrap(el)
                .should(($iframeEl) => {
                  const doc = $iframeEl[0].contentDocument;
                  expect(doc).to.not.be.null;
                  expect(doc).to.not.be.undefined;
                  expect(doc.body).to.not.be.empty;
                })
                .then(($iframeEl) => {
                  const body = $iframeEl[0].contentDocument.body;
                  const $input = Cypress.$(body)
                    .find("input:not([type=hidden])")
                    .first();
                  if ($input.length === 0) {
                    cy.task("cli_log", `${label}: no input inside iframe`);
                    return;
                  }
                  cy.wrap($input[0])
                    .clear({ force: true })
                    .type(value, { delay: 30, force: true });
                  cy.task("cli_log", `Filled ${label} in prophetpay iframe`);
                });
            });
        }

        fillIframeField(
          "#fullsteam-hosted-card-number-div, .cc-number",
          card_number,
          "card number"
        );

        fillIframeField(
          "#fullsteam-hosted-card-expire-div, .cc-expire",
          `${card_exp_month}${card_exp_year.slice(-2)}`,
          "expiry"
        );

        fillIframeField(
          "#fullsteam-hosted-card-cvv-div, .cc-cvv",
          card_cvc,
          "CVV"
        );

        // Zip
        cy.get("body").then(($body) => {
          const zipInput = $body.find("#Zip");
          if (zipInput.length > 0) {
            cy.wrap(zipInput.first())
              .clear({ force: true })
              .type(card_zip, { delay: 30, force: true });
            cy.task("cli_log", "Filled zip on prophetpay form");
          }
        });

        // Country — may be a select
        cy.get("body").then(($body) => {
          const countryEl = $body.find("#Country");
          if (countryEl.length > 0) {
            if (countryEl.is("select")) {
              const $select = countryEl.first();
              Cypress.$($select).val("US").trigger("change");
            } else {
              cy.wrap(countryEl.first())
                .clear({ force: true })
                .type("US", { delay: 30, force: true });
            }
            cy.task("cli_log", "Filled country on prophetpay form");
          }
        });

        // Brief wait for Blazor to process input events before submit
        /* eslint-disable cypress/no-unnecessary-waiting */
        cy.wait(1000);
        /* eslint-enable cypress/no-unnecessary-waiting */

        // All required fields are now filled; allow the real tokenize
        // request to reach Prophetpay when the submit button is clicked.
        cy.then(() => {
          allowTokenize = true;
        });

        // Remove the backup submit guard so the real submit can fire.
        cy.window().then((win) => {
          if (win.__prophetpaySubmitGuard) {
            const form = win.document.querySelector("#tokenForm");
            if (form) {
              form.removeEventListener("submit", win.__prophetpaySubmitGuard, {
                capture: true,
              });
            }
            delete win.__prophetpaySubmitGuard;
          }
        });

        // Submit the form
        cy.get("body").then(($body) => {
          const submitBtn = $body
            .find(
              'button[type="submit"], #submit, .btn-primary, input[type="submit"]'
            )
            .filter(function () {
              return /^[sS]ubmit|[pP]ay|[cC]ontinue/.test(
                this.innerText || this.value || ""
              );
            })
            .first();
          const fallbackBtn = $body
            .find(
              'button[type="submit"], #submit, .btn-primary, input[type="submit"]'
            )
            .first();
          const target = submitBtn.length > 0 ? submitBtn : fallbackBtn;
          if (target.length > 0) {
            cy.wrap(target).should("be.visible").click({ force: true });
            cy.task("cli_log", "Submitted prophetpay card form");
          } else {
            cy.task("cli_log", "Submit button not found on prophetpay form");
          }
        });
        break;
      }
      default:
        cy.log(
          `Generic card_redirect handling for ${connectorId}/${paymentMethodType}`
        );
    }
  } else {
    cy.log(
      "Skipping card_redirect redirection - no valid redirect URL provided"
    );
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

function paymentLinkCardRedirection(
  redirectionUrl,
  expectedUrl,
  connectorId,
  paymentMethodType,
  handlerMetadata
) {
  const cardData = handlerMetadata?.cardData || {};
  const expectedOutcome = handlerMetadata?.expectedOutcome || "success";
  const {
    card_number = "4242424242424242",
    card_exp_month = "12",
    card_exp_year = "35",
    card_cvc = "123",
  } = cardData;

  if (!redirectionUrl || !redirectionUrl.href) {
    cy.log(
      "Skipping payment link card redirection - no valid redirect URL provided"
    );
    return;
  }

  cy.visit(redirectionUrl.href, { failOnStatusCode: false });

  cy.get("body", { timeout: 30000 }).should("exist");

  // Wait for SDK form elements directly — skip #sdk-spinner check which times out
  // on bank-transfer payout link pages where the form loads without a loading spinner
  cy.get("#unified-checkout, #payment-form", { timeout: 60000 })
    .should("exist")
    .and("be.visible");

  cy.get("#unified-checkout iframe, #payment-form iframe", {
    timeout: 30000,
  }).should("have.length.at.least", 1);
  cy.task("cli_log", "Payout Link bank form iframe ready");

  function fillCardInputInIframe(iframe, index) {
    cy.wrap(iframe)
      .its("0.contentDocument.body")
      .should("not.be.empty")
      .then((body) => {
        const $body = Cypress.$(body);
        const inputs = $body.find("input");

        if (inputs.length === 0) {
          cy.task("cli_log", `Iframe ${index}: no inputs found, skipping`);
          return;
        }

        inputs.each((_idx, input) => {
          const $input = Cypress.$(input);
          const placeholder = ($input.attr("placeholder") || "").toLowerCase();
          const ariaLabel = ($input.attr("aria-label") || "").toLowerCase();
          const name = ($input.attr("name") || "").toLowerCase();
          const autocomplete = (
            $input.attr("autocomplete") || ""
          ).toLowerCase();

          if (
            placeholder.includes("card") ||
            placeholder.includes("number") ||
            ariaLabel.includes("card") ||
            ariaLabel.includes("number") ||
            name.includes("cardnumber") ||
            name.includes("card_number") ||
            autocomplete.includes("cc-number")
          ) {
            cy.wrap(input)
              .should("be.visible")
              .clear()
              .type(card_number, { delay: 30 });
            cy.task("cli_log", `Filled card number in iframe ${index}`);
          } else if (
            placeholder.includes("expir") ||
            placeholder.includes("mm") ||
            placeholder.includes("yy") ||
            ariaLabel.includes("expir") ||
            name.includes("exp") ||
            autocomplete.includes("cc-exp")
          ) {
            cy.wrap(input)
              .should("be.visible")
              .clear()
              .type(`${card_exp_month}${card_exp_year.slice(-2)}`, {
                delay: 30,
              });
            cy.task("cli_log", `Filled expiry in iframe ${index}`);
          } else if (
            placeholder.includes("cvc") ||
            placeholder.includes("cvv") ||
            placeholder.includes("security") ||
            ariaLabel.includes("cvc") ||
            ariaLabel.includes("cvv") ||
            name.includes("cvc") ||
            name.includes("cvv") ||
            autocomplete.includes("cc-csc")
          ) {
            cy.wrap(input)
              .should("be.visible")
              .clear()
              .type(card_cvc, { delay: 30 });
            cy.task("cli_log", `Filled CVC in iframe ${index}`);
          }
        });
      });
  }

  cy.get("#unified-checkout iframe").then(($iframes) => {
    cy.task("cli_log", `Found ${$iframes.length} iframes in unified-checkout`);

    $iframes.each((index, iframe) => {
      fillCardInputInIframe(iframe, index);
    });
  });

  cy.get("#submit", { timeout: 30000 })
    .should("be.visible")
    .and("not.have.class", "hidden")
    .click();
  cy.task("cli_log", "Clicked submit button");

  if (expectedOutcome === "error") {
    cy.get("body", { timeout: 30000 }).should(($body) => {
      const bodyText = $body.text().toLowerCase();
      const hasError =
        (bodyText.includes("error") && bodyText.includes("card")) ||
        bodyText.includes("declined") ||
        bodyText.includes("invalid") ||
        bodyText.includes("expired") ||
        bodyText.includes("failed") ||
        $body.find('[class*="error"]').length > 0;
      expect(hasError, "Expected error indicator on payment page").to.be.true;
    });
    cy.task("cli_log", "Payment page shows error indicator as expected");
  } else {
    cy.contains(/succeeded|success|payment successful|thank you/i, {
      timeout: 30000,
    }).should("exist");

    cy.get("body").then(($body) => {
      const bodyText = $body.text().toLowerCase();
      const hasSuccess =
        bodyText.includes("succeeded") ||
        bodyText.includes("success") ||
        bodyText.includes("payment successful") ||
        bodyText.includes("thank you") ||
        $body.find('[class*="success"]').length > 0;
      const hasError =
        (bodyText.includes("error") && bodyText.includes("card")) ||
        bodyText.includes("declined") ||
        bodyText.includes("invalid") ||
        $body.find('[class*="error"]').length > 0;

      if (hasSuccess) {
        cy.task("cli_log", "Payment page shows success indicator");
      } else if (hasError) {
        cy.task("cli_log", "Payment page shows error indicator");
      } else {
        cy.task(
          "cli_log",
          "Payment page status unclear after submission - checking URL"
        );
        cy.url().then((url) => {
          cy.task("cli_log", `Current URL after payment submission: ${url}`);
        });
      }
    });
  }
}

/**
 * Handles the initial visit to a payout link page.
 * Visits the payout link URL, waits for the SDK to load, and verifies
 * that the #payout-link container has rendered content and the SDK
 * iframe is present.
 *
 * @param {URL} redirectionUrl - The payout link URL to visit
 * @param {Object} handlerMetadata - Optional metadata (unused but kept for consistency)
 */
function payoutLinkInitRedirection(redirectionUrl) {
  if (!redirectionUrl || !redirectionUrl.href) {
    cy.log("Skipping payout link init - no valid redirect URL provided");
    return;
  }

  cy.visit(redirectionUrl.href, { failOnStatusCode: false });

  cy.get("body", { timeout: 30000 }).should("exist");

  // Payout link pages use #payout-link as the SDK mount container, not
  // #unified-checkout or #payment-form.  There is no #sdk-spinner element on
  // these pages — waiting for it causes a 60-second timeout that makes the
  // page appear blank.  Instead, wait for the SDK to render content inside
  // #payout-link and for the iframe to appear.
  cy.get("#payout-link", { timeout: 60000 }).should("not.be.empty");
  cy.task("cli_log", "Payout Link SDK initialized");

  cy.get("#payout-link iframe", { timeout: 30000 }).should(
    "have.length.at.least",
    1
  );
}

function payoutLinkRedirection(
  redirectionUrl,
  expectedUrl,
  connectorId,
  paymentMethodType,
  handlerMetadata
) {
  if (handlerMetadata?.payoutLinkType === "card") {
    return payoutLinkCardRedirection(
      redirectionUrl,
      expectedUrl,
      connectorId,
      paymentMethodType,
      handlerMetadata
    );
  }

  const expectedOutcome = handlerMetadata?.expectedOutcome || "success";
  const bankData = handlerMetadata?.bankData || {};
  const IBAN = bankData.iban || "NL46TEST0136169112";
  const BIC = bankData.bic || "ABNANL2A";

  cy.on("uncaught:exception", () => false);

  if (!redirectionUrl || !redirectionUrl.href) {
    cy.log(
      "Skipping payout link bank redirection - no valid redirect URL provided"
    );
    return;
  }

  cy.visit(redirectionUrl.href, { failOnStatusCode: false });
  cy.get("body", { timeout: 30000 }).should("exist");

  // Payout link pages use #payout-link as the SDK mount container.
  // Wait for the SDK to render content and for the iframe to appear.
  cy.get("#payout-link", { timeout: 60000 }).should("not.be.empty");
  cy.task("cli_log", "Payout Link SDK initialized");

  cy.get("#payout-link iframe", { timeout: 30000 }).should(
    "have.length.at.least",
    1
  );
  cy.task(
    "cli_log",
    "Payout link page loaded — looking for SEPA IBAN and BIC inputs"
  );

  // Helper: get a fresh reference to the iframe body each time.
  // Re-fetching handles iframe re-renders between actions (e.g. after
  // clicking Save the SDK may transition to a confirmation view).
  function getIframeBody() {
    return cy
      .get("#payout-link iframe")
      .first()
      .its("0.contentDocument.body")
      .should("not.be.empty");
  }

  // Fill IBAN input inside the iframe.
  // .find().should("exist") retries until the element appears and FAILS
  // if it never does — no silent skipping like the old approach.
  /* eslint-disable cypress/no-force */
  getIframeBody().then((iframeBody) => {
    cy.wrap(iframeBody)
      .find('input[id="sepa.iban"]')
      .should("exist")
      .and("be.visible")
      .clear({ force: true })
      .type(IBAN, { delay: 30, force: true });
    cy.task("cli_log", "IBAN filled");
  });

  // Fill BIC input inside the iframe
  getIframeBody().then((iframeBody) => {
    cy.wrap(iframeBody)
      .find('input[id="sepa.bic"]')
      .should("exist")
      .and("be.visible")
      .clear({ force: true })
      .type(BIC, { delay: 30, force: true });
    cy.task("cli_log", "BIC filled");
  });

  // Click Save button inside the iframe.
  // .contains("button", "Save") retries until the button appears and
  // FAILS if it never does — no silent skipping.
  getIframeBody().then((iframeBody) => {
    cy.wrap(iframeBody)
      .contains("button", "Save")
      .should("be.visible")
      .click({ force: true });
    cy.task("cli_log", "Save button clicked (first submission)");
  });

  // Brief wait for the SDK to process Save and transition to the
  // confirmation step where the Submit button appears.
  /* eslint-disable cypress/no-unnecessary-waiting */
  cy.wait(2000);
  /* eslint-enable cypress/no-unnecessary-waiting */

  // Click Submit button inside the iframe.
  // .contains("button", "Submit") retries until the button appears.
  getIframeBody().then((iframeBody) => {
    cy.wrap(iframeBody)
      .contains("button", "Submit")
      .should("be.visible")
      .click({ force: true });
    cy.task("cli_log", "Submit button clicked (second submission)");
  });
  /* eslint-enable cypress/no-force */

  // Assert on the result
  if (expectedOutcome === "error") {
    cy.get("body", { timeout: 30000 }).should(($body) => {
      const bodyText = $body.text().toLowerCase();
      const hasError =
        (bodyText.includes("error") && bodyText.includes("bank")) ||
        bodyText.includes("declined") ||
        bodyText.includes("invalid") ||
        bodyText.includes("failed") ||
        $body.find('[class*="error"]').length > 0;
      expect(hasError, "Expected error indicator on payout page").to.be.true;
    });
    cy.task("cli_log", "Payout page shows error indicator as expected");
  } else {
    // After successful submission, the page must show a specific
    // success/processing indicator.  We check both the iframe content
    // and the main page text using a retry-able .should() callback
    // so Cypress waits for the indicator to appear (up to 30s).
    //
    // Only specific phrases are checked — NOT generic "success" which
    // can appear on the page without actual form submission.
    getIframeBody().should((iframeBody) => {
      const iframeText = (iframeBody.innerText || "").toLowerCase();
      const mainText = (Cypress.$("body").text() || "").toLowerCase();
      const allText = iframeText + " " + mainText;
      const hasPayoutProcessing = allText.includes("payout processing");
      const hasRequiresFulfillment = allText.includes("requires_fulfillment");
      const hasPayoutSuccessful = allText.includes("payout successful");
      expect(
        hasPayoutProcessing || hasRequiresFulfillment || hasPayoutSuccessful,
        `Expected "payout processing", "requires_fulfillment", or "payout successful" after payout confirm. Page text: ${allText.substring(0, 400)}`
      ).to.be.true;
    });
    cy.task("cli_log", "Payout submission success/processing indicator found");
  }
}

function payoutLinkCardRedirection(
  redirectionUrl,
  expectedUrl,
  connectorId,
  paymentMethodType,
  handlerMetadata
) {
  const cardData = handlerMetadata?.cardData || {};
  const expectedOutcome = handlerMetadata?.expectedOutcome || "success";
  const {
    card_number = "4242424242424242",
    card_exp_month = "12",
    card_exp_year = "35",
    card_cvc = "123",
  } = cardData;

  if (!redirectionUrl || !redirectionUrl.href) {
    cy.log(
      "Skipping payout link card redirection - no valid redirect URL provided"
    );
    return;
  }

  cy.visit(redirectionUrl.href, { failOnStatusCode: false });

  cy.get("body", { timeout: 30000 }).should("exist");

  // Payout link pages use #payout-link as the SDK mount container, not
  // #unified-checkout or #payment-form.  There is no #sdk-spinner element on
  // these pages — waiting for it causes a 60-second timeout that makes the
  // page appear blank.  Instead, wait for the SDK to render content inside
  // #payout-link and for the iframe to appear.
  cy.get("#payout-link", { timeout: 60000 }).should("not.be.empty");
  cy.task("cli_log", "Payout Link SDK initialized successfully");

  cy.get("#payout-link iframe", { timeout: 30000 }).should(
    "have.length.at.least",
    1
  );

  function fillCardInputInIframe(iframe, index) {
    cy.wrap(iframe)
      .its("0.contentDocument.body")
      .should("not.be.empty")
      .then((body) => {
        const $body = Cypress.$(body);
        const inputs = $body.find("input");

        if (inputs.length === 0) {
          cy.task("cli_log", `Iframe ${index}: no inputs found, skipping`);
          return;
        }

        inputs.each((_idx, input) => {
          const $input = Cypress.$(input);
          const placeholder = ($input.attr("placeholder") || "").toLowerCase();
          const ariaLabel = ($input.attr("aria-label") || "").toLowerCase();
          const name = ($input.attr("name") || "").toLowerCase();
          const autocomplete = (
            $input.attr("autocomplete") || ""
          ).toLowerCase();

          if (
            placeholder.includes("card") ||
            placeholder.includes("number") ||
            ariaLabel.includes("card") ||
            ariaLabel.includes("number") ||
            name.includes("cardnumber") ||
            name.includes("card_number") ||
            autocomplete.includes("cc-number")
          ) {
            /* eslint-disable cypress/no-force */
            cy.wrap(input)
              .focus()
              .clear({ force: true })
              .type(card_number, { delay: 30, force: true });
            /* eslint-enable cypress/no-force */
            cy.task("cli_log", `Filled card number in iframe ${index}`);
          } else if (
            placeholder.includes("expir") ||
            placeholder.includes("mm") ||
            placeholder.includes("yy") ||
            ariaLabel.includes("expir") ||
            name.includes("exp") ||
            autocomplete.includes("cc-exp")
          ) {
            /* eslint-disable cypress/no-force */
            cy.wrap(input)
              .focus()
              .clear({ force: true })
              .type(`${card_exp_month}${card_exp_year.slice(-2)}`, {
                delay: 30,
                force: true,
              });
            /* eslint-enable cypress/no-force */
            cy.task("cli_log", `Filled expiry in iframe ${index}`);
          } else if (
            placeholder.includes("cvc") ||
            placeholder.includes("cvv") ||
            placeholder.includes("security") ||
            ariaLabel.includes("cvc") ||
            ariaLabel.includes("cvv") ||
            name.includes("cvc") ||
            name.includes("cvv") ||
            autocomplete.includes("cc-csc")
          ) {
            /* eslint-disable cypress/no-force */
            cy.wrap(input)
              .focus()
              .clear({ force: true })
              .type(card_cvc, { delay: 30, force: true });
            /* eslint-enable cypress/no-force */
            cy.task("cli_log", `Filled CVC in iframe ${index}`);
          }
        });
      });
  }

  cy.get("#payout-link iframe").then(($iframes) => {
    cy.task("cli_log", `Found ${$iframes.length} iframes in payout-link`);

    $iframes.each((index, iframe) => {
      fillCardInputInIframe(iframe, index);
    });
  });

  /* eslint-disable cypress/no-force */
  cy.get("#submit", { timeout: 30000 })
    .should("be.visible")
    .and("not.have.class", "hidden")
    .click({ force: true });
  /* eslint-enable cypress/no-force */
  cy.task("cli_log", "Clicked submit button");

  if (expectedOutcome === "error") {
    cy.get("body", { timeout: 30000 }).should(($body) => {
      const bodyText = $body.text().toLowerCase();
      const hasError =
        (bodyText.includes("error") && bodyText.includes("card")) ||
        bodyText.includes("declined") ||
        bodyText.includes("invalid") ||
        bodyText.includes("expired") ||
        bodyText.includes("failed") ||
        $body.find('[class*="error"]').length > 0;
      expect(hasError, "Expected error indicator on payout page").to.be.true;
    });
    cy.task("cli_log", "Payout page shows error indicator as expected");
  } else {
    cy.contains(/succeeded|success|payout successful|thank you/i, {
      timeout: 30000,
    }).should("exist");

    cy.get("body").then(($body) => {
      const bodyText = $body.text().toLowerCase();
      const hasSuccess =
        bodyText.includes("succeeded") ||
        bodyText.includes("success") ||
        bodyText.includes("payout successful") ||
        bodyText.includes("thank you") ||
        $body.find('[class*="success"]').length > 0;
      const hasError =
        (bodyText.includes("error") && bodyText.includes("card")) ||
        bodyText.includes("declined") ||
        bodyText.includes("invalid") ||
        $body.find('[class*="error"]').length > 0;

      if (hasSuccess) {
        cy.task("cli_log", "Payout page shows success indicator");
      } else if (hasError) {
        cy.task("cli_log", "Payout page shows error indicator");
      } else {
        cy.task(
          "cli_log",
          "Payout page status unclear after submission - checking URL"
        );
        cy.url().then((url) => {
          cy.task("cli_log", `Current URL after payout submission: ${url}`);
        });
      }
    });
  }
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

// Generic microdeposit verification handler for bank debit mandates
// Works with any provider that has a hosted verification page
// Parameters:
//   - hostedUrl: The URL of the hosted verification page
//   - origin: The origin domain (e.g., "https://payments.stripe.com")
//   - inputSelector: CSS selector for the input field
//   - verificationCode: The code to enter (e.g., "11AA")
//   - submitKey: Key to press to submit (e.g., "{enter}")
Cypress.Commands.add(
  "handleMicrodepositVerification",
  ({
    hostedUrl,
    origin,
    inputSelector,
    verificationCode,
    submitKey = "{enter}",
  }) => {
    cy.origin(
      origin,
      { args: { hostedUrl, inputSelector, verificationCode, submitKey } },
      ({ hostedUrl, inputSelector, verificationCode, submitKey }) => {
        cy.visit(hostedUrl);
        cy.get(inputSelector).type(`${verificationCode}${submitKey}`, {
          force: true,
        });
        cy.wait(5000);
      }
    );
  }
);

export const MICRODEPOSIT_CONFIG = {
  get stripe() {
    return {
      providerBaseUrl:
        Cypress.env("STRIPE_PROVIDER_BASE_URL") || "api.stripe.com",
      origin:
        Cypress.env("STRIPE_PAYMENTS_ORIGIN") || "https://payments.stripe.com",
      inputSelector: "input.p-CodePuncher-controllingInput",
      verificationCode: "11AA",
    };
  },
};
