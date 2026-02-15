const { defineConfig } = require('cypress');

module.exports = defineConfig({
  e2e: {
    specPattern: 'e2e/**/*.cy.{js,jsx,ts,tsx}',
    supportFile: false,
    baseUrl: process.env.PAYMENT_BASE_URL || 'http://localhost:8080',
    env: {
      PAYMENT_BASE_URL: process.env.PAYMENT_BASE_URL,
      PAYSTACK_PUBLIC_KEY: process.env.PAYSTACK_PUBLIC_KEY,
      PAYSTACK_SECRET_KEY: process.env.PAYSTACK_SECRET_KEY,
      CYPRESS_SKIP_DASHBOARD: String(process.env.CYPRESS_SKIP_DASHBOARD) === 'true'
    }
  }
});
