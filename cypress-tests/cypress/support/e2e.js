// ***********************************************************
// This example support/e2e.js is processed and
// loaded automatically before your test files.
//
// This is a great place to put global configuration and
// behavior that modifies Cypress.
//
// You can change the location of this file or turn off
// automatically serving support files with the
// 'supportFile' configuration option.
//
// You can read more here:
// https://on.cypress.io/configuration
// ***********************************************************

// Import commands.js using ES2015 syntax:
import "cypress-mochawesome-reporter/register";
require('./commands');
import "./redirectionHandler";

Cypress.on("window:before:load", (win) => {
  // Add security headers
  win.headers = {
    "Content-Security-Policy": "default-src 'self'",
    "X-Content-Type-Options": "nosniff",
    "X-Frame-Options": "DENY",
  };
});

// Add error handling for dynamic imports
Cypress.on("uncaught:exception", (err, runnable) => {
  // Log the error details
  // eslint-disable-next-line no-console
  console.error(
    `Error: ${err.message}\nError occurred in: ${runnable.title}\nStack trace: ${err.stack}`
  );

  // Return false to prevent the error from failing the test
  return false;
});

// Overwrite cy.request to add X-Merchant-Id header to all HTTP requests
Cypress.Commands.overwrite('request', (originalFn, ...args) => {
  const options = typeof args[0] === 'string' ? args[1] || {} : args[0] || {};

  const headers = {
    'x-feature': 'router-new'
  };

  const mergedOptions = {
    ...options,
    headers: {
      ...((options && options.headers) || {}),
      ...headers
    }
  };

  // Handle different argument patterns for cy.request
  if (typeof args[0] === 'string') {
    // cy.request(url, options)
    return originalFn(args[0], mergedOptions);
  } else {
    // cy.request(options)
    return originalFn(mergedOptions);
  }
});
