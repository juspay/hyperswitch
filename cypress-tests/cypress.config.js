let globalState;

module.exports = {
  e2e: {
    setupNodeEvents(on, config) {
      on("task", {
        setGlobalState: (val) => {
          return (globalState = val || {});
        },
        getGlobalState: () => {
          return globalState || {};
        },
        cli_log: (message) => {
          console.log("Logging console message from task");
          console.log(message);
          return null;
        },
      });
    },
    experimentalRunAllSpecs: true,
    reporter: "mochawesome",
    reporterOptions: {
      reportDir: "cypress/reports",
      reportFilename: "report",
      overwrite: false,
      html: false,
      json: true,
      charts: true,
    },
  },
  chromeWebSecurity: false,
  defaultCommandTimeout: 10000,
  pageLoadTimeout: 20000,
  trashAssetsBeforeRuns: false,
};
