// Validate status 2xx
pm.test(
    "[POST]::/account/:account_id/connectors - Status code is 2xx",
    function () {
        pm.response.to.have.status(200);
    },
);

// Validate if response header has matching content-type
pm.test(
    "[POST]::/account/:account_id/connectors - Content-Type is application/json",
    function () {
        pm.expect(pm.response.headers.get("Content-Type")).to.include(
            "application/json",
        );
    },
);

// Set response object as internal variable
let jsonData = pm.response.json();

// Validate if connector_type is authentication_processor
pm.test(
    "[POST]::/account/:account_id/connectors - Validate connector_type",
    function () {
        pm.expect(jsonData.connector_type).to.equal(
            "authentication_processor",
        );
    },
);

// Validate if connector_name is netcetera
pm.test(
    "[POST]::/account/:account_id/connectors - Validate connector_name",
    function () {
        pm.expect(jsonData.connector_name).to.equal(
            "netcetera",
        );
    },
);

// Validate if auth_type is CertificateAuth
pm.test(
    "[POST]::/account/:account_id/connectors - Validate auth_type is CertificateAuth",
    function () {
        pm.expect(jsonData.connector_account_details.auth_type).to.equal(
            "CertificateAuth"
        );
    },
);

// Validate if metadata contains merchant_configuration_id
pm.test(
    "[POST]::/account/:account_id/connectors - Validate metadata contains merchant_configuration_id",
    function () {
        pm.expect(jsonData.metadata.merchant_configuration_id).to.not.be.null;
    },
);

// Validate if test_mode is true
pm.test(
    "[POST]::/account/:account_id/connectors - Validate is not test_mode",
    function () {
        pm.expect(jsonData.test_mode).to.be.true;
    },
);

// Validate if disabled is false
pm.test(
    "[POST]::/account/:account_id/connectors - Validate is not disabled",
    function () {
        pm.expect(jsonData.disabled).to.be.false;
    },
);

// Validate if status is active
pm.test(
    "[POST]::/account/:account_id/connectors - Validate have active status",
    function () {
        pm.expect(jsonData.status).to.equal(
            "active"
        );
    },
);