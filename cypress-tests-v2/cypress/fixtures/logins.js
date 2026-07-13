import merchantConnectorAccount from "./merchant_connector_account.json";

export function createMerchantConnectorDetails(key) {
  const body = {
    ...merchantConnectorAccount.mca_create,
    connector_name: key,
    connector_account_details: {
      auth_type: "BodyKey",
      api_key: "",
      key1: "",
    },
    metadata: {
      status_url: "https://webhook.site",
      account_name: "transaction_processing",
    },
  };
  return body;
}
