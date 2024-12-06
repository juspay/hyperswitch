/* eslint-disable no-console */
const config_fields = ["CONNECTOR_CREDENTIAL", "DELAY", "TRIGGER_SKIP"];

const DEFAULT_CONNECTOR = "connector_1";
const DEFAULT_CREDENTIALS = {
  profile_id: "profileId",
  merchant_connector_id: "merchantConnectorId",
};
const CONNECTOR_2_CREDENTIALS = {
  profile_id: "profile1Id",
  merchant_connector_id: "merchantConnector1Id",
};

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

export function execConfig(configs) {
  // Handle delay if present
  if (configs?.DELAY?.STATUS) {
    cy.wait(configs.DELAY.TIMEOUT);
  }
  if (
    typeof configs?.CONNECTOR_CREDENTIAL === "undefined" ||
    configs?.CONNECTOR_CREDENTIAL.value === "null"
  ) {
    return DEFAULT_CREDENTIALS;
  }

  // Get connector configuration
  const connectorType = determineConnectorConfig(configs.CONNECTOR_CREDENTIAL);

  // Return credentials based on connector type
  return connectorType === "connector_2"
    ? CONNECTOR_2_CREDENTIALS
    : DEFAULT_CREDENTIALS;
}

function determineConnectorConfig(connectorConfig) {
  // Return default if config is undefined or null
  if (!connectorConfig || connectorConfig.value === "null") {
    return DEFAULT_CONNECTOR;
  }

  const { specName = null, value } = connectorConfig;

  // If value is not provided, return default
  if (!value) {
    return DEFAULT_CONNECTOR;
  }

  // If no specName or not an array, return value directly
  if (!specName || !Array.isArray(specName) || specName.length === 0) {
    return value;
  }

  // Check if current spec matches any in specName
  const currentSpec =
    // edge case for running in ui
    Cypress.spec.name.toLowerCase() === "__all"
      ? String(
          Cypress.mocha.getRunner().suite.ctx.test.invocationDetails
            .relativeFile
        )
          .split("/")
          .pop()
      : Cypress.spec.name.toLowerCase();

  try {
    const matchesSpec = specName.some(
      (name) => name && currentSpec.includes(name.toLowerCase())
    );
    return matchesSpec ? value : DEFAULT_CONNECTOR;
  } catch (error) {
    console.error("Error matching spec names:", error);
    return DEFAULT_CONNECTOR;
  }
}
