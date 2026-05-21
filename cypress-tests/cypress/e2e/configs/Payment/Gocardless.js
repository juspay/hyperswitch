const requiredFields = {
  payment_methods: [
    {
      payment_method: "bank_debit",
      payment_method_types: [
        {
          payment_method_type: "sepa",
          recurring_enabled: false,
          installment_payment_enabled: false,
        },
        {
          payment_method_type: "becs",
          recurring_enabled: false,
          installment_payment_enabled: false,
        },
        {
          payment_method_type: "ach",
          recurring_enabled: false,
          installment_payment_enabled: false,
        },
      ],
    },
  ],
};

export const connectorDetails = {
  pm_list: {
    PmListResponse: {
      PmListNull: {
        payment_methods: [],
      },
      pmListDynamicFieldWithoutBilling: requiredFields,
      pmListDynamicFieldWithBilling: requiredFields,
      pmListDynamicFieldWithNames: requiredFields,
      pmListDynamicFieldWithEmail: requiredFields,
    },
  },
};
