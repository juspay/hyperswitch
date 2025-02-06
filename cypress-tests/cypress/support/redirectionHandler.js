/* eslint-disable cypress/unsafe-to-chain-command */
/* eslint-disable cypress/no-unnecessary-waiting */
import jsQR from "jsqr";

// Define constants for wait times
const CONSTANTS = {
  TIMEOUT: 20000, // 20 seconds
  WAIT_TIME: 10000, // 10 seconds
  ERROR_PATTERNS: [
    /4\d{2}/,
    /5\d{2}/,
    /error/i,
    /invalid request/i,
    /server error/i,
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
        urls.redirection_url,
        urls.expected_url,
        connectorId,
        paymentMethodType
      );
      break;
    case "bank_transfer":
      bankTransferRedirection(
        urls.redirection_url,
        urls.expected_url,
        connectorId,
        paymentMethodType,
        handlerMetadata.next_action_type
      );
      break;
    case "three_ds":
      threeDsRedirection(urls.redirection_url, urls.expected_url, connectorId);
      break;
    case "upi":
      upiRedirection(
        urls.redirection_url,
        urls.expected_url,
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
                verifyReturnUrl(redirectionUrl, expectedUrl, true);
              // expected_redirection can be used here to handle other payment methods
            }
            break;
          default:
            verifyReturnUrl(redirectionUrl, expectedUrl, true);
        }
      });
      break;
    case "image_data_url":
      switch (connectorId) {
        case "itaubank":
          switch (paymentMethodType) {
            case "pix":
              fetchAndParseImageData(redirectionUrl).then((qrCodeData) => {
                expect(qrCodeData).to.contains("itau.com.br/pix/qr/v2"); // image data contains the following value
              });
              break;
            default:
              verifyReturnUrl(redirectionUrl, expectedUrl, true);
          }
          break;
        default:
          verifyReturnUrl(redirectionUrl, expectedUrl, true);
      }
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
  waitForRedirect(redirectionUrl.href);

  handleFlow(
    redirectionUrl,
    expectedUrl,
    connectorId,
    ({ connectorId, paymentMethodType, constants }) => {
      switch (connectorId) {
        case "adyen":
          switch (paymentMethodType) {
            case "eps":
              cy.get("h1").should("contain.text", "Acquirer Simulator");
              cy.get('[value="authorised"]').click();
              break;
            case "ideal":
              cy.get(":nth-child(4) > td > p").should(
                "contain.text",
                "Your Payment was Authorised/Refused/Cancelled (It may take up to five minutes to show on the Payment List)"
              );
              cy.get(".btnLink").click();
              cy.url().should("include", "status=succeeded");
              cy.wait(5000);
              break;
            case "giropay":
              cy.get(
                ".rds-cookies-overlay__allow-all-cookies-btn > .rds-button"
              ).click();
              cy.wait(5000);
              cy.get(".normal-3").should(
                "contain.text",
                "Bank suchen ‑ mit giropay zahlen."
              );
              cy.get("#bankSearch").type("giropay TestBank{enter}");
              cy.get(".normal-2 > div").click();
              cy.get('[data-testid="customerIban"]').type(
                "DE48499999601234567890"
              );
              cy.get('[data-testid="customerIdentification"]').type(
                "9123456789"
              );
              cy.get(":nth-child(3) > .rds-button").click();
              cy.get('[data-testid="onlineBankingPin"]').type("1234");
              cy.get(".rds-button--primary").click();
              cy.get(":nth-child(5) > .rds-radio-input-group__label").click();
              cy.get(".rds-button--primary").click();
              cy.get('[data-testid="photoTan"]').type("123456");
              cy.get(".rds-button--primary").click();
              cy.wait(5000);
              cy.url().should("include", "status=succeeded");
              cy.wait(5000);
              break;
            case "sofort":
              cy.get(".modal-overlay.modal-shown.in", {
                timeout: constants.TIMEOUT,
              }).then(($modal) => {
                // If modal is found, handle it
                if ($modal.length > 0) {
                  cy.get("button.cookie-modal-deny-all.button-tertiary")
                    .should("be.visible")
                    .should("contain", "Reject All")
                    .click({ multiple: true });
                  cy.get("div#TopBanks.top-banks-multistep")
                    .should("contain", "Demo Bank")
                    .as("btn")
                    .click();
                  cy.get("@btn").click();
                } else {
                  cy.get("input.phone").type("9123456789");
                  cy.get("#button.onContinue")
                    .should("contain", "Continue")
                    .click();
                }
              });
              break;
            default:
              throw new Error(
                `Unsupported payment method type: ${paymentMethodType}`
              );
          }
          verifyUrl = true;
          break;
        case "paypal":
          if (["eps", "ideal", "giropay"].includes(paymentMethodType)) {
            cy.get('button[name="Successful"][value="SUCCEEDED"]').click();
            verifyUrl = true;
          } else {
            throw new Error(
              `Unsupported payment method type: ${paymentMethodType}`
            );
          }
          verifyUrl = true;
          break;
        case "stripe":
          if (
            ["eps", "ideal", "giropay", "sofort", "przelewy24"].includes(
              paymentMethodType
            )
          ) {
            cy.get('a[name="success"]').click();
            verifyUrl = true;
          } else {
            throw new Error(
              `Unsupported payment method type: ${paymentMethodType}`
            );
          }
          verifyUrl = true;
          break;
        case "trustpay":
          switch (paymentMethodType) {
            case "eps":
              cy.get("#bankname").type(
                "Allgemeine Sparkasse Oberösterreich Bank AG (ASPKAT2LXXX / 20320)"
              );
              cy.get("#selectionSubmit").click();
              cy.get("#user")
                .should("be.visible")
                .should("be.enabled")
                .focus()
                .type("Verfügernummer");
              cy.get("input#submitButton.btn.btn-primary").click();
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
                `Unsupported payment method type: ${paymentMethodType}`
              );
          }
          verifyUrl = false;
          break;
        default:
          throw new Error(`Unsupported connector: ${connectorId}`);
      }
    },
    { paymentMethodType }
  );

  cy.then(() => {
    verifyReturnUrl(redirectionUrl, expectedUrl, verifyUrl);
  });
}

function threeDsRedirection(redirectionUrl, expectedUrl, connectorId) {
  cy.visit(redirectionUrl.href);
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
  if (!forwardFlow) return;

  cy.location("host", { timeout: CONSTANTS.TIMEOUT }).should((currentHost) => {
    expect(currentHost).to.equal(expectedUrl.host);
  });

  cy.url().then((url) => {
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
          if (redirectionHost.endsWith(expectedHost)) {
            cy.wait(constants.WAIT_TIME / 2);

            cy.window()
              .its("location")
              .then((location) => {
                // Check page state before taking screenshots
                cy.document().then((doc) => {
                  const pageText = doc.body.innerText.toLowerCase();
                  if (!pageText) {
                    // eslint-disable-next-line cypress/assertion-before-screenshot
                    cy.screenshot("blank-page-error");
                  } else if (
                    constants.ERROR_PATTERNS.some((pattern) =>
                      pattern.test(pageText)
                    )
                  ) {
                    // eslint-disable-next-line cypress/assertion-before-screenshot
                    cy.screenshot(`error-page-${Date.now()}`);
                  }
                });

                const urlParams = new URLSearchParams(location.search);
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
              });
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
  const originalHost = new URL(redirectionUrl.href).host;
  cy.location("host", { timeout: CONSTANTS.TIMEOUT }).then((currentHost) => {
    if (currentHost !== originalHost) {
      // Regular redirection flow - host changed, use cy.origin()
      cy.url().then((currentUrl) => {
        cy.origin(
          new URL(currentUrl).origin,
          {
            args: {
              connectorId,
              constants: CONSTANTS,
              expectedUrl: expectedUrl.origin,
              ...options, // optional params like paymentMethodType
            },
          },
          callback
        );
      });
    } else {
      // Embedded flow - host unchanged, use cy.get("iframe")
      cy.get("iframe", { timeout: CONSTANTS.TIMEOUT })
        .should("be.visible")
        .should("exist");

      // Execute callback within the iframe context
      callback({
        connectorId,
        constants: CONSTANTS,
        expectedUrl: expectedUrl.origin,
        ...options, // optional params like paymentMethodType
      });
    }
  });
}
