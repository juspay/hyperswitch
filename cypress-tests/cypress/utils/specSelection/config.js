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
  datatrans: ["card"],
  facilitapay: ["bank_transfer"],
  fiservcommercehub: ["card"],
  gigadat: ["bank_redirect"],
  givepayments: ["card"],
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
  tsys_transit: ["card"],
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
  "34-CustomerListTests.cy.js": ["card"],
  "36-DiffCheckValidation.cy.js": ["card"],
  "47-DisputeTests.cy.js": ["card"],
  "52-AcquirerConfigs.cy.js": ["card"],

  "04-NoThreeDSAutoCapture.cy.js": ["card"],
  "05-ThreeDSAutoCapture.cy.js": ["card"],
  "06-NoThreeDSManualCapture.cy.js": ["card"],
  "07-VoidPayment.cy.js": ["card"],
  "08-SyncPayment.cy.js": ["card"],
  "09-RefundPayment.cy.js": ["card"],
  "10-SyncRefund.cy.js": ["card"],
  "11-CreateSingleuseMandate.cy.js": ["card"],
  "12-CreateMultiuseMandate.cy.js": ["card"],
  "13-ListAndRevokeMandate.cy.js": ["card"],
  "14-SaveCardFlow.cy.js": ["card"],
  "15-ZeroAuthMandate.cy.js": ["card"],
  "16-ThreeDSManualCapture.cy.js": ["card"],
  "17-BankTransfers.cy.js": ["bank_transfer"],
  "18-BankRedirect.cy.js": ["bank_redirect"],
  "19-Wallet.cy.js": ["wallet"],
  "20-MandatesUsingPMID.cy.js": ["card"],
  "21-MandatesUsingNTIDProxy.cy.js": ["card"],
  "22-UPI.cy.js": ["upi"],
  "23-Variations.cy.js": ["card"],
  "24-PaymentMethods.cy.js": ["card"],
  "25-ConnectorAgnosticNTID.cy.js": ["card"],
  "26-SessionCall.cy.js": ["card", "wallet"],
  "27-DeletedCustomerPsyncFlow.cy.js": ["card"],
  "28-BusinessProfileConfigs.cy.js": ["card"],
  "29-IncrementalAuth.cy.js": ["card"],
  "30-Overcapture.cy.js": ["card"],
  "31-RealTimePayment.cy.js": ["real_time_payment"],
  "32-DDCRaceCondition.cy.js": ["card"],
  "33-ManualRetry.cy.js": ["card"],
  "35-PaymentsEligibilityAPI.cy.js": ["card"],
  "37-RewardPayment.cy.js": ["reward"],
  "38-CardInstallments.cy.js": ["card"],
  "39-CryptoPayment.cy.js": ["crypto"],
  "40-ExternalVault.cy.js": ["card"],
  "41-CardPaymentBlocking.cy.js": ["card"],
  "42-AutoRetries.cy.js": ["card"],
  "42-CardRedirect.cy.js": ["card_redirect"],
  "42-GiftCardPayment.cy.js": ["gift_card"],
  "42-RequiresCVV.cy.js": ["card"],
  "43-AuthenticationServiceEligibility.cy.js": ["card"],
  "43-BillingDescriptor.cy.js": ["card"],
  "43-PartnerMerchantIdentifier.cy.js": ["card"],
  "43-PayLater.cy.js": ["pay_later"],
  "43-ThreeDSRoutingRegionUAS.cy.js": ["card"],
  "44-ExternalThreeDS.cy.js": ["card"],
  "45-BankDebit.cy.js": ["bank_debit"],
  "46-ConnectorTestingData.cy.js": ["card"],
  "46-ExtendAuthorization.cy.js": ["card"],
  "46-L2L3DataProcessing.cy.js": ["card"],
  "46-StepUpRetries.cy.js": ["card"],
  "47-ManualPaymentUpdate.cy.js": ["card"],
  "47-PollConfig.cy.js": ["card"],
  "47-RefundManualUpdate.cy.js": ["card"],
  "47-StepUpAuth.cy.js": ["card"],
  "47-WalletMandates.cy.js": ["wallet"],
  "48-CardTestingGuard.cy.js": ["card"],
  "48-MITWithLimitedCardData.cy.js": ["card"],
  "48-PaymentLink.cy.js": ["card"],
  "49-PaymentWebhook.cy.js": ["card"],
  "50-PartialAuthorization.cy.js": ["card"],
  "50-RefundWebhook.cy.js": ["card"],
  "51-ExtendedCardInfo.cy.js": ["card"],
  "51-FeatureMetadata.cy.js": ["card"],
  "52-AlternativePayments.cy.js": ["wallet", "pay_later"],
  "52-ClientSessionValidation.cy.js": ["card"],
  "52-FRM.cy.js": ["card"],
  "52-IframeRedirection.cy.js": ["card"],
  "52-MerchantRedirectMethod.cy.js": ["upi"],
  "52-MultipleCapture.cy.js": ["card"],
  "52-OrderDetails.cy.js": ["card"],
  "52-PaymentResponseHash.cy.js": ["card"],
  "52-RelayOperations.cy.js": ["card"],
  "52-UseBillingAsPaymentMethodBilling.cy.js": ["card"],
  "52-VoucherPayment.cy.js": ["voucher"],
  "53-ClearPanRetry.cy.js": ["card"],
  "53-DelayedSessionToken.cy.js": ["wallet"],
  "54-OpenBanking.cy.js": ["open_banking"],
  "54-RefundType.cy.js": ["card"],
  "55-DynamicFields.cy.js": ["card"],
  "56-VaultTokenizationDisable.cy.js": ["card"],
  "54-ConnectorAgnosticMandates.cy.js": ["card"],
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
