class State {
  data = {};
  constructor(data) {
    this.data = data;
    this.data["connectorId"] = Cypress.env("CONNECTOR");
    this.data["baseUrl"] = Cypress.env("BASEURL");
    this.data["adminApiKey"] = Cypress.env("ADMINAPIKEY");
    this.data["connectorAuthFilePath"] = Cypress.env("CONNECTOR_AUTH_FILE_PATH");
  }

  set(key, val) {
    this.data[key] = val;
  }

  get(key) {
    return this.data[key];
  }
}

export default State;
