const connectorConfig = {
  amazonpay: {
    key: "amazonpay",
    connector_account_details: {
      auth_type: "BodyKey",
      api_key: "SANDBOX-AHS63HO2ZP2WZ36Z24MYDPH3",
      key1: "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASC...\n-----END PRIVATE KEY-----",
    },
    connector_wallets_details: {
      amazon_pay: {
        merchant_id: "A3UJN62U20X4GB",
        store_id: "amzn1.application-oa2-client.43ee1af277a94b6c8efd9118dd6c156c",
      },
    },
  },
};

export function getConnectorDetails(connectorName) {
  const connector = connectorConfig[connectorName];
  if (!connector) {
    return { key: connectorName };
  }
  return connector;
}

export const ConnectorPayload = {
  merchantAccountPayload() {
    return {
      merchant_name: "Hyperswitch Seller",
      metadata: {
        aws_account_id: "000000000000",
      },
    };
  },
};
