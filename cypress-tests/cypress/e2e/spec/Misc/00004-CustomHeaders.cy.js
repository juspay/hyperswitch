import State from "../../../utils/State";
import { getCustomHeaders } from "../../../support/commands";

describe("Cypress Framework Custom Headers Support", () => {
  context("State class customHeaders handling", () => {
    it("should initialize with empty customHeaders when none provided", () => {
      const state = new State({});
      expect(state.getCustomHeaders()).to.deep.equal({});
      expect(state.get("customHeaders")).to.deep.equal({});
    });

    it("should set and retrieve individual custom headers using setCustomHeader", () => {
      const state = new State({});
      state.setCustomHeader("x-merchant-id", "merchant_test_123");
      state.setCustomHeader("x-client-platform", "cypress-e2e");

      expect(state.get("customHeaders")).to.deep.equal({
        "x-merchant-id": "merchant_test_123",
        "x-client-platform": "cypress-e2e",
      });
    });

    it("should extract customHeaders using getCustomHeaders helper", () => {
      const state = new State({
        customHeaders: {
          "x-feature-flag": "enabled",
        },
      });

      const extracted = getCustomHeaders(state);
      expect(extracted).to.deep.equal({
        "x-feature-flag": "enabled",
      });
    });
  });

  context("Request header merging", () => {
    it("should include custom headers when making requests", () => {
      // Mock / test with a local Cypress env header
      Cypress.env("CUSTOM_HEADERS", {
        "x-custom-test-header": "test-val-123",
      });

      const state = new State({});
      const headers = getCustomHeaders(state);

      expect(headers).to.have.property("x-custom-test-header", "test-val-123");

      // Clean up
      Cypress.env("CUSTOM_HEADERS", undefined);
    });

    it("should parse stringified JSON in CUSTOM_HEADERS env", () => {
      Cypress.env(
        "CUSTOM_HEADERS",
        JSON.stringify({ "x-parsed-header": "parsed-value" })
      );

      const state = new State({});
      const headers = state.getCustomHeaders();

      expect(headers).to.deep.equal({
        "x-parsed-header": "parsed-value",
      });

      // Clean up
      Cypress.env("CUSTOM_HEADERS", undefined);
    });
  });
});
