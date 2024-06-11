class State {
  data = {};
  constructor(data) {
    this.data = data;
    this.data["connectorId"] = Cypress.env("CONNECTOR");
    this.data["baseUrl"] = Cypress.env("BASEURL");
    this.data["adminApiKey"] = Cypress.env("ADMINAPIKEY");
    this.data["profileId"] = Cypress.env("PROFILE_ID");
    this.data["stripeMcaId"] = Cypress.env("STRIPE_MCA_ID");
    this.data["adyenMcaId"] = Cypress.env("ADYEN_MCA_ID");
    this.data["routingApiKey"] = Cypress.env("ROUTING_API_KEY");
    this.data["email"] = Cypress.env(
      "HS_EMAIL",
    );
    this.data["password"] = Cypress.env(
      "HS_PASSWORD",
    );
    this.data["connectorAuthFilePath"] = Cypress.env(
      "CONNECTOR_AUTH_FILE_PATH",
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
