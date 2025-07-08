// Validate status 400
pm.test("[POST]::/payments/:id/cancel - Status code is 400", function () {
  pm.response.to.have.status(400);
});

// Validate if response header has matching content-type
pm.test(
  "[POST]::/payments/:id/cancel - Content-Type is application/json",
  function () {
    pm.expect(pm.response.headers.get("Content-Type")).to.include(
      "application/json",
    );
  },
);

// Validate if response has JSON Body
pm.test("[POST]::/payments/:id/cancel - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});

// Validate if response has error body
pm.test("[POST]::/payments/:id/cancel - Response has error body", function () {
  pm.expect(pm.response.json()).to.have.property("error");
});

// Validate if response has "invalid_request" error type
pm.test(
  "[POST]::/payments/:id/cancel - Response has 'invalid_request' error type",
  function () {
    pm.expect(pm.response.json().error).to.have.property("type", "invalid_request");
  },
);

// Validate if response has "You cannot cancel this payment because it has status succeeded" error message
pm.test(
  "[POST]::/payments/:id/cancel - Response has 'You cannot cancel this payment because it has status succeeded' error message",
  function () {
    pm.expect(pm.response.json().error).to.have.property(
      "message",
      "You cannot cancel this payment because it has status succeeded",
    );
  },
);