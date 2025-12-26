## 1. Purpose of This Tag

The `PAYMENT_METHODS:UPI:UPI_INAPP` tag encompasses the subsystem responsible for processing UPI In-App payments. This includes handling direct UPI payment flows, managing UPI mandates (One-Time and Recurring), integrating with various UPI payment gateways (e.g., BHIM, Razorpay, ICICI, EaseBuzz), orchestrating transaction lifecycle events (initiation, status checks, refunds), and ensuring data consistency and financial accuracy for UPI-related transactions, especially in the context of in-app experiences.

## 2. Domain & Engineering Philosophy

*   **State Management:** Transaction and Mandate states are meticulously tracked. For mandates, this includes distinct states for registration, first debit execution, and subsequent presentments. Status transitions are often gateway-specific, requiring custom interpretation of gateway response codes and statuses (e.g., `CREATE-INITIATED` for one-time mandates vs. general `PENDING`). `TxnDetail` and `Mandate` objects are central to state persistence. `GatewayTxnData` stores raw gateway responses and statuses.
*   **Configurability:** Feature toggles are extensively used, typically at the merchant level, often through `Cutover.isFeatureEnabled` checks. Examples include enabling/disabling S2S communication (`RAZORPAY_S2S_DISABLED_MERCHANTS`), enabling new SDK parameter flows, or specific `integrationVersion` flags. Gateway credentials and configurations (`MerchantGatewayAccount`) are critical for routing and processing.
*   **Robustness:**
    *   **Asynchronous Processing:** Reliance on webhooks for updating transaction and mandate statuses due to the inherent asynchronous nature of UPI.
    *   **Idempotency:** While not explicitly detailed, the use of unique transaction references (`tr`, `mandateRegRefId`) and `internalReferenceId` across PRs suggests an implicit need for idempotent operations, particularly for mandate registrations and first debits.
    *   **Error Handling:** Extensive use of `Either ErrorPaymentResp SdkParams` for function return types, `makeErrorGatewayResponse` for standardized error responses, and `LogUtils.forkErrorLog` for detailed error logging. Specific error conditions, like missing API keys or invalid gateway responses, lead to `JUSPAY_DECLINED` or `AUTHENTICATION_FAILED` statuses.
    *   **Fallback Mechanisms:** Routing to an AJAX URL for Razorpay UPI when S2S is disabled demonstrates a fallback strategy for specific merchant configurations or gateway limitations.
    *   **Outage Handling:** Explicit consideration for UPI In-App outages by modifying key structures (e.g., including bank code information in the outage key structure).
    *   **Decryption:** Secure handling of sensitive information through encryption/decryption of gateway request/response payloads (e.g., ICICI mandate intent requests).

## 3. System Components Involved

*   **Core Modules:**
    *   `src/PaymentGateways/`: Gateway-specific modules (e.g., `Razorpay.hs`, `IciciUpi.hs`, `EaseBuzz.hs`, `UpiController.hs`, `Billdesk.hs`).
    *   `src/Transaction/`: Core transaction processing logic.
    *   `src/Mandate/`: Mandate management and lifecycle.
    *   `src/Utils/`: Utility functions for logging, encryption, and common operations.
    *   `src/CommonGatewayService/`: Generic functions for handling gateway interactions and responses.
*   **Key Data Structures:**
    *   `TxnDetail`: Central object for transaction details.
    *   `Mandate`: Object representing mandate details (frequency, amount, dates, status).
    *   `TxnCardInfo`: Stores payment method-specific details, including `UpiPaymentSource`.
    *   `UpiPaymentSource`: Structured data type for UPI payment origin (upiIdentifier, upi_app, payer_vpa).
    *   `IciciIntentRequest`, `IciciIntentResponse`, `IciciUpiAuthParams`: Gateway-specific structures for ICICI mandate intent.
    *   `EaseBuzzUpiAuthParams`, `EaseBuzzUpiExecuteMandateRequest`: Gateway-specific structures for EaseBuzz mandates.
    *   `PaymentGatewayInfo`: Standardized structure for gateway response details.
    *   `SdkParams`: Parameters returned to the client for SDK-based interactions (e.g., deep links).
    *   `MerchantGatewayAccount`: Stores gateway credentials and configurations for a merchant.
    *   `SecondFactor`: Stores authentication parameters related to transactions, including mandate auth parameters.
*   **Orchestrators:**
    *   `getSdkParams`: Main entry point for generating client-side SDK parameters, often branching based on gateway and transaction type (e.g., mandate registration).
    *   `callMandateFirstExecution`: Drives the first debit process for mandates.
    *   `processFinalResponse`: Handles the final processing of transaction responses, including potential mandate first debit calls.
    *   `isPendingStatus`, `isPaymentSuccessful`, `isTransactionNotFound`: Generic functions whose implementations are gateway and mandate-type specific for status interpretation.
    *   `callGetStatusResponseForWebhook`: Central function for processing incoming webhooks.

## 4. Engineering Hotspots (Critical Areas)

*   **Validation Hotspots:**
    *   Checks for presence of API keys and required mandate parameters (e.g., `iciciApikey`).
    *   Validation of `SdkParams` for OTM mandates (`validateSdkParamsForOTM`).
    *   Amount validations for refunds (`getSplitRefundAmount`).
*   **Transformation Hotspots:**
    *   Mapping `Mandate` object fields to gateway-specific request payloads (e.g., `getCreateIntentMandatePayload`).
    *   Serializing `UpiPaymentSource` to JSON for specific gateways.
    *   Extracting VPA from `UpiPaymentSource` or raw strings.
    *   Transforming internal transaction statuses to gateway-specific statuses and vice-versa.
*   **Decision Hotspots:**
    *   `getSdkParams` branching logic based on `gateway` and `txnObjectType` (e.g., `TS.isEmandateRegisterTOT`).
    *   Conditional routing based on feature flags (e.g., `isS2SDisabledMerchant`).
    *   `isPendingStatus` and `isTxnNotFound` logic diverging for one-time mandates vs. other transaction types, or based on specific gateway response codes.
    *   Choice of `paymentSource` serialization (raw string vs. JSON `UpiPaymentSource`) based on gateway.

## 5. Reconstructed General Flow

1.  **Initiation:** A request for a UPI In-App payment or mandate registration is received, leading to a call to a high-level orchestrator like `getSdkParams`.
2.  **Gateway & Type Determination:** The system identifies the payment gateway and the transaction type (e.g., direct payment, mandate registration, first debit, split settlement).
3.  **Configuration & Credential Retrieval:** Merchant-specific configurations and gateway credentials are retrieved from `MerchantGatewayAccount`. Feature flags are checked.
4.  **Data Preparation:**
    *   Transaction details (`TxnDetail`), order information (`OrderReference`), and payment method details (`TxnCardInfo`, potentially with `UpiPaymentSource`) are prepared.
    *   For mandates, `Mandate` object details are extracted and validated.
    *   Sensitive payloads are encrypted if required by the gateway.
5.  **External Gateway Interaction:**
    *   A gateway-specific request payload is constructed (e.g., `IciciIntentRequest`).
    *   An API call is made to the payment gateway (e.g., `initMandateIntentPayRequest`, `initRazorpayWebCollectandIntentAjaxRequest`).
    *   Authentication parameters (e.g., `IciciUpiAuthParams`) are saved in `SecondFactor` for later use.
6.  **Response Processing:**
    *   The gateway response is received and decrypted (if applicable).
    *   The response is parsed and translated into internal success/failure/pending states.
    *   For intent flows, `SdkParams` containing deep links are generated.
7.  **State Update & Notification:**
    *   `TxnDetail` and `Mandate` objects are updated with the latest status.
    *   Payment Gateway Response (PGR) information is created.
    *   Webhooks may be triggered to notify downstream systems.
8.  **Asynchronous Status Handling (Webhooks/Polling):** For long-running or asynchronous processes (especially mandates), webhooks or background polling mechanisms (e.g., `callGetStatusResponseForWebhook`) process status updates from the gateway, which in turn update transaction/mandate states.
9.  **Refunds:** For refunds, the `makeRefund` function determines the refund method and constructs the necessary refund request, potentially extracting VPA from the `UpiPaymentSource` if it's a UPI refund.

## 6. Implementation Blueprint (Generalized)

*   **Preconditions:**
    *   `MerchantGatewayAccount` configured for the specific UPI gateway.
    *   `TxnDetail` and `OrderReference` objects representing the transaction.
    *   `TxnCardInfo` populated with payment method details (e.g., VPA, UPI app, potentially `UpiPaymentSource`).
    *   For mandates, a `Mandate` object with complete details (frequency, dates, amount limit).
    *   Required feature flags enabled for the merchant.

*   **Scaffolding (Pseudocode):**

    ```pseudocode
    function processUpiInAppTransaction(transaction_request, merchant_id, gateway_name):
        // 1. Retrieve configurations and credentials
        mga = getMerchantGatewayAccount(merchant_id, gateway_name)
        gateway_details = decodeGatewayCredentials(mga.accountDetails)

        // 2. Determine transaction type (direct pay, mandate register, first debit)
        if isMandateRegistration(transaction_request):
            mandate = getMandateDetails(transaction_request) // from DB or request
            return handleMandateRegistration(transaction_request, mandate, mga, gateway_details)
        else if isMandateFirstDebit(transaction_request):
            mandate = getMandateDetails(transaction_request)
            return handleMandateFirstDebit(transaction_request, mandate, mga, gateway_details)
        else:
            return handleDirectUpiPayment(transaction_request, mga, gateway_details)

    function handleMandateRegistration(transaction_request, mandate, mga, gateway_details):
        // 1. Prepare mandate-specific data
        mandate_reg_ref_id = generateUniqueRefId("mdtreg") + transaction_request.txnId
        remarks = getMandateRemarks(transaction_request)
        collect_by_date = calculateCollectByDate()

        // 2. Construct gateway-specific request payload
        // Example: For ICICI
        request_payload = buildIciciIntentRequest(mandate, gateway_details, mandate_reg_ref_id, remarks, collect_by_date)
        encrypted_payload = encrypt(request_payload)

        // 3. Save authentication parameters
        auth_params = { mandate_reg_ref_id, "v2", tr, collect_by_date }
        saveAuthParamsInSecondFactor(transaction_request.txnDetail.id, auth_params)

        // 4. Make external API call
        gateway_response = callGatewayApi(getMandateIntentEndpoint(gateway_name), encrypted_payload)

        // 5. Process gateway response
        decrypted_response = decrypt(gateway_response)
        parsed_response = parseIciciIntentResponse(decrypted_response)

        if parsed_response.isSuccess:
            deep_link = parsed_response.signedQRData
            sdk_params = makeMandateIntentSdkParams(deep_link)
            updateTransactionStatus(transaction_request.txnDetail, PENDING_AUTHORIZATION)
            return SUCCESS(sdk_params)
        else:
            error_pgr = makeErrorPGR(parsed_response.errorCode, parsed_response.errorMessage)
            updateTransactionStatus(transaction_request.txnDetail, AUTHENTICATION_FAILED)
            return FAILURE(error_pgr)

    function handleDirectUpiPayment(transaction_request, mga, gateway_details):
        // 1. Determine payment source structure
        payment_method = transaction_request.txnCardInfo.paymentMethod
        if gateway_name in gatewaysRequiringJsonUpiPaymentSource:
            payment_source_obj = UpiPaymentSource(upiIdentifier="UPI_PAY", upi_app=transaction_request.upi_app, payer_vpa=transaction_request.upi_vpa)
            transaction_request.txnCardInfo.paymentSource = encodeToJson(payment_source_obj)
        else:
            transaction_request.txnCardInfo.paymentSource = transaction_request.upi_vpa or transaction_request.upi_app

        // 2. Check for S2S disabled (e.g., Razorpay)
        if gateway_name == "RAZORPAY" and isFeatureEnabled("RAZORPAY_S2S_DISABLED_MERCHANTS", merchant_id):
            // Use AJAX routing
            request_payload = makeRazorpayAjaxCollectRequest(...)
            gateway_response = callGatewayAjaxApi(request_payload)
            // Parse AjaxWebCollectResponse
        else:
            // Use standard S2S routing
            request_payload = makeGatewayS2SRequest(...)
            gateway_response = callGatewayS2SApi(request_payload)

        // 3. Process gateway response and generate SdkParams (deep link)
        sdk_params = generateSdkParamsFromGatewayResponse(gateway_response)
        updateTransactionStatus(...)
        return SUCCESS(sdk_params) or FAILURE(...)

    function interpretTransactionStatus(gateway_response_data, transaction_detail, mandate_obj):
        status = DEFAULT_PENDING

        if transaction_detail.txnObjectType == EMANDATE_REGISTER and mandate_obj.frequency == ONETIME:
            // Special handling for one-time mandate states
            if gateway_response_data.status == "CREATE-INITIATED":
                status = AUTHORIZED // Not actually pending for OTM
            else if gateway_response_data.status == "SUCCESS":
                status = SUCCESS
            // ... more OTM specific logic
        else:
            // General transaction status interpretation
            if gateway_response_data.isSuccessCode:
                status = SUCCESS
            else if gateway_response_data.isFailureCode:
                status = FAILED
            // ...

        return status
    ```

*   **Mandatory Validations:**
    *   Presence of `mandate.startDate`, `mandate.endDate`, `mandate.maxAmount` for mandate registrations.
    *   Gateway-specific API key and credentials (e.g., `iciciApikey` not `isStringAbsent`).
    *   `amount > 0`.
    *   `mandate.isActive` (for mandate-related transactions).
    *   URL/Deep Link format validity when parsing and generating `SdkParams`.
    *   One-time mandate parameters (purpose, block, revokable flags) in `SdkParams` must match expected values (`validateSdkParamsForOTM`).

## 7. Code-Aware Guidelines

*   **Naming Conventions:**
    *   Gateway-specific functions for mandates or specific payment flows should be prefixed with the gateway name (e.g., `iciciIntentMandateTransaction`, `EaseBuzz.callMandateFirstExecution`).
    *   Suffix functions with `ForOTM` when they contain logic specific to one-time mandates (e.g., `updatedSdkParamWithOTMCheck`).
    *   Transaction reference IDs for mandate registration should be prefixed with `mdtreg` (e.g., `mdtreg<txn_id>`).
*   **Side-Effect Constraints:**
    *   Never modify `TxnDetail` directly without using helper functions like `updateTxnDetailErrorMessage` or `Txn.updateTxnDetailWithoutOpts` to ensure consistency and trigger necessary side-effects (e.g., logging, event emission).
    *   Always use `saveAuthParamsInSf` to persist gateway-specific authentication parameters in `SecondFactor`.
*   **Error Handling Patterns:**
    *   Gateway communication errors should be caught and mapped to `ErrorPaymentResp` with appropriate `pgInfo` and internal `status` (e.g., `AUTHENTICATION_FAILED`, `JUSPAY_DECLINED`).
    *   For non-critical decoding errors or unexpected gateway responses, `LogUtils.forkErrorLog` should be used, but generally, the transaction should be marked as `AUTHENTICATION_FAILED` or `PENDING_VBV` (if a client-side action is expected).
    *   Map gateway 5xx errors or specific timeout responses to `PENDING` states where retries are possible.
    *   When an error occurs during `SdkParams` generation (e.g., invalid deep link parsing), throw a custom exception with a user-friendly message (`defaultThrowECException`).
*   **Data Structure Usage:**
    *   When dealing with UPI payment sources, prefer using the structured `UpiPaymentSource` data type and encoding it to JSON for storage in `txnCardInfo.paymentSource`, especially for gateways listed in `Constants.paymentSourceAsJsonForUPI`.
    *   Always extract VPA from `UpiPaymentSource` using dedicated helper functions like `fetchVpafromPaymentSource`.
    *   Pass `maybeCustomer` and `maybeMandate` parameters to `getSdkParams` functions if the context allows, as more UPI gateways are starting to require them.

## 8. Anti-Patterns

*   **Hardcoding Gateway Logic:** Do not hardcode gateway-specific logic (e.g., status codes, request formats) directly within generic functions. Instead, abstract it behind gateway-specific modules (e.g., `IciciUpi.isPendingStatus`).
*   **Assuming Plain String VPA:** Do not assume `txnCardInfo.paymentSource` will always be a plain VPA string. It can be a JSON-encoded `UpiPaymentSource` object for certain gateways. Always use `fetchVpafromPaymentSource` or check the format before direct usage.
*   **Ignoring Feature Flags:** Do not bypass feature flag checks (e.g., `isS2SDisabledMerchant`). These flags are critical for merchant-specific configurations and routing.
*   **Inconsistent Mandate Status Interpretation:** Forgetting to account for specific mandate types (e.g., one-time mandates) in status interpretation functions like `isPendingStatus`, which can lead to incorrect state transitions.
*   **Missing `SecondFactor` Updates:** Failing to save gateway-specific authentication parameters (e.g., `IciciUpiAuthParams`) into `SecondFactor` during the mandate registration/first debit flow. This leads to issues in subsequent operations.
*   **Direct Mutation of Core Objects:** Modifying core `TxnDetail` or `Mandate` fields without using established update functions can lead to data inconsistencies and bypass business logic.

## 9. File/Function Impact Map

*   **Primary Files:**
    *   `src/PaymentGateways/<GatewayName>/<GatewayName>Upi.hs` (e.g., `IciciUpi.hs`, `EaseBuzz.hs`, `Razorpay.hs`): Gateway-specific implementation details, request/response structures, and API calls.
    *   `src/UpiController.hs`: Central dispatch for UPI-related flows, often containing the main `getSdkParams` for various UPI gateways.
    *   `src/Transaction/TxnDetail.hs`: Core transaction object definitions and related helper functions.
    *   `src/Mandate/Mandate.hs`: Core mandate object definitions and lifecycle functions.
    *   `src/PaymentMethod/TxnCardInfo.hs`: Defines the `TxnCardInfo` and `UpiPaymentSource` data types.
    *   `src/SecondFactor/SecondFactor.hs`: Handles storage and retrieval of authentication parameters.
    *   `src/CommonGatewayService/CommonGatewayService.hs`: General utilities for gateway interaction and error handling.
*   **Upstream Callers:**
    *   API endpoints handling payment creation (`/payments/create`, `/mandates/register`).
    *   Webhook listeners (`/webhook/<gateway>`).
    *   Internal services requesting transaction status or initiating refunds.
*   **Downstream Dependencies:**
    *   Payment Gateway APIs (e.g., ICICI UPI API, Razorpay API, EaseBuzz API).
    *   Database (for `TxnDetail`, `Mandate`, `MerchantGatewayAccount`, `SecondFactor`).
    *   Logging and Monitoring systems.
    *   Analytics platforms (consuming VPA details, transaction outcomes).
    *   Notification services (for webhooks).
	*   Client-side SDKs (consuming `SdkParams` for deep linking).
