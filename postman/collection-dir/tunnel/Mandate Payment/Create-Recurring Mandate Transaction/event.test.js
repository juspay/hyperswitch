// Validate status 2xx
pm.test("[POST]::/payments - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response has JSON Body
pm.test("[POST]::/payments - Response has JSON Body", function () {
  pm.response.to.have.jsonBody();
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}

// pm.collectionVariables - Set order_id as variable for jsonData.order_id
if (jsonData?.order_id) {
  pm.collectionVariables.set("order_id", jsonData.order_id);
  console.log(
    "- use {{order_id}} as collection variable for value",
    jsonData.order_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{order_id}}, as jsonData.payment_id is undefined.",
  );
}

// pm.collectionVariables - Set order_id as variable for jsonData.mandate_id
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

// Response body should have value "CHARGED" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]::/payments - Content check if value for 'status' matches 'CHARGED'",
    function () {
      pm.expect(jsonData.status).to.eql("CHARGED");
    },
  );
}

// Response body should have "payment.authentication.url"
pm.test(
  "[POST]::/payments - Content check if 'payment.authentication.url' exists",
  function () {
    pm.expect(typeof jsonData.payment.authentication.url !== "undefined").to.be
      .true;
  },
);

// Response body should contain order_id
if (jsonData?.order_id) {
  pm.test(
    "[POST]::/payments - Content check if 'order_id' is present and not empty",
    function () {
      pm.expect(jsonData.order_id).to.be.a("string").and.not.empty;
    },
  );
} else {
  pm.test(
    "[POST]::/payments - Response body does not contain 'order_id' field",
    function () {
      pm.expect.fail("The 'order_id' field was not found in the response body.");
    },
  );
}

//Response body should have "payment.authentication" object
pm.test("Authentication object is not null", function () {
    pm.expect(jsonData.payment.authentication).to.not.be.null;
});

pm.test("Authentication object has 'url' property", function () {
    pm.expect(jsonData.payment.authentication).to.have.property('url');
});

const targetUrl = jsonData.payment.authentication.url; 

const expectedSegment = "/v2/pay/finish";

pm.test(`URL contains "${expectedSegment}"`, function () {
    pm.expect(targetUrl).to.include(expectedSegment);
});
