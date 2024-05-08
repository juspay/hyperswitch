// ResponseHandler will contain all the response handling functions that can be used across the tests
import { globalStateSetter } from "./commands";

// pre-defined granular constant expected values
const validateContentType = function (response) { expect(response.headers["content-type"]).to.include("application/json"); }
const validateExistenceOfMerchantId = function (merchantId, responseMerchantId) { expect(responseMerchantId).to.equal(merchantId).and.not.empty; }
const validateCustomerId = function (customerId, responseCustomerId) { expect(responseCustomerId).to.equal(customerId).and.not.empty; }
const validateConnectorName = function (connectorName, connectorNameFromGlobalState) { expect(connectorName).to.equal(connectorNameFromGlobalState).and.not.empty; }
const validatePaymentId = function (response, paymentId) { expect(response.body).to.have.property("payment_id").equal(paymentId); }
const validateAmount = function (amount, response) { expect(amount).to.equal(response.body.amount); }
const validateAmountToCapture = function (amount, amount_to_capture) { expect(amount).to.equal(amount_to_capture); }
const validateExistenceOfClientSecret = function (body) { expect(body).to.have.property("client_secret"); }
const validateExistenceOfRedirectUrl = function (response) { expect(response.body).to.have.property("next_action").to.have.property("redirect_to_url"); }
const validateExistenceOfPMRedirectUrl = function (response) { expect(response.body).to.have.property("redirect_url"); }
const valdiateExistenceOfPaymentMethods = function (response) { expect(response.body).to.have.property("payment_methods"); };
const validatePaymentToken = function (token, paymentToken) { expect(token).to.equal(paymentToken); }
const validateExistenceOfStatus = function (response) { expect(response.body).to.have.property("status"); }
const validateExistenceOfMandateId = function (response) { expect(response.body).to.have.property("mandate_id"); }
const validateMandateStatus = function (status, expectedStatus) { expect(status).to.equal(expectedStatus); }
const validateMandateReason = function (reason, expectedReason) { expect(reason).to.equal(expectedReason); }
const validateArrayResponse = function (data) { expect(data).to.be.an("array").and.not.empty; }
const validatePaymentStatus = function (expectedStatus, status) {
    expect(status).to.equal(expectedStatus);
}
const validateCapturableAmount = function (request, response) {
    if (response.body.status === "succeeded") {
        expect(response.body.amount_capturable).to.equal(0);
    } else if (response.body.status === "partially_captured") {
        expect(response.body.amount_capturable).to.equal(0);
    } else {
        expect(response.body.amount_capturable).to.equal(request.amount);
    }
}
const validateReceivedAmount = function (amount, request, response) {
    switch (response.body.status) {
        case "succeeded":
            expect(amount).to.equal(response.body.amount_received);
            break;
        case "processing":
            expect(0).to.equal(response.body.amount_received);
            break;
        case "partially_captured":
            expect(request.amount_to_capture).to.equal(response.body.amount_received);
            break;
        case "requires_payment_method":
            expect(null).to.equal(response.body.amount_received);
            break;
        case "cancelled":
            expect(amount).to.be.oneOf([0, null]);
            break;
        default:
            throw new Error(`Unknown status: ${response.body.status}`);
    }
}
const validateCaptureMethod = function (capture_method, response) {
    switch (capture_method) {
        case "automatic":
            expect(response.body.capture_method).to.equal("automatic");
            break;
        case "manual":
            expect(response.body.capture_method).to.equal("manual");
            break;
        case "manual_multiple":
            expect(response.body.capture_method).to.equal("manual_multiple");
        default:
            throw new Error(`Unknown capture method: ${capture_method}`);
    }
};
const validateResponseStatus = function (status, response_status) {
    switch (status) {
        case "requires_capture":
            expect("requires_capture").to.equal(response_status);
            break;
        case "succeeded":
            expect("succeeded").to.equal(response_status);
            break;
        case "processing":
            expect("processing").to.equal(response_status);
            break;
        case "partially_captured":
            expect("partially_captured").to.equal(response_status);
            break;
        case "requires_payment_method":
            expect("requires_payment_method").to.equal(response_status);
            break;
        case "requires_confirmation":
            expect("requires_confirmation").to.equal(response_status);
            break;
        case "cancelled":
            expect("cancelled").to.equal(response_status);
            break;
        default:
            throw new Error(`Unknown status: ${status}`);
    }
}

function logRequestId(xRequestId) {
    if (xRequestId) {
        cy.task('cli_log', "x-request-id -> " + xRequestId);
    } else {
        cy.task('cli_log', "x-request-id is not available in the response headers");
    }
}

function handleAuthType(response, globalState, setNextActionUrl, details) {
    switch (response.body.authentication_type) {
        case "three_ds":
            if (response.body.capture_method === "automatic" || response.body.capture_method === "manual") {
                validateExistenceOfRedirectUrl(response);
                if (setNextActionUrl) {
                    let nextActionUrl = response.body.next_action.redirect_to_url;
                    globalStateSetter(globalState, "nextActionUrl", nextActionUrl);
                    cy.log(response.body);
                    cy.log(nextActionUrl);
                }
            } else {
                throw new Error(`Unsupported capture method: ${capture_method}`);
            }
            break;
        case "no_three_ds":
            if (response.body.capture_method === "automatic") {
                validatePaymentStatus(details.paymentSuccessfulStatus, response.body.status);
                validateCustomerId(globalState.get("customerId"), response.body.customer_id);
            } else if (response.body.capture_method === "manual") {
                validatePaymentStatus("requires_capture", response.body.status);
                validateCustomerId(globalState.get("customerId"), response.body.customer_id);
            } else {
                throw new Error(`Unsupported capture method: ${response.body.capture_method}`);
            }
            break;
        default:
            throw new Error(`Unsupported authentication type: ${response.body.authentication_type}`);
    }
}

export function responseHandler() {
    // To be implemented
}

module.exports = {
    handleAuthType,
    logRequestId,
    validateAmount,
    validateAmountToCapture,
    validateArrayResponse,
    validateCapturableAmount,
    validateCaptureMethod,
    validateConnectorName,
    validateContentType,
    validateCustomerId,
    validateExistenceOfClientSecret,
    validateExistenceOfMandateId,
    validateExistenceOfMerchantId,
    valdiateExistenceOfPaymentMethods,
    validateExistenceOfPMRedirectUrl,
    validateExistenceOfRedirectUrl,
    validateExistenceOfStatus,
    validateMandateReason,
    validateMandateStatus,
    validatePaymentId,
    validatePaymentStatus,
    validatePaymentToken,
    validateReceivedAmount,
    validateResponseStatus,
};
