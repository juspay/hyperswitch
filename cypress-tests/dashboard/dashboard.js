/* eslint-disable no-console */
/* global Chart */

// Dashboard JavaScript
let dashboardData = null;
let connectorChart = null;
let distributionChart = null;
let avgDurationChart = null;

// Convert connector name to PascalCase
function toPascalCase(str) {
  return str
    .toLowerCase()
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join("");
}

// Initialize the dashboard
document.addEventListener("DOMContentLoaded", () => {
  loadTheme();
  setupUI();
  loadDashboardData();
  setupEventListeners();

  // Refresh data every 30 seconds
  setInterval(loadDashboardData, 30000);
});

// Setup UI based on environment
function setupUI() {
  const isHosted = window.location.hostname === "integ.hyperswitch.io";

  // Show/hide report loader for hosted environment
  const reportLoader = document.getElementById("reportLoader");
  if (isHosted) {
    reportLoader.style.display = "flex";
  }
}

// Setup event listeners
function setupEventListeners() {
  document
    .getElementById("refreshBtn")
    .addEventListener("click", loadDashboardData);
  document
    .getElementById("loadLatestBtn")
    .addEventListener("click", loadLatestReport);
  document.getElementById("themeToggle").addEventListener("click", toggleTheme);
  document
    .getElementById("connectorFilter")
    .addEventListener("change", filterData);
  document
    .getElementById("statusFilter")
    .addEventListener("change", filterData);

  // Add event listeners for hosted environment features
  const loadReportBtn = document.getElementById("loadReportBtn");
  const reportNameInput = document.getElementById("reportNameInput");

  if (loadReportBtn) {
    loadReportBtn.addEventListener("click", loadSpecificReport);
  }

  if (reportNameInput) {
    // Allow Enter key to load report
    reportNameInput.addEventListener("keypress", (e) => {
      if (e.key === "Enter") {
        loadSpecificReport();
      }
    });
  }

  // Modal controls
  const modal = document.getElementById("testRunnerModal");
  const closeBtn = document.getElementsByClassName("close")[0];

  closeBtn.onclick = () => {
    modal.style.display = "none";
  };

  window.onclick = (event) => {
    if (event.target === modal) {
      modal.style.display = "none";
    }
  };

  document
    .getElementById("runTestBtn")
    .addEventListener("click", runIndividualTest);
  document
    .getElementById("testConnector")
    .addEventListener("change", updateTestFiles);
  document
    .getElementById("testFile")
    .addEventListener("change", updateTestCases);
}

// Load dashboard data
async function loadDashboardData() {
  try {
    let response;

    // Simple logic: local uses dashboard-data.json, hosted uses report_latest.json
    if (window.location.hostname === "integ.hyperswitch.io") {
      // Hosted environment - use latest report
      response = await fetch("./reports/report_latest.json");
    } else {
      // Local environment - use dashboard data
      response = await fetch("../cypress/reports/dashboard-data.json");
    }

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    dashboardData = await response.json();
    updateDashboard();
  } catch (error) {
    console.error("Error loading dashboard data:", error);
    const environment =
      window.location.hostname === "integ.hyperswitch.io" ? "hosted" : "local";
    showError(
      `Failed to load dashboard data for ${environment} environment. ${environment === "local" ? "Make sure to run the report generator first." : "Check if the latest report is available."}`
    );
  }
}

// Load latest report (for hosted environment)
async function loadLatestReport() {
  try {
    let response;

    if (window.location.hostname === "integ.hyperswitch.io") {
      // Hosted environment - load latest report
      response = await fetch("./reports/report_latest.json");
    } else {
      // Local environment - just reload the dashboard data
      response = await fetch("../cypress/reports/dashboard-data.json");
    }

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    dashboardData = await response.json();
    updateDashboard();

    // Show success message
    showSuccess("Latest report loaded successfully!");
  } catch (error) {
    console.error("Error loading latest report:", error);
    showError(
      "Failed to load latest report. Please check if the report is available."
    );
  }
}

// Load specific report by name (for hosted environment)
async function loadSpecificReport() {
  const reportNameInput = document.getElementById("reportNameInput");
  const reportName = reportNameInput.value.trim();

  if (!reportName) {
    showError("Please enter a report name.");
    return;
  }

  try {
    // Add .json extension if not present
    const fileName = reportName.endsWith(".json")
      ? reportName
      : `${reportName}.json`;
    const response = await fetch(`./reports/${fileName}`);

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    dashboardData = await response.json();
    updateDashboard();

    // Show success message
    showSuccess(`Report "${fileName}" loaded successfully!`);

    // Clear the input
    reportNameInput.value = "";
  } catch (error) {
    console.error("Error loading specific report:", error);
    showError(
      `Failed to load report "${reportName}". Please check if the report exists.`
    );
  }
}

// Update dashboard with loaded data
function updateDashboard() {
  if (!dashboardData) return;

  // Update last updated time (this is the test run timestamp)
  document.getElementById("lastUpdated").textContent =
    `Last Test Run: ${new Date(dashboardData.timestamp).toLocaleString()}`;

  // Update summary cards
  updateSummaryCards();

  // Update charts
  updateCharts();

  // Update connector filters
  updateConnectorFilters();

  // Apply filters
  filterData();

  // Setup collapsible failed tests
  setupFailedTestsCollapsible();
}

// Update summary cards
function updateSummaryCards() {
  // Count active connectors
  const activeConnectors = Object.entries(dashboardData.connectors).filter(
    ([, data]) => data.totalTests > 0
  ).length;

  // Update total connectors
  document.getElementById("totalConnectors").textContent = activeConnectors;

  // Merge skipped and pending
  const totalSkippedPending =
    dashboardData.totalSkipped + dashboardData.totalPending;

  document.getElementById("totalTests").textContent = dashboardData.totalTests;
  document.getElementById("totalPassed").textContent =
    dashboardData.totalPassed;
  document.getElementById("totalFailed").textContent =
    dashboardData.totalFailed;
  document.getElementById("totalSkipped").textContent = totalSkippedPending;
  document.getElementById("successRate").textContent =
    `${dashboardData.overallSuccessRate}%`;

  // Calculate and display failure rate
  const failureRate =
    dashboardData.totalTests > 0
      ? ((dashboardData.totalFailed / dashboardData.totalTests) * 100).toFixed(
          2
        )
      : 0;
  document.getElementById("failureRate").textContent = `${failureRate}%`;

  document.getElementById("executionTime").textContent = formatDuration(
    dashboardData.executionTime
  );
}

// Format duration from milliseconds
function formatDuration(ms) {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`;
  } else if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`;
  } else {
    return `${seconds}s`;
  }
}

// Get chart colors based on theme
function getChartColors() {
  const isDark = document.body.classList.contains("dark-theme");
  return {
    textColor: isDark ? "#e0e0e0" : "#212529",
    gridColor: isDark ? "#2d2d2d" : "#e9ecef",
    executionLineColor: "#f59e0b",
    scales: {
      x: {
        ticks: {
          color: isDark ? "#e0e0e0" : "#212529",
        },
        grid: {
          color: isDark ? "#2d2d2d" : "#e9ecef",
        },
      },
      y: {
        ticks: {
          color: isDark ? "#e0e0e0" : "#212529",
        },
        grid: {
          color: isDark ? "#2d2d2d" : "#e9ecef",
        },
      },
    },
  };
}

// Update charts
function updateCharts() {
  updateCombinedChart();
  updateDistributionChart();
  updateAvgDurationChart();
}

// Update combined chart (test results + execution time)
function updateCombinedChart() {
  const ctx = document.getElementById("connectorChart").getContext("2d");
  const colors = getChartColors();

  // Filter out connectors with no tests
  const activeConnectors = Object.entries(dashboardData.connectors).filter(
    ([, data]) => data.totalTests > 0
  );

  const connectorNames = activeConnectors.map(([name]) => name);

  // Calculate execution times for secondary axis
  const executionTimes = connectorNames.map((c) => {
    const connector = dashboardData.connectors[c];
    const totalTime = connector.executionTime || 0;
    return (totalTime / 1000 / 60).toFixed(2); // Convert to minutes
  });

  const datasets = [
    {
      label: "Passed",
      data: connectorNames.map((c) => dashboardData.connectors[c].passed),
      backgroundColor: "#04c38d",
      stack: "tests",
      yAxisID: "y",
    },
    {
      label: "Failed",
      data: connectorNames.map((c) => dashboardData.connectors[c].failed),
      backgroundColor: "#ef4444",
      stack: "tests",
      yAxisID: "y",
    },
    {
      label: "Skipped/Pending",
      data: connectorNames.map(
        (c) =>
          dashboardData.connectors[c].skipped +
          dashboardData.connectors[c].pending
      ),
      backgroundColor: "#3b82f6",
      stack: "tests",
      yAxisID: "y",
    },
    {
      label: "Execution Time (min)",
      data: executionTimes,
      type: "line",
      borderColor: colors.executionLineColor,
      backgroundColor: "transparent",
      borderWidth: 2,
      pointBackgroundColor: colors.executionLineColor,
      pointBorderColor: colors.executionLineColor,
      pointRadius: 4,
      yAxisID: "y1",
    },
  ];

  if (connectorChart) {
    connectorChart.destroy();
  }

  connectorChart = new Chart(ctx, {
    type: "bar",
    data: {
      labels: connectorNames.map((c) => toPascalCase(c)),
      datasets: datasets,
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      scales: {
        x: {
          stacked: true,
          ticks: {
            color: colors.textColor,
          },
          grid: {
            color: colors.gridColor,
          },
        },
        y: {
          stacked: true,
          beginAtZero: true,
          position: "left",
          title: {
            display: true,
            text: "Number of Tests",
            color: colors.textColor,
          },
          ticks: {
            color: colors.textColor,
          },
          grid: {
            color: colors.gridColor,
          },
        },
        y1: {
          beginAtZero: true,
          position: "right",
          title: {
            display: true,
            text: "Time (minutes)",
            color: colors.executionLineColor,
          },
          ticks: {
            color: colors.executionLineColor,
          },
          grid: {
            drawOnChartArea: false,
          },
        },
      },
      plugins: {
        legend: {
          position: "top",
          labels: {
            color: colors.textColor,
          },
        },
        tooltip: {
          mode: "index",
          intersect: false,
        },
      },
    },
  });
}

// Update distribution chart
function updateDistributionChart() {
  const ctx = document.getElementById("distributionChart").getContext("2d");
  const colors = getChartColors();

  const data = {
    labels: ["Passed", "Failed", "Skipped/Pending"],
    datasets: [
      {
        data: [
          dashboardData.totalPassed,
          dashboardData.totalFailed,
          dashboardData.totalSkipped + dashboardData.totalPending,
        ],
        backgroundColor: ["#04c38d", "#ef4444", "#3b82f6"],
      },
    ],
  };

  if (distributionChart) {
    distributionChart.destroy();
  }

  distributionChart = new Chart(ctx, {
    type: "doughnut",
    data: data,
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: {
          position: "right",
          labels: {
            color: colors.textColor,
          },
        },
        tooltip: {
          callbacks: {
            label: function (context) {
              const label = context.label || "";
              const value = context.parsed || 0;
              const total = dashboardData.totalTests;
              const percentage = ((value / total) * 100).toFixed(1);
              return `${label}: ${value} (${percentage}%)`;
            },
          },
        },
      },
    },
  });
}

// Update average duration chart
function updateAvgDurationChart() {
  const ctx = document.getElementById("avgDurationChart").getContext("2d");
  const colors = getChartColors();

  // Filter out connectors with no tests
  const activeConnectors = Object.entries(dashboardData.connectors).filter(
    ([, data]) => data.totalTests > 0
  );

  const connectorNames = activeConnectors.map(([name]) => name);
  const avgDurations = connectorNames.map((c) => {
    const connector = dashboardData.connectors[c];
    const totalTime = connector.executionTime || 0;
    const totalTests = connector.totalTests || 1;
    return (totalTime / totalTests / 1000).toFixed(2); // Average time in seconds
  });

  if (avgDurationChart) {
    avgDurationChart.destroy();
  }

  avgDurationChart = new Chart(ctx, {
    type: "bar",
    data: {
      labels: connectorNames.map((c) => toPascalCase(c)),
      datasets: [
        {
          label: "Average Test Duration (seconds)",
          data: avgDurations,
          backgroundColor: "#04c38d",
          borderColor: "#059669",
          borderWidth: 1,
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      scales: {
        x: {
          ticks: {
            color: colors.textColor,
          },
          grid: {
            color: colors.gridColor,
          },
        },
        y: {
          beginAtZero: true,
          title: {
            display: true,
            text: "Time (seconds)",
            color: colors.textColor,
          },
          ticks: {
            color: colors.textColor,
          },
          grid: {
            color: colors.gridColor,
          },
        },
      },
      plugins: {
        legend: {
          display: false,
        },
        tooltip: {
          callbacks: {
            label: function (context) {
              return `${context.parsed.y} seconds`;
            },
          },
        },
      },
    },
  });
}

// Update connector filters
function updateConnectorFilters() {
  const filterSelect = document.getElementById("connectorFilter");
  filterSelect.innerHTML = '<option value="">All Connectors</option>';

  // Only show connectors with tests
  Object.entries(dashboardData.connectors)
    .filter(([, data]) => data.totalTests > 0)
    .forEach(([connector]) => {
      const option = document.createElement("option");
      option.value = connector;
      option.textContent = toPascalCase(connector);
      filterSelect.appendChild(option);
    });
}

// Filter data based on selections
function filterData() {
  const connectorFilter = document.getElementById("connectorFilter").value;
  const statusFilter = document.getElementById("statusFilter").value;

  // Update tables
  updateConnectorTables(connectorFilter);

  // Update failed tests
  updateFailedTests(connectorFilter, statusFilter);
}

// Update connector tables
function updateConnectorTables(connectorFilter) {
  const container = document.getElementById("connectorTables");
  container.innerHTML = "";

  Object.entries(dashboardData.connectors).forEach(([connector, data]) => {
    if (connectorFilter && connector !== connectorFilter) {
      return;
    }

    // Hide connectors with no tests executed
    if (data.totalTests === 0) {
      return;
    }

    const tableDiv = createConnectorTable(connector, data);
    container.appendChild(tableDiv);
  });
}

// Create connector table
function createConnectorTable(connector, data) {
  const div = document.createElement("div");
  div.className = "connector-table collapsed";
  div.dataset.connector = connector;

  const header = document.createElement("h3");
  const totalCount = data.totalTests;
  const passedCount = data.passed;
  const failedCount = data.failed;
  const skippedPendingCount = data.skipped + data.pending;

  // Create summary text
  const summaryParts = [];
  summaryParts.push(`${totalCount} tests`);
  if (passedCount > 0) summaryParts.push(`${passedCount} passed`);
  if (failedCount > 0)
    summaryParts.push(
      `<span style="color: var(--error-color)">${failedCount} failed</span>`
    );
  if (skippedPendingCount > 0)
    summaryParts.push(`${skippedPendingCount} skipped`);

  // Add execution time if available
  if (data.executionTime) {
    const timeInMinutes = (data.executionTime / 1000 / 60).toFixed(1);
    summaryParts.push(`${timeInMinutes} min`);
  }

  // Create properly aligned header with padding
  header.innerHTML = `
        <span style="min-width: 150px; display: inline-block;">${toPascalCase(connector)}</span>
        <span style="font-weight: normal; font-size: 16px; flex: 1;">(${summaryParts.join(", ")})</span>
        <button class="btn btn-secondary run-test-btn" onclick="event.stopPropagation(); openTestRunner('${connector}')">
            Run Tests
        </button>
    `;

  // Add click handler to toggle collapse
  header.addEventListener("click", (e) => {
    if (!e.target.classList.contains("run-test-btn")) {
      div.classList.toggle("collapsed");
    }
  });

  div.appendChild(header);

  const table = document.createElement("table");
  table.innerHTML = `
        <thead>
            <tr>
                <th>Test File</th>
                <th>Passed</th>
                <th>Failed</th>
                <th>Skipped/Pending</th>
                <th>Total</th>
            </tr>
        </thead>
        <tbody>
            ${Object.entries(data.testsByFile)
              .map(([file, fileData]) => {
                const fileName = file.split("/").pop();
                const skippedPending = fileData.skipped + fileData.pending;
                return `
                        <tr>
                            <td>${fileName}</td>
                            <td><span class="status passed">${fileData.passed}</span></td>
                            <td><span class="status failed">${fileData.failed}</span></td>
                            <td><span class="status skipped">${skippedPending}</span></td>
                            <td>${fileData.passed + fileData.failed + skippedPending}</td>
                        </tr>
                    `;
              })
              .join("")}
        </tbody>
        <tfoot>
            <tr>
                <th>Total</th>
                <th><span class="status passed">${data.passed}</span></th>
                <th><span class="status failed">${data.failed}</span></th>
                <th><span class="status skipped">${data.skipped + data.pending}</span></th>
                <th>${data.totalTests}</th>
            </tr>
        </tfoot>
    `;

  div.appendChild(table);
  return div;
}

// Create a failed test element
function createFailedTestElement(test, index) {
  const testDiv = document.createElement("div");
  testDiv.className = "failed-test-item";

  // Create header
  const header = document.createElement("h4");
  header.textContent = `${index + 1}. ${test.fullTitle}`;
  testDiv.appendChild(header);

  // Create test details
  const details = createTestDetails(test);
  testDiv.appendChild(details);

  // Add error message if exists
  if (test.error) {
    const errorDiv = createErrorMessage(test.error);
    testDiv.appendChild(errorDiv);
  }

  // Add media links
  const mediaLinks = createMediaLinks(test);
  testDiv.appendChild(mediaLinks);

  return testDiv;
}

// Create test details section
function createTestDetails(test) {
  const detailsDiv = document.createElement("div");
  detailsDiv.className = "failed-test-details";

  const details = [
    { label: "Connector", value: toPascalCase(test.connector) },
    { label: "File", value: test.file.split("/").pop() },
    { label: "Duration", value: `${test.duration}ms` },
  ];

  details.forEach(({ label, value }) => {
    const detailDiv = document.createElement("div");
    detailDiv.innerHTML = `<strong>${label}:</strong> <span>${value}</span>`;
    detailsDiv.appendChild(detailDiv);
  });

  return detailsDiv;
}

// Create error message section
function createErrorMessage(error) {
  const errorDiv = document.createElement("div");
  errorDiv.className = "error-message";
  errorDiv.textContent = error.message || "No error message available";
  return errorDiv;
}

// Create media links section
function createMediaLinks(test) {
  const mediaDiv = document.createElement("div");
  mediaDiv.className = "media-links";

  const links = [];

  if (test.screenshot) {
    const screenshotLink = document.createElement("a");
    screenshotLink.href = test.screenshot;
    screenshotLink.target = "_blank";
    screenshotLink.textContent = "📸 View Screenshot";
    links.push(screenshotLink);
  }

  if (test.video) {
    const videoLink = document.createElement("a");
    videoLink.href = test.video;
    videoLink.target = "_blank";
    videoLink.textContent = "🎥 View Video";
    links.push(videoLink);
  }

  if (links.length === 0) {
    const noMediaSpan = document.createElement("span");
    noMediaSpan.style.color = "var(--text-secondary)";
    noMediaSpan.textContent = "No media available";
    mediaDiv.appendChild(noMediaSpan);
  } else {
    links.forEach((link) => mediaDiv.appendChild(link));
  }

  return mediaDiv;
}

// Update failed tests section
function updateFailedTests(connectorFilter, statusFilter) {
  const container = document.getElementById("failedTestsList");
  container.innerHTML = "";

  // Filter failed tests based on connector
  let filteredTests = dashboardData.failedTests;
  if (connectorFilter) {
    filteredTests = filteredTests.filter(
      (test) => test.connector === connectorFilter
    );
  }

  // If status filter is set and not "failed", hide the failed tests section
  if (statusFilter && statusFilter !== "failed") {
    document.getElementById("failedTestsSection").style.display = "none";
    return;
  }

  if (filteredTests.length === 0) {
    container.innerHTML = '<p class="text-center">No failed tests! 🎉</p>';
    document.getElementById("failedTestsSection").style.display =
      filteredTests.length === 0 && !connectorFilter ? "none" : "block";
    return;
  }

  document.getElementById("failedTestsSection").style.display = "block";

  // Update failed count in header
  document.getElementById("failedCount").textContent =
    `(${filteredTests.length})`;

  filteredTests.forEach((test, index) => {
    const testDiv = createFailedTestElement(test, index);
    container.appendChild(testDiv);
  });
}

// Open test runner modal
window.openTestRunner = function (connector) {
  const modal = document.getElementById("testRunnerModal");
  modal.style.display = "block";

  // Populate connector dropdown
  const connectorSelect = document.getElementById("testConnector");
  connectorSelect.innerHTML = "";

  if (connector) {
    const option = document.createElement("option");
    option.value = connector;
    option.textContent = toPascalCase(connector);
    connectorSelect.appendChild(option);
  } else {
    Object.keys(dashboardData.connectors)
      .filter((conn) => dashboardData.connectors[conn].totalTests > 0)
      .forEach((conn) => {
        const option = document.createElement("option");
        option.value = conn;
        option.textContent = toPascalCase(conn);
        connectorSelect.appendChild(option);
      });
  }

  updateTestFiles();
};

// Update test files dropdown
function updateTestFiles() {
  const connector = document.getElementById("testConnector").value;
  const fileSelect = document.getElementById("testFile");
  fileSelect.innerHTML = '<option value="">Select a test file</option>';

  if (connector && dashboardData.connectors[connector]) {
    Object.keys(dashboardData.connectors[connector].testsByFile).forEach(
      (file) => {
        const option = document.createElement("option");
        option.value = file;
        option.textContent = file.split("/").pop();
        fileSelect.appendChild(option);
      }
    );
  }

  updateTestCases();
}

// Update test cases dropdown
function updateTestCases() {
  const connector = document.getElementById("testConnector").value;
  const file = document.getElementById("testFile").value;
  const caseSelect = document.getElementById("testCase");
  caseSelect.innerHTML = '<option value="">All tests in file</option>';

  if (connector && file && dashboardData.connectors[connector]) {
    const tests = dashboardData.connectors[connector].testsByFile[file].tests;
    tests.forEach((test) => {
      const option = document.createElement("option");
      option.value = test.title;
      option.textContent = test.title;
      caseSelect.appendChild(option);
    });
  }
}

// Run individual test
async function runIndividualTest() {
  const connector = document.getElementById("testConnector").value;
  const file = document.getElementById("testFile").value;
  const testCase = document.getElementById("testCase").value;
  const output = document.getElementById("testOutput");

  if (!connector || !file) {
    alert("Please select a connector and test file");
    return;
  }

  output.innerHTML = '<div class="loading"></div> Running test...';

  try {
    // Construct the command
    const fileName = file.split("/").pop();
    const spec = testCase
      ? `--spec "**/spec/**/${fileName}" --grep "${testCase}"`
      : `--spec "**/spec/**/${fileName}"`;

    const command = `CYPRESS_CONNECTOR="${connector}" npm run cypress:ci -- ${spec}`;

    // For demo purposes, we'll show the command
    output.innerHTML = `
            <strong>Command to run:</strong>
            <pre>${command}</pre>

            <p>To run this test, execute the above command in your terminal.</p>

            <p><em>Note: Real-time test execution requires a backend service to execute commands and stream results.</em></p>
        `;
  } catch (error) {
    output.innerHTML = `<div style="color: red;">Error: ${error.message}</div>`;
  }
}

// Show error message
function showError(message) {
  const container = document.querySelector(".container");
  const errorDiv = document.createElement("div");
  errorDiv.className = "error-message";
  errorDiv.style.position = "fixed";
  errorDiv.style.top = "20px";
  errorDiv.style.right = "20px";
  errorDiv.style.zIndex = "1001";
  errorDiv.textContent = message;

  container.appendChild(errorDiv);

  setTimeout(() => {
    errorDiv.remove();
  }, 5000);
}

// Show success message
function showSuccess(message) {
  const container = document.querySelector(".container");
  const successDiv = document.createElement("div");
  successDiv.className = "success-message";
  successDiv.style.position = "fixed";
  successDiv.style.top = "20px";
  successDiv.style.right = "20px";
  successDiv.style.zIndex = "1001";
  successDiv.style.backgroundColor = "#04c38d";
  successDiv.style.color = "white";
  successDiv.style.padding = "12px 20px";
  successDiv.style.borderRadius = "4px";
  successDiv.style.boxShadow = "0 2px 10px rgba(0,0,0,0.1)";
  successDiv.textContent = message;

  container.appendChild(successDiv);

  setTimeout(() => {
    successDiv.remove();
  }, 5000);
}

// Load theme from localStorage
function loadTheme() {
  const savedTheme = localStorage.getItem("dashboardTheme");
  if (savedTheme === "dark") {
    document.body.classList.add("dark-theme");
  }
}

// Toggle theme
function toggleTheme() {
  document.body.classList.toggle("dark-theme");
  const isDark = document.body.classList.contains("dark-theme");
  localStorage.setItem("dashboardTheme", isDark ? "dark" : "light");

  // Update charts if they exist
  if (connectorChart || distributionChart || avgDurationChart) {
    updateCharts();
  }
}

// Setup collapsible failed tests section
function setupFailedTestsCollapsible() {
  const failedSection = document.getElementById("failedTestsSection");
  const header = failedSection.querySelector("h2");

  header.addEventListener("click", () => {
    failedSection.classList.toggle("collapsed");
  });
}
