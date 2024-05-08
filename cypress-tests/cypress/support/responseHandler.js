// ResponseHandler will contain all the response handling functions that can be used across the tests

// pre-defined granular constant expected values
const validateContentType = function (response) { expect(response.headers["content-type"]).to.include("application/json"); }
const validateExistenceOfMerchantId = function (merchantId) { expect(merchantId).to.equal(globalState.get("merchant_id")).and.not.empty; }
const validateCustomerId = function (customerId) { expect(customerId).to.equal(globalState.get("customerId")).and.not.empty; }
const validateConnectorName = function (connectorName) { expect(connectorName).to.equal(globalState.get("connectorId")).and.not.empty; }
const validatePaymentId = function (body) { expect(body).to.have.property("payment_id").equal(globalState.get("payment_id")); }
const valdiateAmount = function (amount) { expect(amount).to.equal(response.body.amount).to.equal(response.body.amount_capturable); }
const validateAmountToCapture = function (request, response) { expect(response.body.amount).to.equal(request.amount_to_capture).to.equal(request.amount); }
const validateExistenceOfClientSecret = function (body) { expect(body).to.have.property("client_secret"); }
const validateExistenceOfRedirectUrl = function (body) { expect(body).to.have.property("next_action").to.have.property("redirect_url"); }
const valdiateExistenceOfPaymentMethods = function (body) { expect(body).to.have.property("payment_methods"); };
const validatePaymentToken = function (paymentToken) { expect(globalState.get("paymentToken")).to.equal(paymentToken); }
const validateReceivedAmount = function (amount, body) { expect(amount).to.equal(body.amount_received); };
const validateCapturableAmount = function (request, response) {
    if (response.body.status === "succeeded") {
        expect(response.body.amount_capturable).to.equal(0);
    } else {
        expect(response.body.amount_capturable).to.equal(request.amount);
    }
}
const validateAmountReceived = function (request, response) {
    switch (response.body.status) {
        case "succeeded":
            expect(response.body.amount_received).to.equal(request.amount);
            break;
        case "processing":
            expect(response.body.amount_received).to.equal(0);
            break;
        case "partially_captured":
            expect(response.body.amount_received).to.equal(request.amount_to_capture);
            break;
        default:
            throw new Error(`Unknown status: ${response.body.status}`);
    }
}
const validateCaptureMethod = function (capture_method) {
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
const validateResponseStatus = function (status) {
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
                expect(response.body).to.have.property("next_action")
                    .to.have.property("redirect_to_url");

                if (setNextActionUrl) {
                    let nextActionUrl = response.body.next_action.redirect_to_url;
                    globalState.set("nextActionUrl", nextActionUrl);
                    cy.log(response.body);
                    cy.log(nextActionUrl);
                }
            } else {
                throw new Error(`Unsupported capture method: ${capture_method}`);
            }
            break;
        case "no_three_ds":
            if (response.body.capture_method === "automatic") {
                expect(details.paymentSuccessfulStatus).to.equal(response.body.status);
                expect(response.body.customer_id).to.equal(globalState.get("customerId"));
            } else if (response.body.capture_method === "manual") {
                expect("requires_capture").to.equal(response.body.status);
                expect(response.body.customer_id).to.equal(globalState.get("customerId"));
            } else {
                throw new Error(`Unsupported capture method: ${capture_method}`);
            }
            break;
        default:
            throw new Error(`Unsupported authentication type: ${authentication_type}`);
    }
}

export function responseHandler() {

}

module.exports = {
    validateContentType,
    validateExistenceOfMerchantId,
    validateCustomerId,
    validateConnectorName,
    validatePaymentId,
    valdiateAmount,
    validateCapturableAmount,
    validateAmountToCapture,
    validateAmountReceived,
    validateExistenceOfClientSecret,
    validateExistenceOfRedirectUrl,
    valdiateExistenceOfPaymentMethods,
    validatePaymentToken,
    validateReceivedAmount,
    validateCaptureMethod,
    validateResponseStatus,
    logRequestId,
    handleAuthType
};
