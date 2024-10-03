const dateRangeOptions = [
  "Last 30 Mins",
  "Last 1 Hour",
  "Last 2 Hours",
  "Today",
  "Yesterday",
  "Last 2 Days",
  "Last 7 Days",
  "Last 30 Days",
  "This Month",
  "Last Month",
];

const filterOptions = [
  "Connector",
  "Currency",
  "Status",
  "Payment Method",
  "Authentication Type",
  "Payment Method Type",
];

const paymentOperationTableColumns = [
  "S.No",
  "Payment ID",
  "Connector",
  "Connector Transaction ID",
  "Amount",
  "Payment Status",
  "Payment Method",
  "Payment Method Type",
  "Card Network",
  "Customer Email",
  "Merchant Order Reference Id",
  "Description",
  "Metadata",
  "Created",
];

describe("connector", () => {
  const username = "test@gmail.com";
  const password = "Test1441@41";

  // Login before each testcase
  beforeEach(() => {
    // TODO: Make this a custom command if it's not already
    cy.visit("https://app.hyperswitch.io/dashboard/payments");
    cy.url().should("include", "/login");

    cy.get("[data-testid=email]").type(username);
    cy.get("[data-testid=password]").type(password);
    cy.get('button[type="submit"]').click({ force: true });
    cy.get("[data-testid=skip-now]", { timeout: 3000 }).click({ force: true });

    cy.wait(3000);
    cy.url().should("include", "/dashboard/home");
  });

  it("Verify Default Elements on Payment Operations Page", () => {
    // Navigate to the "Payment Operations" page using the side menu.
    cy.navigateFromSideMenu("Operations/Payments");
    // Verify the URL to ensure the redirection to the "Payment Operations" page.
    cy.url().should("include", `/dashboard/payments`);
    // Verify the search box is present with the placeholder "Search payment id."

    cy.get('[data-id="Search payment id"]')
      .should("be.visible")
      .find("input")
      .should("have.attr", "placeholder", "Search payment id");

    // Verify the dropdown to select the time range is present.
    cy.get("[data-component-field-wrapper=field-start_time-end_time]")
      .should("be.visible")
      .within(() => {
        cy.get("button").click({ force: true });
      });
    // Verify the predefined options are present in the dropdown.
    cy.get('[data-date-picker-predifined="predefined-options"]').within(() => {
      dateRangeOptions.forEach((option) =>
        cy.get(`[data-daterange-dropdown-value="${option}"]`).should("have.text", option)
      );
    });

    // Verify the "Add Filters" button is present and visible.
    cy.clickOnElementWithText("button", "Add Filters");
    // Verify the filter options are present in the dropdown.
    cy.get('[role="menu"]').within(() => {
      filterOptions.forEach((option, index) => cy.get("button").eq(index).should("have.text", option));
    });
    // TODO: Add these assertions to the test case later once discussed with the team.
    // Verify the "Generate reports" button is present and visible.
    // Verify the "Customize columns" button is present and visible.
  });

  it("Verify Payments Displayed", () => {
    // Navigate to the "Payment Operations" page using the side menu.
    cy.navigateFromSideMenu("Operations/Payments");
    // Verify the URL to ensure the redirection to the "Payment Operations" page.
    cy.url().should("include", `/dashboard/payments`);
    // TODO: Add these assertions to the test case later once discussed with the team.
    // Make 3 payments.
    // Verify the payments are displayed in the table with valid payment details.
    // Verify the table contains the following columns.
    // select using table tag
    cy.get("table").within(() => {
      paymentOperationTableColumns.forEach((column) => cy.get("th").should("have.text", column));
    });
  });
});
