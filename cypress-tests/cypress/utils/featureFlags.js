/* eslint-disable no-console */
const config_fields = ["CONNECTOR_CREDENTIAL", "DELAY", "TRIGGER_SKIP"];

const DEFAULT_CONNECTOR = "connector_1";

// Helper function for type and range validation
function validateType(value, type) {
  if (typeof value !== type) {
    console.error(
      `Expected value to be of type ${type}, but got ${typeof value}.`
    );
    return false;
  }
  return true;
}

// Helper function to validate specific config keys based on schema rules
function validateConfigValue(key, value) {
  // At present, there are only 2 api keys for connectors. Will be scaled based on the need
  const SUPPORTED_CONNECTOR_CREDENTIAL = ["connector_1", "connector_2"];

  if (config_fields.includes(key)) {
    switch (key) {
      case "DELAY":
        if (typeof value !== "object" || value === null) {
          console.error("DELAY must be an object.");
          return false;
        }
        if (!validateType(value.STATUS, "boolean")) return false;
        if (
          !value.STATUS ||
          typeof value.TIMEOUT !== "number" ||
          value.TIMEOUT < 0 ||
          value.TIMEOUT > 30000
        ) {
          console.error(
            "DELAY.TIMEOUT must be an integer between 0 and 30000 and DELAY.STATUS must be enabled."
          );
          return false;
        }
        break;

      case "CONNECTOR_CREDENTIAL":
        if (typeof value !== "object" || value === null) {
          console.error("CONNECTOR_CREDENTIAL must be an object.");
          return false;
        }
        // Validate nextConnector and multipleConnectors if present
        if (
          value?.nextConnector !== undefined &&
          typeof value.nextConnector !== "boolean"
        ) {
          console.error("nextConnector must be a boolean");
          return false;
        }

        if (
          value?.multipleConnectors &&
          typeof value.multipleConnectors.status !== "boolean"
        ) {
          console.error("multipleConnectors.status must be a boolean");
          return false;
        }

        // Validate structure
        if (
          !value.value ||
          !SUPPORTED_CONNECTOR_CREDENTIAL.includes(value.value)
        ) {
          console.error(
            `Config ${key}.value must be one of ${SUPPORTED_CONNECTOR_CREDENTIAL.join(", ")}.`
          );
          return false;
        }
        break;

      case "TRIGGER_SKIP":
      case "DELAY.STATUS":
        if (!validateType(value, "boolean")) return false;
        break;

      default:
        console.error(`Config key ${key} is invalid.`);
        return false;
    }
  } else {
    console.error(`Config key ${key} is invalid.`);
  }
  return true;
}

// Function to validate the config object
export function validateConfig(configObject) {
  // Configs object is an optional field in Connector Configs
  // If passed, it must be a valid Object
  if (typeof configObject === "undefined") {
    return null;
  } else if (typeof configObject !== "object" || configObject === null) {
    console.error(`Provided config is invalid:\n${configObject}`);
    return null;
  }

  for (const key in configObject) {
    if (Object.prototype.hasOwnProperty.call(configObject, key)) {
      const value = configObject[key];
      if (!validateConfigValue(key, value)) {
        return null; // Return null if any validation fails
      }
    }
  }

  return configObject;
}

export function getProfileAndConnectorId(connectorType) {
  const credentials = {
    connector_1: {
      profileId: "profile",
      connectorId: "merchantConnector",
    },
    connector_2: {
      profileId: "profile1",
      connectorId: "merchantConnector1",
    },
  };

  return credentials[connectorType] || credentials.connector_1;
}

function getSpecName() {
  return Cypress.spec.name.toLowerCase() === "__all"
    ? String(
        Cypress.mocha.getRunner().suite.ctx.test.invocationDetails.relativeFile
      )
        .split("/")
        .pop()
        .toLowerCase()
    : Cypress.spec.name.toLowerCase();
}

function matchesSpecName(specName) {
  if (!specName || !Array.isArray(specName) || specName.length === 0) {
    return false;
  }

  const currentSpec = getSpecName();
  return specName.some(
    (name) => name && currentSpec.includes(name.toLowerCase())
  );
}

export function determineConnectorConfig(config) {
  const connectorCredential = config?.CONNECTOR_CREDENTIAL;
  const multipleConnectors = config?.multipleConnectors;

  // If CONNECTOR_CREDENTIAL doesn't exist or value is null, return default
  if (!connectorCredential || connectorCredential.value === null) {
    return DEFAULT_CONNECTOR;
  }

  // Handle nextConnector cases
  if (
    Object.prototype.hasOwnProperty.call(connectorCredential, "nextConnector")
  ) {
    if (connectorCredential.nextConnector === true) {
      // Check multipleConnectors conditions if available
      if (
        multipleConnectors?.status === true &&
        multipleConnectors?.count > 1
      ) {
        return "connector_2";
      }
      return DEFAULT_CONNECTOR;
    }
    return DEFAULT_CONNECTOR;
  }

  // Handle specName cases
  if (Object.prototype.hasOwnProperty.call(connectorCredential, "specName")) {
    return matchesSpecName(connectorCredential.specName)
      ? connectorCredential.value
      : DEFAULT_CONNECTOR;
  }

  // Return value if it's the only property
  return connectorCredential.value;
}

export function execConfig(configs) {
  if (configs?.DELAY?.STATUS) {
    cy.wait(configs.DELAY.TIMEOUT);
  }

  const connectorType = determineConnectorConfig(configs);
  const { profileId, connectorId } = getProfileAndConnectorId(connectorType);

  return {
    profilePrefix: profileId,
    merchantConnectorPrefix: connectorId,
  };
}
