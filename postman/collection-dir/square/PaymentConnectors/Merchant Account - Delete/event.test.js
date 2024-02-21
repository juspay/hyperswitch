// Validate status 2xx
pm.test("[DELETE]::/accounts/:id - Status code is 2xx", function () {
  pm.response.to.be.success;
});

// Validate if response header has matching content-type
pm.test(
  "[DELETE]::/accounts/:id - Content-Type is application/json",
  function () {
    pm.expect(pm.response.headers.get("Content-Type")).to.include(
      "application/json",
    );
  },
);

// Response Validation
const schema = {
  type: "object",
  description: "Merchant Account",
  required: ["merchant_id", "deleted"],
  properties: {
    merchant_id: {
      type: "string",
      description: "The identifier for the MerchantAccount object.",
      maxLength: 255,
      example: "y3oqhf46pyzuxjbcn2giaqnb44",
    },
    deleted: {
      type: "boolean",
      description:
        "Indicates the deletion status of the Merchant Account object.",
      example: true,
    },
  },
};

// Validate if response matches JSON schema
pm.test("[DELETE]::/accounts/:id - Schema is valid", function () {
  pm.response.to.have.jsonSchema(schema, {
    unknownFormats: ["int32", "int64", "float", "double"],
  });
});
