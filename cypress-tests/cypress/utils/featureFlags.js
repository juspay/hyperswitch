const config_fields = ["CONNECTOR_CREDENTIAL", "DELAY", "TRIGGER_SKIP"];

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
  const CONNECTOR_CREDENTIAL = ["connector_1", "connector_2"];

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
        if (!CONNECTOR_CREDENTIAL.includes(value)) {
          console.error(
            `Config ${key} is invalid. Value should be one of ${CONNECTOR_CREDENTIAL.join(", ")}.`
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

  for (let key in configObject) {
    if (configObject.hasOwnProperty(key)) {
      const value = configObject[key];
      if (!validateConfigValue(key, value)) {
        return null; // Return null if any validation fails
      }
    }
  }

  return configObject;
}

export function execConfig(configs) {
  if (typeof configs !== "undefined" && typeof configs === "object") {
    if (configs?.DELAY?.STATUS) {
      cy.wait(configs.DELAY.TIMEOUT);
    }

    if (
      typeof configs?.CONNECTOR_CREDENTIAL === "undefined" ||
      typeof configs?.CONNECTOR_CREDENTIAL === "null"
    ) {
      return "profileId";
    } else {
      if (configs.CONNECTOR_CREDENTIAL === "connector_1") {
        return {
          profile_id: "profileId",
          merchant_connector_id: "merchantConnectorId",
        };
      } else if (configs.CONNECTOR_CREDENTIAL === "connector_2") {
        return {
          profile_id: "profile1Id",
          merchant_connector_id: "merchantConnector1Id",
        };
      }
    }
  }
}
