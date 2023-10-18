// Validate status 2xx
pm.test("[POST]::/payments/:id/confirm - Status code is 2xx", function () {
  pm.response.to.be.success;
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

// pm.collectionVariables - Set mandate_id as variable for jsonData.mandate_id
if (jsonData?.mandate_id) {
  pm.collectionVariables.set("mandate_id", jsonData.mandate_id);
  console.log(
    "- use {{mandate_id}} as collection variable for value",
    jsonData.mandate_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{mandate_id}}, as jsonData.mandate_id is undefined.",
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

// Response body should have value "requires_customer_action" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]::/payments/:id/confirm - Content check if value for 'status' matches 'requires_customer_action'",
    function () {
      pm.expect(jsonData.status).to.eql("requires_customer_action");
    },
  );
}

// Response body should have "next_action.redirect_to_url"
pm.test(
  "[POST]::/payments - Content check if 'next_action.redirect_to_url' exists",
  function () {
    pm.expect(typeof jsonData.next_action.redirect_to_url !== "undefined").to.be
      .true;
  },
);

// Response body should have value "eps" for "payment_method_type"
if (jsonData?.payment_method_type) {
  pm.test(
    "[POST]::/payments/:id/confirm - Content check if value for 'payment_method_type' matches 'eps'",
    function () {
      pm.expect(jsonData.payment_method_type).to.eql("eps");
    },
  );
}

// Response body should have value "adyen" for "connector"
if (jsonData?.connector) {
  pm.test(
    "[POST]::/payments/:id/confirm - Content check if value for 'connector' matches 'adyen'",
    function () {
      pm.expect(jsonData.connector).to.eql("adyen");
    },
  );
}
