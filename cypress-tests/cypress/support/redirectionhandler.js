import jsQR from "jsqr";

export function handleRedirection(
  redirection_type,
  urls,
  connectorId,
  payment_method_type
) {
  switch (redirection_type) {
    case "three_ds":
      threeDsRedirection(urls.redirection_url, urls.expected_url, connectorId);
      break;
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
  payment_method_type
) {
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
}

function threeDsRedirection(redirection_url, expected_url, connectorId) {
  cy.visit(redirection_url.href);
  if (connectorId == "adyen") {
    cy.get("iframe")
      .its("0.contentDocument.body")
      .within((body) => {
        cy.get('input[type="password"]').click();
        cy.get('input[type="password"]').type("password");
        cy.get("#buttonSubmit").click();
      });
  } else if (connectorId === "bankofamerica" || connectorId === "cybersource") {
    cy.get("iframe", { timeout: 15000 })
      .its("0.contentDocument.body")
      .within((body) => {
        cy.get('input[type="text"]').click().type("1234");
        cy.get('input[value="SUBMIT"]').click();
      });
  } else if (connectorId === "nmi" || connectorId === "noon") {
    cy.get("iframe", { timeout: 150000 })
      .its("0.contentDocument.body")
      .within((body) => {
        cy.get("iframe", { timeout: 20000 })
          .its("0.contentDocument.body")
          .within((body) => {
            cy.get('form[name="cardholderInput"]', { timeout: 20000 })
              .should("exist")
              .then((form) => {
                cy.get('input[name="challengeDataEntry"]').click().type("1234");
                cy.get('input[value="SUBMIT"]').click();
              });
          });
      });
  } else if (connectorId === "stripe") {
    cy.get("iframe", { timeout: 15000 })
      .its("0.contentDocument.body")
      .within((body) => {
        cy.get("iframe")
          .its("0.contentDocument.body")
          .within((body) => {
            cy.get("#test-source-authorize-3ds").click();
          });
      });
  } else if (connectorId === "trustpay") {
    cy.get('form[name="challengeForm"]', { timeout: 10000 })
      .should("exist")
      .then((form) => {
        cy.get("#outcomeSelect").select("Approve").should("have.value", "Y");
        cy.get('button[type="submit"]').click();
      });
  } else {
    // If connectorId is neither of adyen, trustpay, nmi, stripe, bankofamerica or cybersource, wait for 10 seconds
    cy.wait(10000);
  }

  verifyReturnUrl(redirection_url, expected_url, true);
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
          cy.get('[data-testid="customerIdentification"]').type("1234567890");
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
          cy.get(".modal-overlay.modal-shown.in", { timeout: 5000 }).then(
            ($modal) => {
              // If modal is found, handle it
              if ($modal.length > 0) {
                cy.get("button.cookie-modal-deny-all.button-tertiary")
                  .should("be.visible")
                  .should("contain", "Reject All")
                  .click({ force: true, multiple: true });
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
    case "trustpay":
      switch (payment_method_type) {
        case "eps":
          cy.get("._transactionId__header__iXVd_").should(
            "contain.text",
            "Bank suchen ‑ mit eps zahlen."
          );
          cy.get(".BankSearch_searchInput__uX_9l").type(
            "Allgemeine Sparkasse Oberösterreich Bank AG{enter}"
          );
          cy.get(".BankSearch_searchResultItem__lbcKm").click();
          cy.get("._transactionId__primaryButton__nCa0r").click();
          cy.get("#loginTitle").should(
            "contain.text",
            "eps Online-Überweisung Login"
          );
          cy.get("#user")
            .should("be.visible")
            .should("be.enabled")
            .focus()
            .type("Verfügernummer");
          cy.get("input#submitButton.btn.btn-primary").click();
          break;
        case "ideal":
          cy.get("p").should(
            "contain.text",
            "Choose your iDeal Issuer Bank please"
          );
          cy.get("#issuerSearchInput").click();
          cy.get("#issuerSearchInput").type("ING{enter}");
          cy.get("#trustpay__selectIssuer_submit").click();
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
  verifyReturnUrl(redirection_url, expected_url, verifyUrl);
}

function verifyReturnUrl(redirection_url, expected_url, forward_flow) {
  if (forward_flow) {
    // Handling redirection
    if (redirection_url.host.endsWith(expected_url.host)) {
      // No CORS workaround needed
      cy.window().its("location.origin").should("eq", expected_url.origin);
    } else {
      // Workaround for CORS to allow cross-origin iframe
      cy.origin(
        expected_url.origin,
        { args: { expected_url: expected_url.origin } },
        ({ expected_url }) => {
          cy.window().its("location.origin").should("eq", expected_url);
        }
      );
    }
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
