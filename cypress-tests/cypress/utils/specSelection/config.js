/**
 * Data behind payment-method based spec selection. Logic lives in `index.js`.
 */

/**
 * Payment methods a spec can be tagged with, and the vocabulary of
 * `CONNECTOR_PAYMENT_METHODS` below.
 */
export const PAYMENT_METHODS = Object.freeze([
  "bank_debit",
  "bank_redirect",
  "bank_transfer",
  "card",
  "card_redirect",
  "crypto",
  "gift_card",
  "mobile_payment",
  "open_banking",
  "pay_later",
  "real_time_payment",
  "reward",
  "upi",
  "voucher",
  "wallet",
]);

/**
 * Payment methods supported by connectors that do not support cards.
 *
 * This table is opt-in: a connector listed here runs only the specs matching
 * its payment methods, and any connector *not* listed runs every spec. Card
 * connectors are therefore deliberately absent — add one only when trimming its
 * run is worth maintaining the entry.
 *
 * Card-shaped methods (credit, debit, network token) all collapse into `card`;
 * the card specs do not distinguish them.
 */
export const CONNECTOR_PAYMENT_METHODS = Object.freeze({
  affirm: ["pay_later"],
  bitpay: ["crypto"],
  calida: ["wallet"],
  cashtocode: ["reward"],
  checkbook: ["bank_transfer"],
  coingate: ["crypto"],
  cryptopay: ["crypto"],
  facilitapay: ["bank_transfer"],
  gigadat: ["bank_redirect"],
  globepay: ["wallet"],
  iatapay: ["bank_redirect", "real_time_payment", "upi"],
  inespay: ["bank_debit"],
  itaubank: ["bank_transfer"],
  klarna: ["pay_later"],
  loonio: ["bank_redirect"],
  mifinity: ["wallet"],
  payjustnow: ["pay_later"],
  payjustnowinstore: ["pay_later"],
  paystack: ["bank_redirect"],
  plaid: ["open_banking"],
  prophetpay: ["card_redirect"],
  volt: ["bank_redirect"],
});

/**
 * Payment spec -> the payment methods it exercises, from `PAYMENT_METHODS`.
 *
 * A spec runs when the connector supports *any* of its payment methods, so
 * specs covering several (`52-AlternativePayments`) stay enabled for connectors
 * supporting only one of them.
 *
 * `null` means "always run". `01`-`03` seed the shared `globalState` (merchant,
 * API key, customer, merchant connector account) that every later spec reads
 * through the `getGlobalState` task, so dropping them breaks the whole run.
 * Specs missing from this map are treated the same way, so a newly added spec
 * stays visible until it is tagged here.
 */
const PAYMENT_SPEC_METHODS = Object.freeze({
  "01-AccountCreate.cy.js": null,
  "02-CustomerCreate.cy.js": null,
  "03-ConnectorCreate.cy.js": null,

  // Merchant-platform specs: they exercise merchant/customer/profile/dispute
  // APIs rather than a payment method, but they are re-run for every connector.
  // Tagging them `card` keeps that coverage on the card suites while sparing
  // single-payment-method connectors a run that yields no connector signal.
  "00-CoreFlows.cy.js": ["card"],
  "35-CustomerListTests.cy.js": ["card"],
  "37-DiffCheckValidation.cy.js": ["card"],
  "58-DisputeTests.cy.js": ["card"],
  "72-AcquirerConfigs.cy.js": ["card"],

  "04-NoThreeDSAutoCapture.cy.js": ["card"],
  "05-ThreeDSAutoCapture.cy.js": ["card"],
  "06-NoThreeDSManualCapture.cy.js": ["card"],
  "07-VoidPayment.cy.js": ["card"],
  "08-SyncPayment.cy.js": ["card"],
  "09-RefundPayment.cy.js": ["card"],
  "10-OrderDetails.cy.js": ["card"],
  "11-SyncRefund.cy.js": ["card"],
  "12-CreateSingleuseMandate.cy.js": ["card"],
  "13-CreateMultiuseMandate.cy.js": ["card"],
  "14-ListAndRevokeMandate.cy.js": ["card"],
  "15-SaveCardFlow.cy.js": ["card"],
  "16-ZeroAuthMandate.cy.js": ["card"],
  "17-ThreeDSManualCapture.cy.js": ["card"],
  "18-BankTransfers.cy.js": ["bank_transfer"],
  "19-BankRedirect.cy.js": ["bank_redirect"],
  "20-Wallet.cy.js": ["wallet"],
  "21-MandatesUsingPMID.cy.js": ["card"],
  "22-MandatesUsingNTIDProxy.cy.js": ["card"],
  "23-UPI.cy.js": ["upi"],
  "24-Variations.cy.js": ["card"],
  "25-PaymentMethods.cy.js": ["card"],
  "26-ConnectorAgnosticNTID.cy.js": ["card"],
  "27-SessionCall.cy.js": ["card", "wallet"],
  "28-DeletedCustomerPsyncFlow.cy.js": ["card"],
  "29-BusinessProfileConfigs.cy.js": ["card"],
  "30-IncrementalAuth.cy.js": ["card"],
  "31-Overcapture.cy.js": ["card"],
  "32-RealTimePayment.cy.js": ["real_time_payment"],
  "33-DDCRaceCondition.cy.js": ["card"],
  "34-ManualRetry.cy.js": ["card"],
  "36-PaymentsEligibilityAPI.cy.js": ["card"],
  "38-RewardPayment.cy.js": ["reward"],
  "39-CardInstallments.cy.js": ["card"],
  "40-CryptoPayment.cy.js": ["crypto"],
  "41-ExternalVault.cy.js": ["card"],
  "42-CardPaymentBlocking.cy.js": ["card"],
  "43-AutoRetries.cy.js": ["card"],
  "44-CardRedirect.cy.js": ["card_redirect"],
  "45-GiftCardPayment.cy.js": ["gift_card"],
  "46-RequiresCVV.cy.js": ["card"],
  "47-AuthenticationServiceEligibility.cy.js": ["card"],
  "48-BillingDescriptor.cy.js": ["card"],
  "49-PartnerMerchantIdentifier.cy.js": ["card"],
  "50-PayLater.cy.js": ["pay_later"],
  "51-ThreeDSRoutingRegionUAS.cy.js": ["card"],
  "52-ExternalThreeDS.cy.js": ["card"],
  "53-BankDebit.cy.js": ["bank_debit"],
  "54-ConnectorTestingData.cy.js": ["card"],
  "55-ExtendAuthorization.cy.js": ["card"],
  "56-L2L3DataProcessing.cy.js": ["card"],
  "57-StepUpRetries.cy.js": ["card"],
  "59-ManualPaymentUpdate.cy.js": ["card"],
  "60-PollConfig.cy.js": ["card"],
  "61-RefundManualUpdate.cy.js": ["card"],
  "62-StepUpAuth.cy.js": ["card"],
  "63-WalletMandates.cy.js": ["wallet"],
  "64-CardTestingGuard.cy.js": ["card"],
  "65-MITWithLimitedCardData.cy.js": ["card"],
  "66-PaymentLink.cy.js": ["card"],
  "67-PaymentWebhook.cy.js": ["card"],
  "68-PartialAuthorization.cy.js": ["card"],
  "69-RefundWebhook.cy.js": ["card"],
  "70-ExtendedCardInfo.cy.js": ["card"],
  "71-FeatureMetadata.cy.js": ["card"],
  "73-AlternativePayments.cy.js": ["wallet", "pay_later"],
  "74-ClientSessionValidation.cy.js": ["card"],
  "75-FRM.cy.js": ["card"],
  "76-IframeRedirection.cy.js": ["card"],
  "77-MerchantRedirectMethod.cy.js": ["upi"],
  "78-MultipleCapture.cy.js": ["card"],
  "79-PaymentResponseHash.cy.js": ["card"],
  "80-RelayOperations.cy.js": ["card"],
  "81-UseBillingAsPaymentMethodBilling.cy.js": ["card"],
  "82-VoucherPayment.cy.js": ["voucher"],
  "83-ClearPanRetry.cy.js": ["card"],
  "84-DelayedSessionToken.cy.js": ["wallet"],
  "85-OpenBanking.cy.js": ["open_banking"],
  "86-RefundType.cy.js": ["card"],
  "87-DynamicFields.cy.js": ["card"],
  "88-VaultTokenizationDisable.cy.js": ["card"],
});

/**
 * Services whose specs are selected per connector, keyed by the
 * `cypress:<service>` npm script that runs them.
 *
 * Only `payments` is wired today; the remaining `cypress:*` scripts keep their
 * plain spec globs. Register a service here to filter it too.
 */
export const SERVICES = Object.freeze({
  payments: {
    specDir: "cypress/e2e/spec/Payment",
    specMethods: PAYMENT_SPEC_METHODS,
  },
});
