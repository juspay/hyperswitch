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


// Response body should have value "active" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]::/payments - Content check if value for 'status' matches 'active'",
    function () {
      pm.expect(jsonData.status).to.eql("active");
    },
  );
}

pm.test("[POST]::/payments - Verify last 4 digits of the card", function () {
    var jsonData = pm.response.json();
    pm.expect(jsonData.card.last4_digits).to.eql("4242");
});

pm.test("[POST]::/payments - Verify card expiration month", function () {
    var jsonData = pm.response.json();
    pm.expect(jsonData.card.card_exp_month).to.eql("10");
});

pm.test("[POST]::/payments - Verify card expiration year", function () {
    var jsonData = pm.response.json();
    pm.expect(jsonData.card.card_exp_year).to.eql("25");
});



