export default function step(name, shouldContinue, callback) {
    if (!shouldContinue) {
      cy.task("cli_log", `Skipping step: ${name}`);
      return;
    }
    callback();
}