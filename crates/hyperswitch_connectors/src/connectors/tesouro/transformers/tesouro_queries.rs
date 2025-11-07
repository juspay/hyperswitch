pub const AUTHORIZE_TRANSACTION: &str = "mutation AuthorizeCustomerInitiatedTransaction(
    $authorizeCustomerInitiatedTransactionInput: AuthorizeCustomerInitiatedTransactionInput!
) {
    authorizeCustomerInitiatedTransaction(
        authorizeCustomerInitiatedTransactionInput: $authorizeCustomerInitiatedTransactionInput
    ) {
        authorizationResponse {
            paymentId
            transactionId
            tokenDetails {
                token
            }
            activityDate
            __typename
            ... on AuthorizationApproval {
                __typename
                paymentId
                transactionId
                tokenDetails {
                    token
                }
                activityDate
            }
            ... on AuthorizationDecline {
                __typename
                transactionId
                paymentId
                message
                tokenDetails {
                    token
                }
            }
        }
        errors {
            ... on InternalServiceError {
                message
                transactionId
                processorResponseCode
            }
            ... on AcceptorNotFoundError {
                message
                transactionId
                processorResponseCode
            }
            ... on RuleInViolationError {
                message
                transactionId
                processorResponseCode
            }
            ... on SyntaxOnNetworkResponseError {
                message
                transactionId
                processorResponseCode
            }
            ... on TimeoutOnNetworkResponseError {
                message
                transactionId
                processorResponseCode
            }
            ... on ValidationFailureError {
                message
                processorResponseCode
                transactionId
            }
            ... on UnknownCardError {
                message
                processorResponseCode
                transactionId
            }
            ... on TokenNotFoundError {
                message
                processorResponseCode
                transactionId
            }
            ... on InvalidTokenError {
                message
                processorResponseCode
                transactionId
            }
            ... on RouteNotFoundError {
                message
                processorResponseCode
                transactionId
            }
        }
    }
}";

pub const AUTHORIZE_RECURRING: &str = "mutation AuthorizeRecurring(
    $authorizeRecurringInput: AuthorizeRecurringInput!
) {
    authorizeRecurring(
        authorizeRecurringInput: $authorizeRecurringInput
    ) {
        authorizationResponse {
            paymentId
            transactionId
            tokenDetails {
                token
            }
            activityDate
            __typename
            ... on AuthorizationApproval {
                __typename
                paymentId
                transactionId
                tokenDetails {
                    token
                }
                activityDate
            }
            ... on AuthorizationDecline {
                __typename
                transactionId
                paymentId
                message
            }
        }
        errors {
            ... on InternalServiceError {
                message
                transactionId
                processorResponseCode
            }
            ... on AcceptorNotFoundError {
                message
                transactionId
                processorResponseCode
            }
            ... on RuleInViolationError {
                message
                transactionId
                processorResponseCode
            }
            ... on SyntaxOnNetworkResponseError {
                message
                transactionId
                processorResponseCode
            }
            ... on TimeoutOnNetworkResponseError {
                message
                transactionId
                processorResponseCode
            }
            ... on ValidationFailureError {
                message
                processorResponseCode
                transactionId
            }
            ... on UnknownCardError {
                message
                processorResponseCode
                transactionId
            }
            ... on TokenNotFoundError {
                message
                processorResponseCode
                transactionId
            }
            ... on InvalidTokenError {
                message
                processorResponseCode
                transactionId
            }
            ... on RouteNotFoundError {
                message
                processorResponseCode
                transactionId
            }
            ... on PriorPaymentNotFoundError {
                message
                processorResponseCode
                transactionId
            }
        }
    }
}";

pub const SETUP_MANDATE: &str = "mutation VerifyAccount(
    $verifyAccountInput: VerifyAccountInput!
) {
    verifyAccount(
        verifyAccountInput: $verifyAccountInput
    ) {
        verifyAccountResponse {
            paymentId
            transactionId
            tokenDetails {
                token
            }
            activityDate
        }
        errors {
            ... on InternalServiceError {
                message
                transactionId
                processorResponseCode
            }
            ... on AcceptorNotFoundError {
                message
                transactionId
                processorResponseCode
            }
            ... on RuleInViolationError {
                message
                transactionId
                processorResponseCode
            }
            ... on SyntaxOnNetworkResponseError {
                message
                transactionId
                processorResponseCode
            }
            ... on TimeoutOnNetworkResponseError {
                message
                transactionId
                processorResponseCode
            }
            ... on ValidationFailureError {
                message
                processorResponseCode
                transactionId
            }
            ... on UnknownCardError {
                message
                processorResponseCode
                transactionId
            }
            ... on TokenNotFoundError {
                message
                processorResponseCode
                transactionId
            }
            ... on InvalidTokenError {
                message
                processorResponseCode
                transactionId
            }
            ... on RouteNotFoundError {
                message
                processorResponseCode
                transactionId
            }
        }
    }
}";

pub const CAPTURE_TRANSACTION: &str =
    "mutation CaptureAuthorization($captureAuthorizationInput: CaptureAuthorizationInput!) {
  captureAuthorization(captureAuthorizationInput: $captureAuthorizationInput) {
    captureAuthorizationResponse {
      __typename
      ... on CaptureAuthorizationApproval {
        __typename
        paymentId
        transactionId
        tokenDetails {
                token
            }
        activityDate
      }
      ... on CaptureAuthorizationDecline {
        __typename
        paymentId
        transactionId
        message
      }
    }
    errors {
      ... on InternalServiceError {
        message
        processorResponseCode
        transactionId
      }
      ... on RuleInViolationError {
        message
        processorResponseCode
        transactionId
      }
      ... on SyntaxOnNetworkResponseError {
        message
        processorResponseCode
        transactionId
      }
      ... on TimeoutOnNetworkResponseError {
        message
        processorResponseCode
        transactionId
      }
      ... on ValidationFailureError {
        message
        processorResponseCode
        transactionId
      }
      ... on PriorPaymentNotFoundError {
        message
        processorResponseCode
        transactionId
      }
    }
  }
}";

pub const VOID_TRANSACTION: &str =
    "mutation ReverseTransaction($reverseTransactionInput: ReverseTransactionInput!) {
  reverseTransaction(reverseTransactionInput: $reverseTransactionInput) {
    errors {
      ... on InternalServiceError {
        message
        processorResponseCode
        transactionId
      }
      ... on RuleInViolationError {
        message
        processorResponseCode
        transactionId
      }
      ... on SyntaxOnNetworkResponseError {
        message
        processorResponseCode
        transactionId
      }
      ... on TimeoutOnNetworkResponseError {
        message
        processorResponseCode
        transactionId
      }
      ... on ValidationFailureError {
        message
        processorResponseCode
        transactionId
      }
      ... on PriorTransactionNotFoundError {
        message
        processorResponseCode
        transactionId
      }
    }
    reverseTransactionResponse {
      paymentId
      transactionId
      ... on ReverseTransactionApproval {
        paymentId
        transactionId
      }
      ... on ReverseTransactionDecline {
        message
        paymentId
        transactionId
        declineType
      }
    }
  }
}";

pub const REFUND_TRANSACTION: &str =
    "mutation RefundPreviousPayment($refundPreviousPaymentInput: RefundPreviousPaymentInput!) {
  refundPreviousPayment(refundPreviousPaymentInput: $refundPreviousPaymentInput) {
    errors {
      ... on InternalServiceError {
        message
        processorResponseCode
        transactionId
      }
      ... on RuleInViolationError {
        processorResponseCode
        message
        transactionId
      }
      ... on SyntaxOnNetworkResponseError {
        message
        processorResponseCode
        transactionId
      }
      ... on TimeoutOnNetworkResponseError {
        processorResponseCode
        message
        transactionId
      }
      ... on ValidationFailureError {
        message
        processorResponseCode
        transactionId
      }
      ... on PriorPaymentNotFoundError {
        message
        processorResponseCode
        transactionId
      }
    }
    refundPreviousPaymentResponse {
      __typename
      ... on RefundPreviousPaymentApproval {
        __typename
        paymentId
        transactionId
      }
      ... on RefundPreviousPaymentDecline {
        __typename
        declineType
        message
        transactionId
        paymentId
      }
    }
  }
}";

pub const SYNC_TRANSACTION: &str = "query PaymentTransaction($paymentTransactionId: UUID!) {
  paymentTransaction(id: $paymentTransactionId) {
    __typename
    responseType
    reference
    id
    paymentId
    ... on AcceptedSale {
      __typename
      id
      processorResponseCode
      processorResponseMessage
    }
    ... on ApprovedAuthorization {
      __typename
      id
      processorResponseCode
      processorResponseMessage
    }
    ... on ApprovedCapture {
      __typename
      id
      processorResponseCode
      processorResponseMessage
    }
    ... on ApprovedReversal {
      __typename
      id
      processorResponseCode
      processorResponseMessage
    }
    ... on DeclinedAuthorization {
      __typename
      id
      processorResponseCode
      processorResponseMessage
    }
    ... on DeclinedCapture {
      __typename
      id
      processorResponseCode
      processorResponseMessage
    }
    ... on DeclinedReversal {
      __typename
      id
      processorResponseCode
      processorResponseMessage
    }
    ... on GenericPaymentTransaction {
      __typename
      id
      processorResponseCode
      processorResponseMessage
    }
    ... on Authorization {
      __typename
      id
      processorResponseCode
      processorResponseMessage
    }
    ... on Capture {
      __typename
      id
      processorResponseCode
      processorResponseMessage
    }
    ... on Reversal {
      __typename
      id
      processorResponseCode
      processorResponseMessage
    }
    ... on Sale {
      __typename
      id
      processorResponseCode
      processorResponseMessage
    }
  }
}";
