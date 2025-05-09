// Validate status 2xx
pm.test("[POST]::/organization - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test("[POST]::/organization - Content-Type is application/json", function () {
  pm.expect(pm.response.headers.get("Content-Type")).to.include(
    "application/json",
  );
});

// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}

// pm.collectionVariables - Set merchant_id as variable for jsonData.merchant_id
if (jsonData?.organization_id) {
  pm.collectionVariables.set("organization_id", jsonData.organization_id);
  console.log(
    "- use {{organization_id}} as collection variable for value",
    jsonData.organization_id,
  );
} else {
  console.log(
    "INFO - Unable to assign variable {{organization_id}}, as jsonData.organization_id is undefined.",
  );
}