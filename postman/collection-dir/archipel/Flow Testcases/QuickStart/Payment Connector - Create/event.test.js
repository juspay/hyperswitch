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

pm.test(
    "[POST]::/account/:account_id/connectors - Validate connector_type",
    function () {
        pm.expect(jsonData.connector_type).to.equal(
            "payment_processor",
        );
    },
);

pm.test(
    "[POST]::/account/:account_id/connectors - Validate connector_name",
    function () {
        pm.expect(jsonData.connector_name).to.equal(
            "archipel",
        );
    },
);

pm.test(
    "[POST]::/account/:account_id/connectors - Validate merchant_connector_id",
    function () {
        // pm.collectionVariables - Set merchant_connector_id as variable for jsonData.merchant_connector_id
        if (jsonData?.merchant_connector_id) {
            pm.collectionVariables.set(
                "merchant_connector_id",
                jsonData.merchant_connector_id,
            );
            console.log(
                "- use {{merchant_connector_id}} as collection variable for value",
                jsonData.merchant_connector_id,
            );
        } else {
            console.log(
                "INFO - Unable to assign variable {{merchant_connector_id}}, as jsonData.merchant_connector_id is undefined.",
            );
        }
        pm.expect(jsonData.merchant_connector_id).to.equal(
            pm.collectionVariables.get("merchant_connector_id")
        );
    },
);

pm.test(
    "[POST]::/account/:account_id/connectors - Validate auth_type is NoKey",
    function () {
        pm.expect(jsonData.connector_account_details.auth_type).to.equal(
            "NoKey"
        );
    },
);

pm.test(
    "[POST]::/account/:account_id/connectors - Validate metadata contains tenant_id",
    function () {
        pm.expect(jsonData.metadata.tenant_id).to.not.be.null;
    },
);

pm.test(
    "[POST]::/account/:account_id/connectors - Validate is not test_mode",
    function () {
        pm.expect(jsonData.test_mode).to.be.false;
    },
);


pm.test(
    "[POST]::/account/:account_id/connectors - Validate is not disabled",
    function () {
        pm.expect(jsonData.test_mode).to.be.false;
    },
);

pm.test(
    "[POST]::/account/:account_id/connectors - Validate have active status",
    function () {
        pm.expect(jsonData.status).to.equal(
            "active"
        );
    },
);

