/* eslint-disable no-console */
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

class CypressReportGenerator {
  constructor(reportsDir = path.join(__dirname, "../cypress/reports")) {
    this.reportsDir = reportsDir;
    this.summaryData = {
      connectors: {},
      totalTests: 0,
      totalPassed: 0,
      totalFailed: 0,
      totalSkipped: 0,
      totalPending: 0,
      executionTime: 0,
      timestamp: new Date().toISOString(),
      failedTests: [],
    };
  }

  async generateReport() {
    try {
      await this.collectTestResults();
      await this.calculateMetrics();
      await this.generateSummaryReport();
      await this.generateDashboardData();
      console.log("✅ Report generation completed successfully!");
    } catch (error) {
      console.error("❌ Error generating report:", error);
      process.exit(1);
    }
  }

  async collectTestResults() {
    const connectorDirs = fs
      .readdirSync(this.reportsDir)
      .filter((item) =>
        fs.statSync(path.join(this.reportsDir, item)).isDirectory()
      );

    for (const connector of connectorDirs) {
      const connectorPath = path.join(this.reportsDir, connector);
      const jsonFiles = fs
        .readdirSync(connectorPath)
        .filter(
          (file) => file.endsWith(".json") && file !== "mochawesome.json"
        );

      this.summaryData.connectors[connector] = {
        tests: [],
        totalTests: 0,
        passed: 0,
        failed: 0,
        skipped: 0,
        pending: 0,
        executionTime: 0,
        testsByFile: {},
      };

      for (const jsonFile of jsonFiles) {
        const reportPath = path.join(connectorPath, jsonFile);
        const reportData = JSON.parse(fs.readFileSync(reportPath, "utf8"));

        this.processReportData(connector, reportData);
      }
    }
  }

  processReportData(connector, reportData) {
    const connectorData = this.summaryData.connectors[connector];

    // Process stats
    if (reportData.stats) {
      connectorData.totalTests += reportData.stats.tests || 0;
      connectorData.passed += reportData.stats.passes || 0;
      connectorData.failed += reportData.stats.failures || 0;
      connectorData.skipped += reportData.stats.skipped || 0;
      connectorData.pending += reportData.stats.pending || 0;
      connectorData.executionTime += reportData.stats.duration || 0;
    }

    // Process individual test results
    if (reportData.results && reportData.results.length > 0) {
      reportData.results.forEach((result) => {
        if (result.suites) {
          this.processSuites(connector, result.suites, result.file);
        }
      });
    }
  }

  processSuites(connector, suites, file) {
    const connectorData = this.summaryData.connectors[connector];

    suites.forEach((suite) => {
      if (suite.tests) {
        suite.tests.forEach((test) => {
          const testInfo = {
            title: test.title,
            fullTitle: test.fullTitle || `${suite.title} - ${test.title}`,
            state: test.state,
            duration: test.duration || 0,
            file: file,
            error: test.err
              ? {
                  message: test.err.message,
                  stack: test.err.stack,
                  diff: test.err.diff,
                }
              : null,
          };

          connectorData.tests.push(testInfo);

          // Track by file
          if (!connectorData.testsByFile[file]) {
            connectorData.testsByFile[file] = {
              passed: 0,
              failed: 0,
              skipped: 0,
              pending: 0,
              tests: [],
            };
          }

          connectorData.testsByFile[file].tests.push(testInfo);
          connectorData.testsByFile[file][test.state || "pending"]++;

          // Track failed tests globally
          if (test.state === "failed") {
            this.summaryData.failedTests.push({
              connector,
              ...testInfo,
              screenshot: this.findScreenshot(connector, test.title),
              video: this.findVideo(connector, file),
            });
          }
        });
      }

      // Process nested suites
      if (suite.suites && suite.suites.length > 0) {
        this.processSuites(connector, suite.suites, file);
      }
    });
  }

  findScreenshot(connector, testTitle) {
    const screenshotDir = path.join(__dirname, "../screenshots", connector);
    if (!fs.existsSync(screenshotDir)) return null;

    const screenshots = fs.readdirSync(screenshotDir);
    const sanitizedTitle = testTitle.replace(/[^a-zA-Z0-9]/g, "-");

    const screenshot = screenshots.find((file) =>
      file.toLowerCase().includes(sanitizedTitle.toLowerCase())
    );

    return screenshot ? `/screenshots/${connector}/${screenshot}` : null;
  }

  findVideo(connector, testFile) {
    const videoDir = path.join(__dirname, "../cypress/videos", connector);
    if (!fs.existsSync(videoDir)) return null;

    const videos = fs.readdirSync(videoDir);
    const testFileName = path.basename(testFile, ".cy.js");

    const video = videos.find((file) => file.includes(testFileName));

    return video ? `/videos/${connector}/${video}` : null;
  }

  calculateMetrics() {
    // Calculate totals
    Object.values(this.summaryData.connectors).forEach((connector) => {
      this.summaryData.totalTests += connector.totalTests;
      this.summaryData.totalPassed += connector.passed;
      this.summaryData.totalFailed += connector.failed;
      this.summaryData.totalSkipped += connector.skipped;
      this.summaryData.totalPending += connector.pending;
      this.summaryData.executionTime += connector.executionTime;

      // Calculate rates
      connector.successRate =
        connector.totalTests > 0
          ? ((connector.passed / connector.totalTests) * 100).toFixed(2)
          : 0;
      connector.failureRate =
        connector.totalTests > 0
          ? ((connector.failed / connector.totalTests) * 100).toFixed(2)
          : 0;
    });

    // Calculate overall rates
    this.summaryData.overallSuccessRate =
      this.summaryData.totalTests > 0
        ? (
            (this.summaryData.totalPassed / this.summaryData.totalTests) *
            100
          ).toFixed(2)
        : 0;
    this.summaryData.overallFailureRate =
      this.summaryData.totalTests > 0
        ? (
            (this.summaryData.totalFailed / this.summaryData.totalTests) *
            100
          ).toFixed(2)
        : 0;
  }

  async generateSummaryReport() {
    const reportContent = this.formatSummaryReport();
    const outputPath = path.join(this.reportsDir, "summary-report.md");

    fs.writeFileSync(outputPath, reportContent);
    console.log(`📄 Summary report generated: ${outputPath}`);
  }

  formatSummaryReport() {
    let report = `# Cypress Test Summary Report

**Generated:** ${new Date(this.summaryData.timestamp).toLocaleString()}
**Total Execution Time:** ${this.formatDuration(this.summaryData.executionTime)}

## Overall Summary

- **Total Connectors Tested:** ${Object.keys(this.summaryData.connectors).length}
- **Total Tests:** ${this.summaryData.totalTests}
- **Passed:** ${this.summaryData.totalPassed} ✅
- **Failed:** ${this.summaryData.totalFailed} ❌
- **Skipped:** ${this.summaryData.totalSkipped} ⏭️
- **Pending:** ${this.summaryData.totalPending} ⏸️
- **Overall Success Rate:** ${this.summaryData.overallSuccessRate}%
- **Overall Failure Rate:** ${this.summaryData.overallFailureRate}%

## Connector Details

`;

    // Add connector details
    Object.entries(this.summaryData.connectors).forEach(([connector, data]) => {
      report += `### ${connector.toUpperCase()}

| Metric | Value |
|--------|-------|
| Total Tests | ${data.totalTests} |
| Passed | ${data.passed} ✅ |
| Failed | ${data.failed} ❌ |
| Skipped | ${data.skipped} ⏭️ |
| Pending | ${data.pending} ⏸️ |
| Success Rate | ${data.successRate}% |
| Failure Rate | ${data.failureRate}% |
| Execution Time | ${this.formatDuration(data.executionTime)} |

`;

      // Add test details by file
      if (Object.keys(data.testsByFile).length > 0) {
        report += `#### Test Files\n\n`;
        Object.entries(data.testsByFile).forEach(([file, fileData]) => {
          const fileName = path.basename(file);
          report += `- **${fileName}**: ${fileData.passed}✅ ${fileData.failed}❌ ${fileData.skipped}⏭️ ${fileData.pending}⏸️\n`;
        });
        report += "\n";
      }
    });

    // Add failed tests section
    if (this.summaryData.failedTests.length > 0) {
      report += `## Failed Tests Details\n\n`;

      this.summaryData.failedTests.forEach((test, index) => {
        report += `### ${index + 1}. ${test.fullTitle}

- **Connector:** ${test.connector}
- **File:** ${path.basename(test.file)}
- **Duration:** ${test.duration}ms
- **Error:** ${test.error?.message || "No error message available"}
`;

        if (test.screenshot) {
          report += `- **Screenshot:** [View Screenshot](${test.screenshot})\n`;
        }

        if (test.video) {
          report += `- **Video:** [View Recording](${test.video})\n`;
        }

        report += "\n";
      });
    }

    return report;
  }

  formatDuration(ms) {
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);

    if (hours > 0) {
      return `${hours}h ${minutes % 60}m ${seconds % 60}s`;
    } else if (minutes > 0) {
      return `${minutes}m ${seconds % 60}s`;
    } else {
      return `${seconds}s`;
    }
  }

  async generateDashboardData() {
    const dashboardDataPath = path.join(this.reportsDir, "dashboard-data.json");
    fs.writeFileSync(
      dashboardDataPath,
      JSON.stringify(this.summaryData, null, 2)
    );
    console.log(`📊 Dashboard data generated: ${dashboardDataPath}`);
  }
}

// Run the report generator
const generator = new CypressReportGenerator();
generator.generateReport();
