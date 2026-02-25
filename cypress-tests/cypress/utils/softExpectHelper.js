// softAssertHelper.js
// Place in cypress/e2e/utils/ or cypress/support/

/**
 * Wraps an assertion in try-catch and stores failures in globalState
 * Usage: softExpect(globalState, () => expect(actual).to.equal(expected))
 */
export function softExpect(globalState, name, assertionFn) {
    try {
      assertionFn();
    } catch (error) {
      // Initialize errors array if not exists
      const errors = globalState.get("softAssertErrors") || [];
      errors.push({
        message: error.message,
        name,
        actual: error.actual,
        expected: error.expected,
      });
      globalState.set("softAssertErrors", errors);
      
      // Log the failure but don't throw
      cy.task("cli_log", `${RED} EXPECT FAILED: ${error.message} ${RESET}`);
    }
  }
  
  /**
   * Call at the start of each it block to clear previous errors
   */
  export function clearSoftAssertErrors(globalState) {
    globalState.set("softAssertErrors", []);
  }
  
  /**
   * Call at the end of each it block - throws if there were any failures
   */
  export function assertAllSoftErrors(globalState, testName = "Test") {
    const errors = globalState.get("softAssertErrors") || [];
    
    if (errors.length > 0) {
      const errorMessage = [
        "",
        `  SOFT ASSERTION FAILURES — ${testName} (${errors.length} failure${errors.length > 1 ? "s" : ""})`,
        "",
        ...errors.map((e, i) => {
          const diffBlock =
            e.actual !== undefined && e.expected !== undefined
              ? buildDiff(e.actual, e.expected)
              : "";
      
          return [
            `  ${i + 1}. [${e.name}]`,
            `     ${e.message}`,
            "",
            diffBlock,
            "",
          ].join("\n");
        }),
      ].join("\n");
      // Clear errors after reporting
      globalState.set("softAssertErrors", []);
      throw new Error(errorMessage);
    }
  }

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
        if (aLine !== undefined) diffLines.push(`${RED}  -   ${aLine.trim()}${RESET}`);
        if (eLine !== undefined) diffLines.push(`${GREEN}  +   ${eLine.trim()}${RESET}`);
      }
    }
  
    return [
     `      ${GREEN}+ expected${RESET} - actual`,
      "",
      ...diffLines,
    ]
      // .filter((line, i) => i !== 0 || label)
      .join("\n");
  }