// Validate status 4xx
pm.test("[POST]::/refunds - Status code is 4xx", function () {
  pm.response.to.be.error;
});

// Validate if response header has matching content-type
pm.test("[POST]::/refunds - Content-Type is application/json", function () {
  pm.expect(pm.response.headers.get("Content-Type")).to.include(
    "application/json",
  );
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}

// pm.collectionVariables - Set refund_id as variable for jsonData.payment_id
if (jsonData?.refund_id) {
  pm.collectionVariables.set("refund_id", jsonData.refund_id);
  console.log(
    "- use {{refund_id}} as collection variable for value",
    jsonData.refund_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{refund_id}}, as jsonData.refund_id is undefined.",
  );
}

// Response body should have "error"
pm.test(
  "[POST]::/payments/:id/confirm - Content check if 'error' exists",
  function () {
    pm.expect(typeof jsonData.error !== "undefined").to.be.true;
  },
);

// Response body should have value "invalid_request" for "error type"
if (jsonData?.error?.type) {
  pm.test(
    "[POST]::/payments/:id/confirm - Content check if value for 'error.type' matches 'invalid_request'",
    function () {
      pm.expect(jsonData.error.type).to.eql("invalid_request");
    },
  );
}

// Response body should have value "invalid_request" for "error type"
if (jsonData?.error?.message) {
  pm.test(
    "[POST]::/payments - Content check if value for 'error.message' matches 'This Payment could not be refund because it has a status of requires_confirmation. The expected state is succeeded, partially_captured'",
    function () {
      pm.expect(jsonData.error.message).to.eql(
        "This Payment could not be refund because it has a status of requires_confirmation. The expected state is succeeded, partially_captured",
      );
    },
  );
}
