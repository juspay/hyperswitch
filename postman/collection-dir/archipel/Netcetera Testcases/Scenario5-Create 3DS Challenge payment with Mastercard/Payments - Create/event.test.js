// Validate status 2xx
pm.test("[POST]::/payments - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[POST]::/payments - Content-Type is application/json", function () {
  pm.expect(pm.response.headers.get("Content-Type")).to.include(
    "application/json",
  );
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

// Verify if 'external_3ds_authentication_attempted' is true
pm.test("[POST]::/payments - 'external_3ds_authentication_attempted' should be true", function () {
  pm.expect(jsonData.external_3ds_authentication_attempted).to.be.true;
});

// Response body should have value "requires_customer_action" for "status"
if (jsonData?.status) {
  pm.test(
    "[POST]::/payments - Content Verify if value for 'status' matches 'requires_customer_action'",
    function () {
      pm.expect(jsonData.status).to.eql("requires_customer_action");
    },
  );
}

// Verify if 'next_action' exists
pm.test("[POST]::/payments - Should contain 'next_action'", function () {
  pm.expect(jsonData).to.have.property("next_action");
});

// Verify if 'next_action.type' is 'three_ds_invoke'
pm.test("[POST]::/payments - 'next_action.type' should be 'three_ds_invoke'", function () {
  pm.expect(jsonData.next_action.type).to.eql("three_ds_invoke");
});

// Verify if 'three_ds_data' exists inside 'next_action'
pm.test("[POST]::/payments - Should contain 'three_ds_data' under 'next_action'", function () {
  pm.expect(jsonData.next_action).to.have.property("three_ds_data");
});

// Verify if 'three_ds_authentication_url' exists
pm.test("[POST]::/payments - Should contain 'three_ds_authentication_url'", function () {
  pm.expect(jsonData.next_action.three_ds_data).to.have.property("three_ds_authentication_url");
});

// Verify if 'three_ds_authorize_url' exists
pm.test("[POST]::/payments - Should contain 'three_ds_authorize_url'", function () {
  pm.expect(jsonData.next_action.three_ds_data).to.have.property("three_ds_authorize_url");
});

// Verify if 'three_ds_method_url' exists
pm.test("[POST]::/payments - Should contain 'three_ds_method_url'", function () {
  pm.expect(jsonData.next_action.three_ds_data.three_ds_method_details).to.have.property("three_ds_method_url");
});

// Verify if 'three_ds_method_data' exists and is not empty
// Carries session-specific metadata for fingerprinting — must exist and be valid Base64.
pm.test("[POST]::/payments - Should contain non-empty 'three_ds_method_data'", function () {
  pm.expect(jsonData.next_action.three_ds_data.three_ds_method_details).to.have.property("three_ds_method_data").that.is.a('string').and.is.not.empty;
});

// Verify if 'poll_id' exists inside 'poll_config'
// Polling is required to check 3DS status asynchronously while Netcetera finishes auth.
pm.test("[POST]::/payments - Should contain 'poll_id' inside 'poll_config'", function () {
  pm.expect(jsonData.next_action.three_ds_data.poll_config).to.have.property("poll_id");
});

// Verify if 'message_version' exists and is 2.1.0 or higher
// Ensures compatibility with supported Netcetera protocol versions (2.1.0 and above).
pm.test("[POST]::/payments - 'message_version' should exist and be >= 2.1.0", function () {
  const version = jsonData.next_action.three_ds_data.message_version;
  pm.expect(version).to.match(/^2\.(1|2|3)(\.\d+)?$/);
});

// Verify if 'directory_server_id' exists
// Identifies the card network’s Directory Server (e.g., Visa = A000000003) for Netcetera routing.
pm.test("[POST]::/payments - Should contain 'directory_server_id'", function () {
  pm.expect(jsonData.next_action.three_ds_data).to.have.property("directory_server_id").that.is.a('string');
});

// Assert 'external_authentication_details' exists
pm.test("[POST]::/payments - Should contain 'external_authentication_details'", function () {
  pm.expect(jsonData).to.have.property("external_authentication_details");
});

// Assert 'status' is 'pending' at the start of 3DS
pm.test("[POST]::/payments - external_authentication_details.status should be 'pending'", function () {
  pm.expect(jsonData.external_authentication_details.status).to.eql("pending");
});

// Assert 'ds_transaction_id' exists (required for Netcetera 3DS tracking)
pm.test("[POST]::/payments - Should contain 'ds_transaction_id'", function () {
  pm.expect(jsonData.external_authentication_details.ds_transaction_id).to.be.a('string').and.is.not.empty;
});

// Assert 'version' is present and valid
pm.test("[POST]::/payments - Should contain 3DS version", function () {
  pm.expect(jsonData.external_authentication_details.version).to.match(/^2\.(1|2|3)(\.\d+)?$/);
});
