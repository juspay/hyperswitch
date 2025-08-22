// Validate status 400
pm.test("[POST]::/payments/:id/capture - Status code is 400", function () {
  pm.response.to.have.status(400);
});

// Validate if response header has matching content-type
pm.test(
  "[POST]::/payments/:id/capture - Content-Type is application/json",
  function () {
    pm.expect(pm.response.headers.get("Content-Type")).to.include(
      "application/json",
    );
  },
);

// Validate if response has JSON Body
pm.test("[POST]::/payments/:id/capture - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}

// pm.collectionVariables - Set payment_id as variable for jsonData.payment_id
if (jsonData?.payment_id) {
  pm.collectionVariables.set("payment_id", jsonData.payment_id);
  console.log(
    "- use {{payment_id}} as collection variable for value",
    jsonData.payment_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{payment_id}}, as jsonData.payment_id is undefined.",
  );
}

// pm.collectionVariables - Set client_secret as variable for jsonData.client_secret
if (jsonData?.client_secret) {
  pm.collectionVariables.set("client_secret", jsonData.client_secret);
  console.log(
    "- use {{client_secret}} as collection variable for value",
    jsonData.client_secret,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{client_secret}}, as jsonData.client_secret is undefined.",
  );
}

// Validate if response has error body
pm.test("[POST]::/payments/:id/confirm - Response has error body", function () {
  pm.expect(jsonData).to.have.property("error");
});

// Validate if error type is "invalid_request"
pm.test(
  "[POST]::/payments/:id/confirm - Error type is 'invalid_request'",
  function () {
    pm.expect(jsonData.error).to.have.property("type", "invalid_request");
  },
);

// Validate if error message is "This Payment could not be captured because it has a payment.status of failed. The expected state is requires_capture, partially_captured_and_capturable, processing"
pm.test(
  "[POST]::/payments/:id/confirm - Error message is 'This Payment could not be captured because it has a payment.status of failed. The expected state is requires_capture, partially_captured_and_capturable, processing'",
  function () {
    pm.expect(jsonData.error).to.have.property(
      "message",
      "This Payment could not be captured because it has a payment.status of failed. The expected state is requires_capture, partially_captured_and_capturable, processing",
    );
  },
);