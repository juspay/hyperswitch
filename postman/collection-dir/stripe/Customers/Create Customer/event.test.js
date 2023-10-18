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

// Response body should have "customer_id"
pm.test(
  "[POST]::/customers - Content check if 'customer_id' exists",
  function () {
    pm.expect(typeof jsonData.customer_id !== "undefined").to.be.true;
  },
);

// Response body should have a minimum length of "1" for "customer_id"
if (jsonData?.customer_id) {
  pm.test(
    "[POST]::/customers - Content check if value of 'customer_id' has a minimum length of '1'",
    function () {
      pm.expect(jsonData.customer_id.length).is.at.least(1);
    },
  );
}

// pm.collectionVariables - Set customer_id as variable for jsonData.customer_id
if (jsonData?.customer_id) {
  pm.collectionVariables.set("customer_id", jsonData.customer_id);
  console.log(
    "- use {{customer_id}} as collection variable for value",
    jsonData.customer_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{customer_id}}, as jsonData.customer_id is undefined.",
  );
}
