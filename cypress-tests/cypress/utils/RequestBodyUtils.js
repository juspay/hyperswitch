const keyPrefixes = {
  localhost: {
    publishable_key: "pk_dev_",
    key_id: "dev_",
  },
  hyperswitch: {
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

/**
 * Generates a random email address for testing purposes.
 * @returns {string} A randomly generated email address (e.g., "alex.smith123@example.com").
 */
export function generateRandomEmail() {
  const firstNames = [
    "alex",
    "jamie",
    "taylor",
    "morgan",
    "casey",
    "jordan",
    "pat",
    "sam",
    "chris",
    "dana",
    "olivia",
    "liam",
    "emma",
    "noah",
    "ava",
    "william",
    "sophia",
    "james",
    "isabella",
    "oliver",
    "charlotte",
    "benjamin",
    "amelia",
    "elijah",
    "mia",
    "lucas",
    "harper",
    "mason",
    "evelyn",
    "logan",
    "abigail",
  ];

  const lastNames = [
    "smith",
    "jones",
    "williams",
    "brown",
    "davis",
    "miller",
    "wilson",
    "moore",
    "taylor",
    "lee",
    "anderson",
    "thomas",
    "jackson",
    "white",
    "harris",
    "martin",
    "garcia",
    "martinez",
    "robinson",
    "clark",
    "rodriguez",
  ];

  const domains = [
    "example.com",
    "test.com",
    "demo.org",
    "sample.net",
    "testing.io",
    "cypress.test",
    "automation.dev",
    "qa.example",
  ];

  const randomFirstName =
    firstNames[Math.floor(Math.random() * firstNames.length)];
  const randomLastName =
    lastNames[Math.floor(Math.random() * lastNames.length)];
  const randomDomain = domains[Math.floor(Math.random() * domains.length)];
  const randomNumber = Math.floor(Math.random() * 1000);

  return `${randomFirstName}.${randomLastName}${randomNumber}@${randomDomain}`;
}

/**
 * Generates a random-ish card holder name from predefined lists.
 * @returns {string} A randomly generated full name (e.g., "Jane Smith").
 */
export function generateRandomName() {
  const firstNames = [
    "Alex",
    "Jamie",
    "Taylor",
    "Morgan",
    "Casey",
    "Jordan",
    "Pat",
    "Sam",
    "Chris",
    "Dana",
    "Olivia",
    "Liam",
    "Emma",
    "Noah",
    "Ava",
    "William",
    "Sophia",
    "James",
    "Isabella",
    "Oliver",
    "Charlotte",
    "Benjamin",
    "Amelia",
    "Elijah",
    "Mia",
    "Lucas",
    "Harper",
    "Mason",
    "Evelyn",
    "Logan",
    "Abigail",
    "Alexander",
    "Emily",
    "Ethan",
    "Elizabeth",
    "Jacob",
    "Mila",
    "Michael",
    "Ella",
    "Daniel",
    "Avery",
    "Henry",
    "Sofia",
    "Jackson",
    "Camila",
    "Sebastian",
    "Aria",
    "Aiden",
    "Scarlett",
    "Matthew",
    "Victoria",
    "Samuel",
    "Madison",
    "David",
    "Luna",
    "Joseph",
    "Grace",
    "Carter",
    "Chloe",
    "Owen",
    "Penelope",
    "Wyatt",
    "Layla",
    "John",
    "Riley",
    "Jack",
    "Zoey",
    "Luke",
    "Nora",
    "Jayden",
    "Lily",
  ];
  const lastNames = [
    "Smith",
    "Jones",
    "Williams",
    "Brown",
    "Davis",
    "Miller",
    "Wilson",
    "Moore",
    "Taylor",
    "Lee",
    "Dylan",
    "Eleanor",
    "Grayson",
    "Hannah",
    "Levi",
    "Lillian",
    "Isaac",
    "Addison",
    "Gabriel",
    "Aubrey",
    "Julian",
    "Ellie",
    "Mateo",
    "Stella",
    "Anthony",
    "Natalie",
    "Jaxon",
    "Zoe",
    "Lincoln",
    "Leah",
    "Joshua",
    "Hazel",
    "Christopher",
    "Violet",
    "Andrew",
    "Aurora",
    "Theodore",
    "Savannah",
    "Caleb",
    "Audrey",
    "Ryan",
    "Brooklyn",
    "Asher",
    "Bella",
    "Nathan",
    "Claire",
    "Thomas",
    "Skylar",
    "Leo",
    "Lucy",
    "Isaiah",
    "Paisley",
    "Charles",
    "Everly",
    "Josiah",
    "Anna",
    "Hudson",
    "Caroline",
    "Christian",
    "Nova",
    "Hunter",
    "Genesis",
    "Connor",
    "Emilia",
    "Eli",
    "Kennedy",
    "Ezra",
    "Samantha",
    "Aaron",
    "Maya",
    "Landon",
    "Willow",
    "Adrian",
    "Kinsley",
    "Jonathan",
    "Naomi",
    "Nolan",
    "Aaliyah",
  ];

  const randomFirstName =
    firstNames[Math.floor(Math.random() * firstNames.length)];
  const randomLastName =
    lastNames[Math.floor(Math.random() * lastNames.length)];

  return `${randomFirstName} ${randomLastName}`;
}

/**
 * Detects if running in CI environment
 * @returns {boolean} True if running in CI, false otherwise
 */
export const isCI = () => {
  return process.env.CI === "true" || process.env.GITHUB_ACTIONS === "true";
};

/**
 * Gets the appropriate timeout multiplier based on environment
 * @returns {number} 1.5 for CI environments, 1.0 for local development
 */
export const getTimeoutMultiplier = () => {
  return isCI() ? 1.5 : 1;
};
