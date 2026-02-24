import * as fixtures from "../../../fixtures/imports";
import { generateRandomName } from "../../../utils/RequestBodyUtils";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - SaveCard payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("Save card for NoThreeDS automatic capture payment- Create+Confirm [on_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    cy.step("Create Customer", () =>
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
    );

    const createConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSAutoCapture"];

    cy.step("Create and Confirm Payment", () =>
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        createConfirmData,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(createConfirmData)) return;

    cy.step("Retrieve Payment after Confirm", () =>
      cy.retrievePaymentCallTest({ globalState, data: createConfirmData })
    );

    cy.step("List Customer Payment Methods", () =>
      cy.listCustomerPMCallTest(globalState)
    );

    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntent"];

    cy.step("Create Payment Intent", () =>
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        paymentIntentData,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const saveCardConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSAutoCapture"];

    cy.step("Save Card Confirm Call", () =>
      cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState)
    );
  });

  it("Save card for NoThreeDS manual full capture payment- Create+Confirm [on_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    cy.step("Create Customer", () =>
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
    );

    const createConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSAutoCapture"];

    cy.step("Create and Confirm Payment", () =>
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        createConfirmData,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(createConfirmData)) return;

    cy.step("Retrieve Payment after Confirm", () =>
      cy.retrievePaymentCallTest({ globalState, data: createConfirmData })
    );

    cy.step("List Customer Payment Methods", () =>
      cy.listCustomerPMCallTest(globalState)
    );

    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntent"];

    cy.step("Create Payment Intent", () =>
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        paymentIntentData,
        "no_three_ds",
        "manual",
        globalState
      )
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const saveCardConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSManualCapture"];

    cy.step("Save Card Confirm Call", () =>
      cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState)
    );

    if (!utils.should_continue_further(saveCardConfirmData)) return;

    cy.step("Retrieve Payment after Save Card Confirm", () =>
      cy.retrievePaymentCallTest({ globalState, data: saveCardConfirmData })
    );

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.step("Capture Payment", () =>
      cy.captureCallTest(fixtures.captureBody, captureData, globalState)
    );

    if (!utils.should_continue_further(captureData)) return;

    cy.step("Retrieve Payment after Capture", () =>
      cy.retrievePaymentCallTest({ globalState, data: captureData })
    );
  });

  it("Save card for NoThreeDS manual partial capture payment- Create + Confirm [on_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    cy.step("Create Customer", () =>
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
    );

    const createConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSAutoCapture"];

    cy.step("Create and Confirm Payment", () =>
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        createConfirmData,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(createConfirmData)) return;

    cy.step("Retrieve Payment after Confirm", () =>
      cy.retrievePaymentCallTest({ globalState, data: createConfirmData })
    );

    cy.step("List Customer Payment Methods", () =>
      cy.listCustomerPMCallTest(globalState)
    );

    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntent"];

    cy.step("Create Payment Intent", () =>
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        paymentIntentData,
        "no_three_ds",
        "manual",
        globalState
      )
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const saveCardConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSManualCapture"];

    cy.step("Save Card Confirm Call", () =>
      cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState)
    );

    if (!utils.should_continue_further(saveCardConfirmData)) return;

    cy.step("Retrieve Payment after Save Card Confirm", () =>
      cy.retrievePaymentCallTest({ globalState, data: saveCardConfirmData })
    );

    const partialCaptureData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PartialCapture"];

    cy.step("Partial Capture Payment", () =>
      cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState)
    );

    if (!utils.should_continue_further(partialCaptureData)) return;

    cy.step("Retrieve Payment after Partial Capture", () =>
      cy.retrievePaymentCallTest({ globalState, data: partialCaptureData })
    );
  });

  it("Save card for NoThreeDS automatic capture payment [off_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    cy.step("Create Customer", () =>
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
    );

    const createConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSAutoCaptureOffSession"];

    cy.step("Create and Confirm Payment", () =>
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        createConfirmData,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(createConfirmData)) return;

    cy.step("Retrieve Payment after Confirm", () =>
      cy.retrievePaymentCallTest({ globalState, data: createConfirmData })
    );

    cy.step("List Customer Payment Methods", () =>
      cy.listCustomerPMCallTest(globalState)
    );

    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntentOffSession"];

    cy.step("Create Payment Intent", () =>
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        paymentIntentData,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const saveCardConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardConfirmAutoCaptureOffSession"];

    cy.step("Save Card Confirm Call", () =>
      cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState)
    );
  });

  it("Save card for NoThreeDS manual capture payment- Create+Confirm [off_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    cy.step("Create Customer", () =>
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
    );

    const createConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSManualCaptureOffSession"];

    cy.step("Create and Confirm Payment", () =>
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        createConfirmData,
        "no_three_ds",
        "manual",
        globalState
      )
    );

    if (!utils.should_continue_further(createConfirmData)) return;

    cy.step("Retrieve Payment after Confirm", () =>
      cy.retrievePaymentCallTest({ globalState, data: createConfirmData })
    );

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.step("Capture Payment", () =>
      cy.captureCallTest(fixtures.captureBody, captureData, globalState)
    );

    if (!utils.should_continue_further(captureData)) return;

    cy.step("Retrieve Payment after Capture", () =>
      cy.retrievePaymentCallTest({ globalState, data: captureData })
    );

    cy.step("List Customer Payment Methods", () =>
      cy.listCustomerPMCallTest(globalState)
    );

    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntentOffSession"];

    cy.step("Create Payment Intent", () =>
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        paymentIntentData,
        "no_three_ds",
        "manual",
        globalState
      )
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const saveCardConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardConfirmManualCaptureOffSession"];

    cy.step("Save Card Confirm Call", () =>
      cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState)
    );

    if (!utils.should_continue_further(saveCardConfirmData)) return;

    cy.step("Retrieve Payment after Save Card Confirm", () =>
      cy.retrievePaymentCallTest({ globalState, data: saveCardConfirmData })
    );

    cy.step("Capture Payment", () =>
      cy.captureCallTest(fixtures.captureBody, captureData, globalState)
    );

    if (!utils.should_continue_further(captureData)) return;

    cy.step("Retrieve Payment after Capture", () =>
      cy.retrievePaymentCallTest({ globalState, data: captureData })
    );
  });

  it("Save card for NoThreeDS automatic capture payment - create and confirm [off_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    cy.step("Create Customer", () =>
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
    );

    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntentOffSession"];

    cy.step("Create Payment Intent", () =>
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        paymentIntentData,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SaveCardUseNo3DSAutoCaptureOffSession"];

    cy.step("Confirm Payment", () =>
      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
    );

    if (!utils.should_continue_further(confirmData)) return;

    cy.step("Retrieve Payment after Confirm", () =>
      cy.retrievePaymentCallTest({ globalState, data: confirmData })
    );

    cy.step("List Customer Payment Methods", () =>
      cy.listCustomerPMCallTest(globalState)
    );

    cy.step("Create Payment Intent for Subsequent Payment", () =>
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        paymentIntentData,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const saveCardConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardConfirmAutoCaptureOffSession"];

    cy.step("Save Card Confirm Call for Subsequent Payment", () =>
      cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState)
    );
  });

  it("Use billing address from payment method during subsequent payment [off_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
    cy.step("Create Customer", () =>
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
    );

    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntentOffSession"];

    cy.step("Create Payment Intent", () =>
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        paymentIntentData,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SaveCardUseNo3DSAutoCaptureOffSession"];

    cy.step("Confirm Payment", () =>
      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
    );

    if (!utils.should_continue_further(confirmData)) return;

    cy.step("list Customer Payment Methods", () =>
      cy.listCustomerPMCallTest(globalState)
    );

    cy.step("Create Payment Intent for Subsequent Payment", () =>
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        paymentIntentData,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const saveCardConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardConfirmAutoCaptureOffSessionWithoutBilling"];

    cy.step(
      "Save Card Confirm Call for Subsequent Payment without Billing Address",
      () =>
        cy.saveCardConfirmCallTest(
          saveCardBody,
          saveCardConfirmData,
          globalState
        )
    );
  });

  it("Check if card fields are populated when saving card again after a metadata update", () => {
    cy.step("Create Customer", () =>
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
    );

    const createConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSAutoCapture"];

    cy.step("Create and Confirm Payment", () =>
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        createConfirmData,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(createConfirmData)) return;

    cy.step("Retrieve Payment after Confirm", () =>
      cy.listCustomerPMCallTest(globalState)
    );

    const card_holder_name = generateRandomName();
    const newData = {
      ...createConfirmData,
      Request: {
        ...createConfirmData.Request,
        payment_method_data: {
          card: {
            ...createConfirmData.Request.payment_method_data.card,
            card_holder_name: card_holder_name,
          },
        },
      },
    };

    cy.step(
      "Create and Confirm Payment again with updated card holder name in metadata",
      () =>
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          newData,
          "no_three_ds",
          "automatic",
          globalState
        )
    );

    if (!utils.should_continue_further(createConfirmData)) return;

    cy.step("Retrieve Customer Payment Methods", () =>
      cy.listCustomerPMCallTest(globalState)
    );
  });
});
