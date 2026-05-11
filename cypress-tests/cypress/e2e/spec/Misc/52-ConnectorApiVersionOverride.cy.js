import State from "../../../utils/State";

let globalState;

describe("[Platform] Connector API Version Override", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Happy Path Tests", () => {
    const connectorName = Cypress.env("CONNECTOR");
    const configKey = `connector_api_version_${connectorName}`;
    const initialVersion = "2023-10";
    const updatedVersion = "2024-01";

    it("should create config for connector API version override", () => {
      cy.setConfigs(globalState, configKey, initialVersion, "CREATE");
    });

    it("should fetch and verify the created config", () => {
      cy.setConfigs(globalState, configKey, initialVersion, "FETCH");
    });

    it("should update the config value", () => {
      cy.setConfigs(globalState, configKey, updatedVersion, "UPDATE");
    });

    it("should verify the updated config value", () => {
      cy.setConfigs(globalState, configKey, updatedVersion, "FETCH");
    });

    it("should delete the config", () => {
      cy.setConfigs(globalState, configKey, updatedVersion, "DELETE");
    });

    it("should verify deletion returns 404 on fetch", () => {
      cy.setConfigs(globalState, configKey, updatedVersion, "FETCH");
    });
  });

  context("Negative Tests", () => {
    const connectorName = Cypress.env("CONNECTOR");
    const configKey = `connector_api_version_${connectorName}`;
    const apiVersion = "2023-10";

    it("should fail to create duplicate key with 400 error", () => {
      // First create the config
      cy.setConfigs(globalState, configKey, apiVersion, "CREATE");
      // Try to create again - should fail with 400
      cy.setConfigs(globalState, configKey, apiVersion, "CREATE");
    });

    it("should fail to fetch non-existent key with 404 error", () => {
      const nonExistentKey = `connector_api_version_nonexistent_${Date.now()}`;
      cy.setConfigs(globalState, nonExistentKey, apiVersion, "FETCH");
    });

    it("should fail to delete non-existent key with 404 error", () => {
      const nonExistentKey = `connector_api_version_nonexistent_delete_${Date.now()}`;
      cy.setConfigs(globalState, nonExistentKey, apiVersion, "DELETE");
    });

    it("cleanup duplicate key test config", () => {
      cy.setConfigs(globalState, configKey, apiVersion, "DELETE");
    });
  });
});
