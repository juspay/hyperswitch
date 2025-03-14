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
