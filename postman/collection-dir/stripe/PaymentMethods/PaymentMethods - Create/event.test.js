// Validate status 2xx
pm.test("[POST]::/payment_methods - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test(
  "[POST]::/payment_methods - Content-Type is application/json",
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

// pm.collectionVariables - Set payment_method_id as variable for jsonData.payment_method_id
if (jsonData?.payment_method_id) {
  pm.collectionVariables.set("payment_method_id", jsonData.payment_method_id);
  console.log(
    "- use {{payment_method_id}} as collection variable for value",
    jsonData.payment_method_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{payment_method_id}}, as jsonData.payment_method_id is undefined.",
  );
}

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
