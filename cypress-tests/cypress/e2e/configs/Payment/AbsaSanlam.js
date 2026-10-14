import { getCustomExchange } from "./Modifiers";
import { standardBillingAddress } from "./Commons";

/*
 * Pre-FRM scenarios for the sanlam_payshield -> absa_sanlam bank_debit
 * (eft_debit_order) payment chain (see cypress/e2e/spec/Payment/57-FRMBankDebit.cy.js).
 * The Payshield sandbox flags an amount over 1,000,000 cents as fraud, same
 * threshold as the payout pre-FRM chain. frm_message details containing
 * non-deterministic ids (frm_transaction_id) are asserted separately in the
 * spec rather than baked into a fixed Response body.
 */
const eftDebitOrder = {
  account_number: "12313131",
  branch_code: "2131234",
  bank_account_holder_name: "John Doe",
  bank_type: "savings",
  bank_name: "absa",
};

export const connectorDetails = {
  bank_debit_pm: {
    FRMLegit: getCustomExchange({
      Request: {
        amount: 5000,
        currency: "ZAR",
        payment_method: "bank_debit",
        payment_method_type: "eft_debit_order",
        payment_method_data: {
          bank_debit: {
            eft_debit_order: eftDebitOrder,
          },
        },
        billing: standardBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    }),
    FRMFraud: getCustomExchange({
      Request: {
        amount: 10000000,
        currency: "ZAR",
        payment_method: "bank_debit",
        payment_method_type: "eft_debit_order",
        payment_method_data: {
          bank_debit: {
            eft_debit_order: eftDebitOrder,
          },
        },
        billing: standardBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "fraud",
        },
      },
    }),
    FRMTransactionFailureFailClosed: getCustomExchange({
      Request: {
        amount: 5000,
        currency: "ZAR",
        payment_method: "bank_debit",
        payment_method_type: "eft_debit_order",
        payment_method_data: {
          bank_debit: {
            eft_debit_order: eftDebitOrder,
          },
        },
        billing: standardBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "transaction_failure",
        },
      },
    }),
    FRMTransactionFailureFailOpen: getCustomExchange({
      Request: {
        amount: 5000,
        currency: "ZAR",
        payment_method: "bank_debit",
        payment_method_type: "eft_debit_order",
        payment_method_data: {
          bank_debit: {
            eft_debit_order: eftDebitOrder,
          },
        },
        billing: standardBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    }),
  },
};
