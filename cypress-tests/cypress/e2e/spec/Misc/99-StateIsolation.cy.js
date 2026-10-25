import * as fixtures from "../../../fixtures/imports";

describe("State and Fixture Isolation", () => {
  it("sets customerId and mutates session fixture in test 1", () => {
    cy.task("setGlobalState", { customerId: "test_customer_123" });
    cy.then(() => {
      fixtures.sessionTokenBody.payment_id = "test_payment_id";
      fixtures.sessionTokenBody.wallets = ["apple_pay"];
    });

    cy.task("getGlobalState").then((state) => {
      expect(state.customerId).to.equal("test_customer_123");
    });
    cy.then(() => {
      expect(fixtures.sessionTokenBody.payment_id).to.equal("test_payment_id");
    });
  });

  it("verifies customerId and session fixtures are cleanly reset in test 2", () => {
    cy.task("getGlobalState").then((state) => {
      expect(state.customerId).to.be.undefined;
    });
    cy.then(() => {
      expect(fixtures.sessionTokenBody.payment_id).to.equal("{{payment_id}}");
      expect(fixtures.sessionTokenBody.wallets).to.deep.equal([]);
    });
  });
});
