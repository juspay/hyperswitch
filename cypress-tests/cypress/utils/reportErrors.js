const GREEN = "\x1b[32m";
const RED = "\x1b[31m";
const RESET = "\x1b[0m";

function buildDiff(actual, expected) {
  const actualLines = actual.trim().split("\n");
  const expectedLines = expected.trim().split("\n");

  const diffLines = [];
  const maxLen = Math.max(actualLines.length, expectedLines.length);

  for (let i = 0; i < maxLen; i++) {
    const aLine = actualLines[i];
    const eLine = expectedLines[i];

    if (aLine !== eLine) {
      if (aLine !== undefined)
        diffLines.push(`${RED}  -   ${aLine.trim()}${RESET}`);
      if (eLine !== undefined)
        diffLines.push(`${GREEN}  +   ${eLine.trim()}${RESET}`);
    }
  }

  return [
    `      ${GREEN}+ expected${RESET} - actual`,
    "",
    ...diffLines,
  ].join("\n");
}

export default function reportErrors(errors) {
  const errorMessages = errors
    .map(({ step, error }) => {
      let msg = `[${step}]: ${error.message}`;
      if (error.actual !== undefined && error.expected !== undefined) {
        msg += `\n${buildDiff(String(error.actual), String(error.expected))}`;
      }
      return msg;
    })
    .join("\n\n");
  throw new Error(`Errors occurred during the test:\n\n${errorMessages}`);
}
