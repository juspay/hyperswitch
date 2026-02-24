// softAssertHelper.js
// Place in cypress/e2e/utils/ or cypress/support/

/**
 * Wraps an assertion in try-catch and stores failures in globalState
 * Usage: softExpect(globalState, () => expect(actual).to.equal(expected), "description")
 */
export function softExpect(globalState, name, assertionFn, description = "") {
    try {
      assertionFn();
    } catch (error) {
      // Initialize errors array if not exists
      cy.task("cli_log",error);
      const errors = globalState.get("softAssertErrors") || [];
      errors.push({
        description,
        message: error.message,
        name,
        actual: error.actual,
        expected: error.expected,
      });
      globalState.set("softAssertErrors", errors);
      
      // Log the failure but don't throw
      cy.task("cli_log", `⚠️ SOFT ASSERT FAILED: [${description}] ${error.message}`);
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
        "╔══════════════════════════════════════════════════════════════╗",
        "║               SOFT ASSERTION FAILURES                       ║",
        "╚══════════════════════════════════════════════════════════════╝",
        "",
        `  Test   : ${testName}`,
        `  Failures: ${errors.length}`,
        "",
        "──────────────────────────────────────────────────────────────",
        "",
        ...errors.map((e, i) => [
          `  ${i + 1}. ${e.name}`,
          `     Description : ${e.description}`,
          `     Message     : ${e.message}`,
          ...(e.actual !== undefined
            ? [
                `     Actual      : ${e.actual.split("\n").join("\n" + " ".repeat(19))}`,
                `     Expected    : ${e.expected.split("\n").join("\n" + " ".repeat(19))}`,
              ]
            : []),
          "",
          "  ┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄┄",
          "",
        ].join("\n")),
        "══════════════════════════════════════════════════════════════",
        "",
      ].join("\n");
      // Clear errors after reporting
      globalState.set("softAssertErrors", []);
      throw new Error(errorMessage);
    }
  }