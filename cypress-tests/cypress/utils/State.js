class State {
  data = {};
  constructor(data) {
    this.data = data;
    this.data["connectorId"] = Cypress.env("CONNECTOR");
    this.data["baseUrl"] = Cypress.env("BASEURL");
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
  }

  set(key, val) {
    this.data[key] = val;
  }

  get(key) {
    return this.data[key];
  }
}

export default State;
