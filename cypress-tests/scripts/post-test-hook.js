/* eslint-disable no-console */
import { exec } from "child_process";
import path from "path";
import { fileURLToPath } from "url";
import { promisify } from "util";

const execAsync = promisify(exec);
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// This script runs after Cypress tests complete
async function runPostTestTasks() {
  console.log("🔄 Running post-test tasks...");

  try {
    // Generate the report
    console.log("📊 Generating test report...");
    const reportGeneratorPath = path.join(__dirname, "report-generator.js");
    await execAsync(`node ${reportGeneratorPath}`);

    // Open the dashboard in the default browser (optional)
    const dashboardPath = path.join(__dirname, "../dashboard/index.html");
    const openCommand =
      process.platform === "darwin"
        ? `open ${dashboardPath}`
        : process.platform === "win32"
          ? `start ${dashboardPath}`
          : `xdg-open ${dashboardPath}`;

    console.log("🌐 Opening dashboard...");
    await execAsync(openCommand);

    console.log("✅ Post-test tasks completed successfully!");
  } catch (error) {
    console.error("❌ Error in post-test tasks:", error);
    process.exit(1);
  }
}

// Run the tasks
runPostTestTasks();
