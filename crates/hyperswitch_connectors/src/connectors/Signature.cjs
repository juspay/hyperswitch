const crypto = require("crypto");

const payloadObj = {
  amount: 100,
  currency: "ZAR",
  merchant_reference: "cuytbjmhvngczsdxfcg",
  // callback_url: "https://yourposserver/callbacks/payjustnow",
  callback_url: "null",
  //   "items": []
    "items": [
        {
            "name": "UGG Boots Purple Size 6",
            // "name": null,
            // "name": "",
            // "sku": "rwget",
            "sku": null,
            "quantity": 1,
            "price": 100
        }
    ]
};

// const payloadObj = {
//   token: "c0f224b8624b943d2e2a6c6e918a48f1",
//   // merchant_reference: "ewrefwrefrwrfegefertrwegtgegetegtbrgn"
// };

// const payloadObj = {
// //   "request_id": "ref_fjshvdgeijlfru",
//   "merchant_reference": "rgyuetb",
//   "token": "b649dd9217af602cb6b5c122fac7c8f5",
//   "amount": 1
// //   "refund_reference": "string"
// }

// JSON encode without escaped slashes (PHP: JSON_UNESCAPED_SLASHES)
let payload = JSON.stringify(payloadObj).replace(/\\\//g, "/");

// Remove all whitespace (same as PHP preg_replace("/\s+/", "", $payload))
let stringToSign = payload.replace(/\s+/g, "");

// PayJustNow API key
const key = "secret";

// Generate HMAC SHA256 signature
const signature = crypto.createHmac("sha256", key)
                        .update(stringToSign)
                        .digest("hex");

console.log("String To Sign:", stringToSign);
console.log("Signature:", signature);
