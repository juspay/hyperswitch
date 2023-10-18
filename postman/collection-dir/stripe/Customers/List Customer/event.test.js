// Validate status 2xx
pm.test("[POST]::/customers - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[POST]::/customers - Content-Type is application/json", function () {
  pm.expect(pm.response.headers.get("Content-Type")).to.include(
    "application/json",
  );
});

// Validate if response has JSON Body
pm.test("[POST]::/customers - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}


// Response body should have a minimum length of "1" for "customer_id"
if (jsonData?.customer_id) {
  pm.test(
    "[POST]::/customers - Content check if value of 'customer_id' has a minimum length of '2'",
    function () {
      pm.expect(jsonData.customer_id.length).is.at.least(2);
    },
  );
}


// Define the regular expression pattern to match customer_id
var customerIdPattern = /^[a-zA-Z0-9_]+$/;

// Define an array to store the validation results
var validationResults = [];

// Iterate through the JSON array
jsonData.forEach(function(item, index) {
    if (item.hasOwnProperty("customer_id")) {
        if (customerIdPattern.test(item.customer_id)) {
            validationResults.push("customer_id " + item.customer_id + " is valid.");
        } else {
            validationResults.push("customer_id " + item.customer_id + " is not valid.");
        }
    } else {
        validationResults.push("customer_id is missing for item at index " + index);
    }
});

// Check if any customer_id is not valid and fail the test if necessary
if (validationResults.some(result => !result.includes("is valid"))) {
    pm.test("Customer IDs validation failed: " + validationResults.join(", "), function() {
        pm.expect(false).to.be.true;
    });
} else {
    pm.test("All customer IDs are valid: " + validationResults.join(", "), function() {
        pm.expect(true).to.be.true;
    });
}
