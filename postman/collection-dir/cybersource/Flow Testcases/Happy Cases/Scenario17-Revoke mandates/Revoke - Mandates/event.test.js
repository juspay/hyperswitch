// Validate status 2xx
pm.test("[GET]::/payments/:id - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[GET]::/payments/:id - Content-Type is application/json", function () {
  pm.expect(pm.response.headers.get("Content-Type")).to.include(
    "application/json",
  );
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}


// Response body should have value "revoked" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]::/payments/:id - Content check if value for 'status' matches 'revoked'",
    function () {
      pm.expect(jsonData.status).to.eql("revoked");
    },
  );
}

// Response body should have "mandate_id"
pm.test(
  "[POST]::/payments - Content check if 'mandate_id' exists",
  function () {
    pm.expect(typeof jsonData.mandate_id !== "undefined").to.be.true;
  },
);

