/* eslint-disable cypress/unsafe-to-chain-command */
/* eslint-disable cypress/no-unnecessary-waiting */
import jsQR from "jsqr";

// Define constants for wait times
const TIMEOUT = 20000; // 20 seconds
const WAIT_TIME = 10000; // 10 seconds

export function handleRedirection(
  redirection_type,
  urls,
  connectorId,
  payment_method_type,
  handler_metadata
) {
  switch (redirection_type) {
    case "bank_redirect":
      bankRedirectRedirection(
        urls.redirection_url,
        urls.expected_url,
        connectorId,
        payment_method_type
      );
      break;
    case "bank_transfer":
      bankTransferRedirection(
        urls.redirection_url,
        urls.expected_url,
        connectorId,
        payment_method_type,
        handler_metadata.next_action_type
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
        payment_method_type
      );
      break;
    default:
      throw new Error(`Redirection known: ${redirection_type}`);
  }
}

function bankTransferRedirection(
  redirection_url,
  expected_url,
  connectorId,
  payment_method_type,
  next_action_type
) {
  switch (next_action_type) {
    case "qr_code_url":
      cy.request(redirection_url.href).then((response) => {
        switch (connectorId) {
          case "adyen":
            switch (payment_method_type) {
              case "pix":
                expect(response.status).to.eq(200);
                fetchAndParseQRCode(redirection_url.href).then((qrCodeData) => {
                  expect(qrCodeData).to.eq("TestQRCodeEMVToken");
                });
                break;
              default:
                verifyReturnUrl(redirection_url, expected_url, true);
              // expected_redirection can be used here to handle other payment methods
            }
            break;
          default:
            verifyReturnUrl(redirection_url, expected_url, true);
        }
      });
      break;
    case "image_data_url":
      switch (connectorId) {
        case "itaubank":
          switch (payment_method_type) {
            case "pix":
              fetchAndParseImageData(redirection_url).then((qrCodeData) => {
                expect(qrCodeData).to.contains("itau.com.br/pix/qr/v2"); // image data contains the following value
              });
              break;
            default:
              verifyReturnUrl(redirection_url, expected_url, true);
          }
          break;
        default:
          verifyReturnUrl(redirection_url, expected_url, true);
      }
      break;
    default:
      verifyReturnUrl(redirection_url, expected_url, true);
  }
}

function bankRedirectRedirection(
  redirection_url,
  expected_url,
  connectorId,
  payment_method_type
) {
  let verifyUrl = false;
  cy.visit(redirection_url.href);

  switch (connectorId) {
    case "adyen":
      switch (payment_method_type) {
        case "eps":
          cy.get("h1").should("contain.text", "Acquirer Simulator");
          cy.get('[value="authorised"]').click();
          cy.url().should("include", "status=succeeded");
          cy.wait(5000);
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
          cy.get('[data-testid="customerIban"]').type("DE48499999601234567890");
          cy.get('[data-testid="customerIdentification"]').type("9123456789");
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
          cy.get(".modal-overlay.modal-shown.in", { timeout: TIMEOUT }).then(
            ($modal) => {
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
            }
          );
          break;
        case "trustly":
          break;
        default:
          throw new Error(
            `Unsupported payment method type: ${payment_method_type}`
          );
      }
      verifyUrl = true;
      break;
    case "paypal":
      switch (payment_method_type) {
        case "eps":
          cy.get('button[name="Successful"][value="SUCCEEDED"]').click();
          break;
        case "ideal":
          cy.get('button[name="Successful"][value="SUCCEEDED"]').click();
          break;
        case "giropay":
          cy.get('button[name="Successful"][value="SUCCEEDED"]').click();
          break;
        default:
          throw new Error(
            `Unsupported payment method type: ${payment_method_type}`
          );
      }
      verifyUrl = true;
      break;
    case "stripe":
      switch (payment_method_type) {
        case "eps":
          cy.get('a[name="success"]').click();
          break;
        case "ideal":
          cy.get('a[name="success"]').click();
          break;
        case "giropay":
          cy.get('a[name="success"]').click();
          break;
        case "sofort":
          cy.get('a[name="success"]').click();
          break;
        case "przelewy24":
          cy.get('a[name="success"]').click();
          break;
        default:
          throw new Error(
            `Unsupported payment method type: ${payment_method_type}`
          );
      }
      verifyUrl = true;
      break;
    case "trustpay":
      switch (payment_method_type) {
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
        case "sofort":
          break;
        case "trustly":
          break;
        default:
          throw new Error(
            `Unsupported payment method type: ${payment_method_type}`
          );
      }
      verifyUrl = false;
      break;
    default:
      throw new Error(`Unsupported connector: ${connectorId}`);
  }

  cy.then(() => {
    verifyReturnUrl(redirection_url, expected_url, verifyUrl);
  });
}

function threeDsRedirection(redirection_url, expected_url, connectorId) {
  cy.visit(redirection_url.href);

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
      cy.get("iframe", { timeout: TIMEOUT })
        .should("be.visible")
        .its("0.contentDocument.body")
        .should("not.be.empty")
        .within(() => {
          cy.get(
            'input[type="text"], input[type="password"], input[name="challengeDataEntry"]',
            { timeout: TIMEOUT }
          )
            .should("be.visible")
            .should("be.enabled")
            .click()
            .type("1234");

          cy.get('input[value="SUBMIT"], button[type="submit"]', {
            timeout: TIMEOUT,
          })
            .should("be.visible")
            .click();
        });
      break;

    case "cybersource":
      cy.url({ timeout: TIMEOUT }).should("include", expected_url.origin);
      break;

    case "checkout":
      cy.get("iframe", { timeout: TIMEOUT })
        .its("0.contentDocument.body")
        .within(() => {
          cy.get('form[id="form"]', { timeout: WAIT_TIME })
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
      cy.get("iframe", { timeout: TIMEOUT })
        .its("0.contentDocument.body")
        .within(() => {
          cy.get("iframe", { timeout: TIMEOUT })
            .its("0.contentDocument.body")
            .within(() => {
              cy.get('form[name="cardholderInput"]', { timeout: TIMEOUT })
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
      cy.get("form", { timeout: WAIT_TIME })
        .should("exist")
        .then(() => {
          cy.get('input[id="submit"]').click();
        });
      break;

    case "stripe":
      cy.get("iframe", { timeout: TIMEOUT })
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
      cy.get('form[name="challengeForm"]', { timeout: WAIT_TIME })
        .should("exist")
        .then(() => {
          cy.get("#outcomeSelect").select("Approve").should("have.value", "Y");
          cy.get('button[type="submit"]').click();
        });
      break;

    case "worldpay":
      cy.get("iframe", { timeout: WAIT_TIME })
        .its("0.contentDocument.body")
        .within(() => {
          cy.get('form[name="cardholderInput"]', { timeout: WAIT_TIME })
            .should("exist")
            .then(() => {
              cy.get('input[name="challengeDataEntry"]').click().type("1234");
              cy.get('input[value="SUBMIT"]').click();
            });
        });
      break;

    case "fiuu":
      cy.get('form[id="cc_form"]', { timeout: TIMEOUT })
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
      cy.wait(WAIT_TIME);
  }

  // Verify return URL after handling the specific connector
  verifyReturnUrl(redirection_url, expected_url, true);
}

function upiRedirection(
  redirection_url,
  expected_url,
  connectorId,
  payment_method_type
) {
  let verifyUrl = false;
  if (connectorId === "iatapay") {
    switch (payment_method_type) {
      case "upi_collect":
        cy.visit(redirection_url.href);
        cy.wait(TIMEOUT).then(() => {
          verifyUrl = true;
        });
        break;
      case "upi_intent":
        cy.request(redirection_url.href).then((response) => {
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
          `Unsupported payment method type: ${payment_method_type}`
        );
    }
  } else {
    // If connectorId is not iatapay, wait for 10 seconds
    cy.wait(WAIT_TIME);
  }

  cy.then(() => {
    verifyReturnUrl(redirection_url, expected_url, verifyUrl);
  });
}

function verifyReturnUrl(redirection_url, expected_url, forward_flow) {
  if (!forward_flow) return;

  try {
    if (redirection_url.host.endsWith(expected_url.host)) {
      cy.wait(WAIT_TIME / 2);

      cy.window()
        .its("location")
        .then((location) => {
          // Check page state before taking screenshots
          cy.document().then((doc) => {
            // For blank page
            cy.wrap(doc.body.innerText.trim()).then((text) => {
              if (text === "") {
                cy.wrap(text).should("eq", "");
                cy.screenshot("blank-page-error");
              }
            });

            // For error pages
            const errorPatterns = [
              /4\d{2}/,
              /5\d{2}/,
              /error/i,
              /invalid request/i,
              /server error/i,
            ];

            const pageText = doc.body.innerText.toLowerCase();
            cy.wrap(pageText).then((text) => {
              if (errorPatterns.some((pattern) => pattern.test(text))) {
                cy.wrap(text).should((content) => {
                  expect(errorPatterns.some((pattern) => pattern.test(content)))
                    .to.be.true;
                });
                cy.screenshot(`error-page-${Date.now()}`);
              }
            });
          });

          const url_params = new URLSearchParams(location.search);
          const payment_status = url_params.get("status");

          if (
            payment_status !== "failed" &&
            payment_status !== "processing" &&
            payment_status !== "requires_capture" &&
            payment_status !== "succeeded"
          ) {
            cy.wrap(payment_status).should("exist");
            cy.screenshot(`failed-payment-${payment_status}`);
            throw new Error(
              `Redirection failed with payment status: ${payment_status}`
            );
          }
        });
    } else {
      cy.origin(
        expected_url.origin,
        { args: { expected_url: expected_url.origin } },
        ({ expected_url }) => {
          cy.window().its("location.origin").should("eq", expected_url);

          Cypress.on("uncaught:exception", (err, runnable) => {
            // Log the error details
            // eslint-disable-next-line no-console
            console.log(
              `Error: ${err.message}\nError occurred in: ${runnable.title}\nStack trace: ${err.stack}`
            );
            // Return false to prevent the error from failing the test
            return false;
          });
        }
      );
    }
  } catch (error) {
    cy.error("Redirection verification failed:", error);
    throw error;
  }
}

async function fetchAndParseQRCode(url) {
  const response = await fetch(url, { encoding: "binary" });
  if (!response.ok) {
    throw new Error(`Failed to fetch QR code image: ${response.statusText}`);
  }
  const blob = await response.blob();
  const reader = new FileReader();
  return await new Promise((resolve, reject) => {
    reader.onload = () => {
      const base64Image = reader.result.split(",")[1]; // Remove data URI prefix
      const image = new Image();
      image.src = base64Image;

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
    };
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
