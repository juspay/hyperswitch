// Validate status 2xx
pm.test(
  "[GET]::/payment_methods/:merchant_id - Status code is 2xx",
  function () {
    pm.response.to.be.success;
  },
);

// Validate if response header has matching content-type
pm.test(
  "[GET]::/payment_methods/:merchant_id - Content-Type is application/json",
  function () {
    pm.expect(pm.response.headers.get("Content-Type")).to.include(
      "application/json",
    );
  },
);

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}


// Response body should have value "new_mandate" for "payment_type"
if (jsonData?.payment_type) {
  pm.test(
    "[POST]::/payments - Content check if value for 'payment_type' matches 'new_mandate'",
    function () {
      pm.expect(jsonData.payment_type).to.eql("new_mandate");
    },
  );
}
