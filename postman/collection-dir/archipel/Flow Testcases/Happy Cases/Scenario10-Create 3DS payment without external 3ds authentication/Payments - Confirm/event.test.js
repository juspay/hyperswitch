// Validate status 400
pm.test("[POST]::/payments/:id/confirm - Status code is 400", function () {
  pm.response.to.have.status(400);
});

// Validate if response header has matching content-type
pm.test(
  "[POST]::/payments/:id/confirm - Content-Type is application/json",
  function () {
    pm.expect(pm.response.headers.get("Content-Type")).to.include(
      "application/json",
    );
  },
);

// Validate if response has JSON Body
pm.test("[POST]::/payments/:id/confirm - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}

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

// Validate if error message is "Payment method type not supported"
pm.test(
  "[POST]::/payments/:id/confirm - Error message is 'Payment method type not supported'",
  function () {
    pm.expect(jsonData.error).to.have.property(
      "message",
      "Payment method type not supported",
    );
  },
);

// Validate if error reason is "Selected 3DS authentication method is not supported by archipel"
pm.test(
  "[POST]::/payments/:id/confirm - Error reason is 'Selected 3DS authentication method is not supported by archipel'",
  function () {
    pm.expect(jsonData.error).to.have.property(
      "reason",
      "Selected 3DS authentication method is not supported by archipel",
    );
  },
);