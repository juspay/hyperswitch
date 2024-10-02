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
    // Verify the "Generate reports" button is present and visible.
    // Verify the "Customize columns" button is present and visible.
  });
});
