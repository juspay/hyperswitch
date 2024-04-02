// const { defineConfig } = require('cypress');

let globalState;
module.exports = {
  e2e: {
    setupNodeEvents(on, config) {
      on('task', {
        setGlobalState: (val) => {
          return (globalState = (val || {}))
        },
        getGlobalState: () => {
          return (globalState || {})
        },
        cli_log: (message) => {
          console.log("Logging console message from task");
          // console.trace();
          console.log(message);
          return null;
        }
      })
    },
    experimentalRunAllSpecs: true
  },
  chromeWebSecurity: false
  // retries: { runMode: 3, openMode: 3 },
};