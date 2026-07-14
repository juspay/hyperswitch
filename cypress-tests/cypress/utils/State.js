class State {
  data = {};
  constructor(data) {
    this.data = data;
    this.data["connectorId"] = Cypress.env("CONNECTOR");
    // Keep original connector when connectorId gets changed (e.g., stripeconnect -> stripe); optional and defaults to connectorId if not explicitly set.
    this.data["originalConnectorId"] = Cypress.env("CONNECTOR");
    this.data["baseUrl"] = Cypress.env("BASEURL");
    this.data["pmServiceUrl"] = Cypress.env("PM_SERVICE_URL");
    this.data["adminApiKey"] = Cypress.env("ADMINAPIKEY");
    this.data["email"] = Cypress.env("HS_EMAIL");
    this.data["password"] = Cypress.env("HS_PASSWORD");
    this.data["connectorAuthFilePath"] = Cypress.env(
      "CONNECTOR_AUTH_FILE_PATH"
    );
    this.data["ucsEnabled"] = Cypress.env("UCS_ENABLED");
    this.data["proxyHttp"] = Cypress.env("PROXY_HTTP");
    this.data["proxyHttps"] = Cypress.env("PROXY_HTTPS");
    this.data["methodFlow"] = Cypress.env("METHOD_FLOW");
    this.data["validationServiceUrl"] = Cypress.env("VALIDATION_SERVICE_URL");
    this.data["superpositionBaseUrl"] = Cypress.env("SUPERPOSITION_BASE_URL");
    this.data["superpositionSecret"] = Cypress.env("SUPERPOSITION_SECRET");
    this.data["superpositionApiKey"] = Cypress.env("SUPERPOSITION_API_KEY");
    this.data["superpositionOrgId"] = Cypress.env("SUPERPOSITION_ORG_ID");
    this.data["superpositionWorkspaceId"] = Cypress.env(
      "SUPERPOSITION_WORKSPACE_ID"
    );
  }

  set(key, val) {
    this.data[key] = val;
  }

  get(key) {
    return this.data[key];
  }
}

export default State;
