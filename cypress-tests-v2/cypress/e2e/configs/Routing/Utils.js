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
    return data[key];
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
