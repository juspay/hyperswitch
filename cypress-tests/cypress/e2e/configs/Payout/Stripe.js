const billing = {
  address: {
    line1: "100 Main St",
    city: "San Francisco",
    state: "CA",
    zip: "94105",
    country: "US",
    first_name: "John",
    last_name: "Doe",
  },
};

// Stripe test bank account (ACH) from Stripe's testing documentation.
const ach_payout_method_data = {
  bank: {
    bank_name: "Stripe Test Bank",
    bank_account_number: "000123456789",
    bank_routing_number: "110000000",
    bank_country_code: "US",
  },
};

const ach_payout_request = {
  amount: 1000,
  currency: "USD",
  payout_type: "bank",
  payout_method_data: ach_payout_method_data,
  billing: billing,
};

// Stripe Connect vendor details. account_type must be one of
// custom / express / standard (PayoutAccountType in
// hyperswitch_connectors/src/connectors/stripe/transformers/connect.rs).
const vendor_details = {
  account_type: "standard",
  business_type: "individual",
};

// Stripe payouts read payout metadata as PayoutVendorAccountDetails
// ({ vendor_details, individual_details }, both required). Metadata that
// carries either key but fails to parse is rejected with the parse error;
// metadata without either key is treated as absent and the Stripe connector
// then reports vendor_details as missing (PR #14846).
const missingVendorDetailsError = {
  status: 400,
  body: {
    error: {
      type: "invalid_request",
      message: "Missing required param: vendor_details",
      code: "IR_04",
    },
  },
};

export const connectorDetails = {
  bank_transfer_pm: {
    ach_bank_transfer: {
      VendorMetadataValid: {
        Request: {
          ...ach_payout_request,
          metadata: {
            vendor_details: vendor_details,
            individual_details: {},
          },
        },
        Response: {
          status: 200,
          body: {
            status: "requires_vendor_account_creation",
            payout_type: "bank",
            error_code: null,
            error_message: null,
          },
        },
      },
      VendorMetadataMissingIndividualDetails: {
        Request: {
          ...ach_payout_request,
          metadata: {
            vendor_details: vendor_details,
          },
        },
        Response: {
          status: 400,
          body: {
            error: {
              type: "invalid_request",
              message:
                "Invalid payout metadata: missing field `individual_details`",
              code: "IR_06",
            },
          },
        },
      },
      VendorMetadataMissingVendorDetails: {
        Request: {
          ...ach_payout_request,
          metadata: {
            individual_details: {},
          },
        },
        Response: {
          status: 400,
          body: {
            error: {
              type: "invalid_request",
              message:
                "Invalid payout metadata: missing field `vendor_details`",
              code: "IR_06",
            },
          },
        },
      },
      VendorMetadataInvalidFieldType: {
        Request: {
          ...ach_payout_request,
          metadata: {
            vendor_details: {
              ...vendor_details,
              account_type: 123,
            },
            individual_details: {},
          },
        },
        Response: {
          status: 400,
          body: {
            error: {
              type: "invalid_request",
              message:
                "Invalid payout metadata: invalid type: integer `123`, expected a string",
              code: "IR_06",
            },
          },
        },
      },
      VendorMetadataUnrelatedKeys: {
        Request: {
          ...ach_payout_request,
          metadata: {
            ref: "123",
          },
        },
        Response: missingVendorDetailsError,
      },
      VendorMetadataAbsent: {
        Request: ach_payout_request,
        Response: missingVendorDetailsError,
      },
      VendorMetadataNull: {
        Request: {
          ...ach_payout_request,
          metadata: null,
        },
        Response: missingVendorDetailsError,
      },
    },
  },
};
