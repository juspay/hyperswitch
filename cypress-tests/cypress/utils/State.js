class State {
    data = {};
  constructor(data) {
    this.data = data;
    this.data["connectorId"] = Cypress.env("CONNECTOR");
  }

  set(key, val) {
    this.data[key] = val;
  }

  get(key) {
    return this.data[key];
  }
}

export default State;
