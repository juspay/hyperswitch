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

export const generateRandomString = (prefix = "cypress_merchant_GHAction_") => {
  const uuidPart = "xxxxxxxx";

  const randomString = uuidPart.replace(/[xy]/g, function (c) {
    const r = (Math.random() * 16) | 0;
    const v = c === "x" ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });

  return prefix + randomString;
};

export const setMerchantId = (merchantCreateBody, merchantId) => {
  merchantCreateBody["merchant_id"] = merchantId;
};
