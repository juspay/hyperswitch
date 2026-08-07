class State {
  data = {};
  constructor(data) {
    this.data = data || {};
    this.data["connectorId"] = this.getEnvOrState("CONNECTOR", "connectorId");
    // Keep original connector when connectorId gets changed (e.g., stripeconnect -> stripe); optional and defaults to connectorId if not explicitly set.
    this.data["originalConnectorId"] = this.getEnvOrState(
      "CONNECTOR",
      "originalConnectorId"
    );
    this.data["baseUrl"] = this.getUrlEnvOrState("BASEURL", "baseUrl");
    this.data["pmServiceUrl"] = this.getUrlEnvOrState(
      "PM_SERVICE_URL",
      "pmServiceUrl"
    );
    this.data["kvEnabled"] = Cypress.env("KV_ENABLED");
    this.data["adminApiKey"] = this.getEnvOrState("ADMINAPIKEY", "adminApiKey");
    this.data["email"] = this.getEnvOrState("HS_EMAIL", "email");
    this.data["password"] = this.getEnvOrState("HS_PASSWORD", "password");
    this.data["connectorAuthFilePath"] = this.getEnvOrState(
      "CONNECTOR_AUTH_FILE_PATH",
      "connectorAuthFilePath"
    );
    this.data["ucsEnabled"] = this.getEnvOrState("UCS_ENABLED", "ucsEnabled");
    this.data["proxyHttp"] = this.getUrlEnvOrState("PROXY_HTTP", "proxyHttp");
    this.data["proxyHttps"] = this.getUrlEnvOrState(
      "PROXY_HTTPS",
      "proxyHttps"
    );
    this.data["methodFlow"] = this.getEnvOrState("METHOD_FLOW", "methodFlow");
    this.data["validationServiceUrl"] = this.getUrlEnvOrState(
      "VALIDATION_SERVICE_URL",
      "validationServiceUrl"
    );
    this.data["superpositionBaseUrl"] = this.getUrlEnvOrState(
      "SUPERPOSITION_BASE_URL",
      "superpositionBaseUrl"
    );
    this.data["superpositionSecret"] = this.getEnvOrState(
      "SUPERPOSITION_SECRET",
      "superpositionSecret"
    );
    this.data["superpositionApiKey"] = this.getEnvOrState(
      "SUPERPOSITION_API_KEY",
      "superpositionApiKey"
    );
    this.data["superpositionOrgId"] = this.getEnvOrState(
      "SUPERPOSITION_ORG_ID",
      "superpositionOrgId"
    );
    this.data["superpositionWorkspaceId"] = this.getEnvOrState(
      "SUPERPOSITION_WORKSPACE_ID",
      "superpositionWorkspaceId"
    );
  }

  getEnvOrState(envKey, stateKey = envKey) {
    const envValue = Cypress.env(envKey);
    return envValue === undefined || envValue === null || envValue === ""
      ? this.data[stateKey]
      : envValue;
  }

  getUrlEnvOrState(envKey, stateKey) {
    return State.sanitizeUrl(this.getEnvOrState(envKey, stateKey));
  }

  static sanitizeUrl(url) {
    if (!url || typeof url !== "string") {
      return url;
    }

    const markdownLinkMatch = url.match(/^\[(https?:\/\/[^\]]+)\]/);
    const normalizedUrl = markdownLinkMatch ? markdownLinkMatch[1] : url;
    return normalizedUrl.trim().replace(/\/+$/, "");
  }

  set(key, val) {
    this.data[key] = val;
  }

  get(key) {
    return this.data[key];
  }
}

export default State;
