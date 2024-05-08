// ResponseHandler will contain all the response handling functions that can be used across the tests
import { globalStateSetter } from "./commands";

// pre-defined granular constant expected values
const validateContentType = function (response) { expect(response.headers["content-type"]).to.include("application/json"); }
const validateExistenceOfMerchantId = function (globalState, merchantId) { expect(merchantId).to.equal(globalState.get("merchant_id")).and.not.empty; }
const validateCustomerId = function (globalState, customerId) { expect(customerId).to.equal(globalState.get("customerId")).and.not.empty; }
const validateConnectorName = function (globalState, connectorName) { expect(connectorName).to.equal(globalState.get("connectorId")).and.not.empty; }
const validatePaymentId = function (globalState, response) { expect(response.body).to.have.property("payment_id").equal(globalState.get("payment_id")); }
const valdiateAmount = function (amount, response) { expect(amount).to.equal(response.body.amount).to.equal(response.body.amount_capturable); }
const validateAmountToCapture = function (request, response) { expect(response.body.amount).to.equal(request.amount_to_capture).to.equal(request.amount); }
const validateExistenceOfClientSecret = function (response) { expect(response.body).to.have.property("client_secret"); }
const validateExistenceOfRedirectUrl = function (response) { expect(response.body).to.have.property("next_action").to.have.property("redirect_to_url"); }
const valdiateExistenceOfPaymentMethods = function (response) { expect(response.body).to.have.property("payment_methods"); };
const validatePaymentToken = function (globalState, paymentToken) { expect(globalState.get("paymentToken")).to.equal(paymentToken); }
const validatePaymentStatus = function (expectedStatus, response) {
    expect(response.body.status).to.equal(expectedStatus);
}
const validateCapturableAmount = function (request, response) {
    if (response.body.status === "succeeded") {
        expect(response.body.amount_capturable).to.equal(0);
    } else {
        expect(response.body.amount_capturable).to.equal(request.amount);
    }
}
const validateReceivedAmount = function (request, response) {
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
const validateResponseStatus = function (status, response) {
    switch (status) {
        case "requires_capture":
            expect("requires_capture").to.equal(response.body.status);
            break;
        case "succeeded":
            expect("succeeded").to.equal(response.body.status);
            break;
        case "processing":
            expect("processing").to.equal(response.body.status);
            break;
        case "partially_captured":
            expect("partially_captured").to.equal(response.body.status);
            break;
        case "requires_payment_method":
            expect("requires_payment_method").to.equal(response.body.status);
            break;
        case "requires_confirmation":
            expect("requires_confirmation").to.equal(response.body.status);
            break;
        case "cancelled":
            expect("cancelled").to.equal(response.body.status);
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
                validatePaymentStatus(details.paymentSuccessfulStatus, response);
                validateCustomerId(globalState, response.body.customer_id);
            } else if (response.body.capture_method === "manual") {
                validatePaymentStatus("requires_capture", response);
                validateCustomerId(globalState, response.body.customer_id);
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
    valdiateAmount,
    validateAmountToCapture,
    validateCapturableAmount,
    validateCaptureMethod,
    validateConnectorName,
    validateContentType,
    validateCustomerId,
    validateExistenceOfClientSecret,
    validateExistenceOfMerchantId,
    valdiateExistenceOfPaymentMethods,
    validateExistenceOfRedirectUrl,
    validatePaymentId,
    validatePaymentStatus,
    validatePaymentToken,
    validateReceivedAmount,
    validateResponseStatus,
};
