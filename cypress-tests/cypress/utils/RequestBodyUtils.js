const keyPrefixes = {
  localhost: {
    publishable_key: "pk_dev_",
    key_id: "dev_",
  },
  integ: {
    publishable_key: "pk_snd_",
    key_id: "snd_",
  },
  sandbox: {
    publishable_key: "pk_snd_",
    key_id: "snd_",
  },
};

export const setClientSecret = (requestBody, clientSecret) => {
  requestBody["client_secret"] = clientSecret;
};
export const setCardNo = (requestBody, cardNo) => {
  // pass confirm body here to set CardNo
  requestBody["payment_method_data"]["card"]["card_number"] = cardNo;
};

export const setApiKey = (requestBody, apiKey) => {
  requestBody["connector_account_details"]["api_key"] = apiKey;
};

export const generateRandomString = (prefix = "cyMerchant") => {
  const uuidPart = "xxxxxxxx";

  const randomString = uuidPart.replace(/[xy]/g, function (c) {
    const r = (Math.random() * 16) | 0;
    const v = c === "x" ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });

  return `${prefix}_${randomString}`;
};

export const setMerchantId = (merchantCreateBody, merchantId) => {
  merchantCreateBody["merchant_id"] = merchantId;
};

export function isoTimeTomorrow() {
  const now = new Date();

  // Create a new date object for tomorrow
  const tomorrow = new Date(now);
  tomorrow.setDate(now.getDate() + 1);

  // Convert to ISO string format
  const isoStringTomorrow = tomorrow.toISOString();
  return isoStringTomorrow;
}

export function validateEnv(baseUrl, keyIdType) {
  if (!baseUrl) {
    throw new Error("Please provide a baseUrl");
  }

  const environment = Object.keys(keyPrefixes).find((env) =>
    baseUrl.includes(env)
  );

  if (!environment) {
    throw new Error("Unsupported baseUrl");
  }

  const prefix = keyPrefixes[environment][keyIdType];

  if (!prefix) {
    throw new Error(`Unsupported keyIdType: ${keyIdType}`);
  }

  return prefix;
}
