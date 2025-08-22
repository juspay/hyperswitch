// Validate status 2xx
pm.test(
  "[POST]::/account/:account_id/business_profile - Status code is 2xx",
  function () {
    pm.response.to.be.success;
  },
);

// Validate if response header has matching content-type
pm.test(
  "[POST]::/account/:account_id/business_profile - Content-Type is application/json",
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

// Validate if response has correct merchant_id
pm.test(
  "[POST]::/account/:account_id/business_profile - Validate merchant_id",
  function () {
    pm.expect(jsonData.merchant_id).to.eql(
      pm.collectionVariables.get("merchant_id")
    );
  }
);

// Validate if response has correct profile_id
pm.test(
  "[POST]::/account/:account_id/business_profile - Validate profile_id",
  function () {
    pm.expect(jsonData.profile_id).to.eql(
      pm.collectionVariables.get("profile_id")
    );
  }
);

// Validate if authentication_connector_details is present
pm.test(
  "[POST]::/account/:account_id/business_profile - Validate authentication_connector_details is present",
  function () {
    pm.expect(jsonData.authentication_connector_details).to.be.an("object");
  }
);

// Validate if authentication_connector_details has netcetera as authentication_connectors
pm.test(
  "[POST]::/account/:account_id/business_profile - Validate authentication_connector_details has netcetera",
  function () {
    pm.expect(jsonData.authentication_connector_details.authentication_connectors[0]).to.eql(
      "netcetera",
    );
  },
);

// Validate if authentication_connector_details has three_ds_requestor_url and three_ds_requestor_app_url
pm.test(
  "[POST]::/account/:account_id/business_profile - Validate authentication_connector_details has three_ds_requestor_url and three_ds_requestor_app_url",
  function () {
    pm.expect(jsonData.authentication_connector_details).to.have.property(
      "three_ds_requestor_url",
    );
    pm.expect(jsonData.authentication_connector_details).to.have.property(
      "three_ds_requestor_app_url",
    );
  },
);

// Validate if is_connector_agnostic_mit_enabled true or not
pm.test(
  "[POST]::/account/:account_id/business_profile - Validate is_connector_agnostic_mit_enabled is true",
  function () {
    pm.expect(jsonData.is_connector_agnostic_mit_enabled).to.eql(true);
  }
);