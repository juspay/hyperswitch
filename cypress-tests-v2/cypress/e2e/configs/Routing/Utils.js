import { configs as advancedConfigs } from "./AdvancedConfigs.js";
import { configs as fallbackConfigs } from "./FallbackConfigs.js";

const serviceDetails = {
  advanced_configs: advancedConfigs,
  fallback_configs: fallbackConfigs,
};

export const getServiceDetails = (serviceId) => {
  let data = getValueByKey(serviceDetails, serviceId);
  return data;
};

function getValueByKey(jsonObject, key) {
  const data =
    typeof jsonObject === "string" ? JSON.parse(jsonObject) : jsonObject;

  if (data && typeof data === "object" && key in data) {
    const value = data[key];

    // Check if the value has connector_account_details
    if (value && typeof value === "object" && value.connector_account_details) {
      return value;
    }

    // Check if it has nested structure like connector_1, connector_2
    if (value && typeof value === "object") {
      // Default to connector_1 if it exists
      if (value.connector_1 && value.connector_1.connector_account_details) {
        return value.connector_1;
      }
      // Fallback to first key that has connector_account_details
      const keys = Object.keys(value);
      for (const nestedKey of keys) {
        if (value[nestedKey] && value[nestedKey].connector_account_details) {
          return value[nestedKey];
        }
      }
    }

    return value;
  } else {
    return null;
  }
}

export const should_continue_further = (res_data) => {
  if (
    res_data.body.error !== undefined ||
    res_data.body.error_code !== undefined ||
    res_data.body.error_message !== undefined
  ) {
    return false;
  } else {
    return true;
  }
};
