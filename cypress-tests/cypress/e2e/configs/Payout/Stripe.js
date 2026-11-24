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
  account_type: "custom",
  business_type: "individual",
  capabilities_transfers: true,
  business_profile_mcc: 5734,
  business_profile_url: "https://accessible.stripe.com",
};

// Stripe test values that let a Custom account verify instantly, so the
// transfers capability becomes active and the payout runs end to end.
const individual_details = {
  external_account_account_holder_type: "individual",
  tos_acceptance_date: 1609459200,
  tos_acceptance_ip: "8.8.8.8",
  date_of_birth: "1901-01-01",
  individual_id_number: "000000000",
  individual_ssn_last_4: "0000",
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
      // A payout with an existing customer_id uses that customer's phone, and
      // Stripe Connect account creation rejects the default customer fixture's
      // phone ("999999999", "+65"). The valid case creates a customer with a
      // US phone first.
      VendorMetadataCustomer: {
        Request: {
          phone: "2015550123",
          phone_country_code: "+1",
        },
      },
      VendorMetadataValid: {
        Request: {
          ...ach_payout_request,
          // Stripe test address that passes address verification.
          billing: {
            address: { ...billing.address, line1: "address_full_match" },
          },
          metadata: {
            vendor_details: vendor_details,
            individual_details: individual_details,
          },
        },
        Response: {
          status: 200,
          body: {
            status: "requires_vendor_account_creation",
            payout_type: "bank",
          },
        },
      },
      RetrieveAfterAccountAttach: {
        Response: {
          status: 200,
          body: {
            status: "pending",
            payout_type: "bank",
          },
        },
      },
      VendorMetadataMissingIndividual: {
        Request: {
          ...ach_payout_request,
          metadata: {
            vendor_details: vendor_details,
          },
        },
        Response: {
          status: 422,
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
      VendorMetadataMissingVendor: {
        Request: {
          ...ach_payout_request,
          metadata: {
            individual_details: {},
          },
        },
        Response: {
          status: 422,
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
          status: 422,
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
        // undefined overrides the base fixture's metadata and is dropped when
        // the request body is serialized, so the key is absent on the wire.
        Request: { ...ach_payout_request, metadata: undefined },
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
