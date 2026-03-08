export default function reportErrors(errors) {
  const errorMessages = errors
    .map(({ step, error }) => `[${step}]: ${error.message}`)
    .join("\n\n");
  throw new Error(`Errors occurred during the test:\n\n${errorMessages}`);
}
