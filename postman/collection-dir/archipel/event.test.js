// Set response object as internal variable
let jsonData = {};
try {
  jsonData = pm.response.json();
} catch (e) {}

// pm.collectionVariables - Set payment_id as variable for jsonData.payment_id
if (jsonData?.payment_id) {
  pm.collectionVariables.set("payment_id", jsonData.payment_id);
  console.log("[LOG]::payment_id - " + jsonData.payment_id);
}

console.log("[LOG]::x-request-id - " + pm.response.headers.get("x-request-id"));
