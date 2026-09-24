use common_types::primitive_wrappers::AcceptAmountMismatchBool;
use common_utils::{errors::IntegrityCheckError, types::MinorUnit};
use hyperswitch_domain_models::router_request_types::{
    AuthoriseIntegrityObject, CaptureIntegrityObject, PaymentsAuthorizeData, PaymentsCaptureData,
    PaymentsSyncData, RefundIntegrityObject, RefundsData, SyncIntegrityObject,
};

/// Whether the connector may report a lower amount than requested (e.g. partial authorization).
#[derive(Debug, Clone, Copy)]
pub struct AllowLowerAmount(bool);
impl AllowLowerAmount {
    /// Creates a new instance of `AllowLowerAmount`
    pub fn new(value: bool) -> Self {
        Self(value)
    }
}
impl std::ops::Deref for AllowLowerAmount {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Whether the connector may report a higher amount than requested (e.g. overcapture).
#[derive(Debug, Clone, Copy)]
pub struct AllowHigherAmount(bool);
impl AllowHigherAmount {
    /// Creates a new instance of `AllowHigherAmount`
    pub fn new(value: bool) -> Self {
        Self(value)
    }
}
impl std::ops::Deref for AllowHigherAmount {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Governs whether a connector-reported amount that differs from the amount we requested
/// should still be treated as an integrity failure. Some payment features intentionally
/// authorize/capture a different amount than requested (e.g. partial authorization,
/// overcapture), so those differences must not be conflated with genuine data-integrity
/// mismatches.
#[derive(Debug, Clone, Copy)]
pub struct AmountMismatchTolerance {
    /// The connector may report a lower amount than requested (e.g. partial authorization).
    pub allow_lower: AllowLowerAmount,
    /// The connector may report a higher amount than requested (e.g. overcapture).
    pub allow_higher: AllowHigherAmount,
}

impl AmountMismatchTolerance {
    /// Tolerance driven only by the merchant's `accept_amount_mismatch` config: any difference
    /// is permitted when it is enabled, otherwise the connector-reported amount must exactly
    /// match what was requested.
    fn from_accept_amount_mismatch(accept_amount_mismatch: AcceptAmountMismatchBool) -> Self {
        Self {
            allow_lower: AllowLowerAmount::new(*accept_amount_mismatch),
            allow_higher: AllowHigherAmount::new(*accept_amount_mismatch),
        }
    }

    fn permits(self, expected: MinorUnit, actual: MinorUnit) -> bool {
        actual == expected
            || (*self.allow_lower && actual < expected)
            || (*self.allow_higher && actual > expected)
    }
}

/// Connector Integrity trait to check connector data integrity
pub trait FlowIntegrity {
    /// Output type for the connector
    type IntegrityObject;
    /// helps in connector integrity check
    fn compare(
        req_integrity_object: Self::IntegrityObject,
        res_integrity_object: Self::IntegrityObject,
        connector_transaction_id: Option<String>,
        amount_tolerance: AmountMismatchTolerance,
    ) -> Result<(), IntegrityCheckError>;
}

/// Trait to get connector integrity object based on request and response
pub trait GetIntegrityObject<T: FlowIntegrity> {
    /// function to get response integrity object
    fn get_response_integrity_object(&self) -> Option<T::IntegrityObject>;
    /// function to get request integrity object
    fn get_request_integrity_object(&self) -> T::IntegrityObject;
}

/// Trait to check flow type, based on which various integrity checks will be performed
pub trait CheckIntegrity<Request, T> {
    /// Function to check to initiate integrity check
    fn check_integrity(
        &self,
        request: &Request,
        connector_transaction_id: Option<String>,
        accept_amount_mismatch: AcceptAmountMismatchBool,
    ) -> Result<(), IntegrityCheckError>;
}

impl<T, Request> CheckIntegrity<Request, T> for RefundsData
where
    T: FlowIntegrity,
    Request: GetIntegrityObject<T>,
{
    fn check_integrity(
        &self,
        request: &Request,
        connector_refund_id: Option<String>,
        accept_amount_mismatch: AcceptAmountMismatchBool,
    ) -> Result<(), IntegrityCheckError> {
        match request.get_response_integrity_object() {
            Some(res_integrity_object) => {
                let req_integrity_object = request.get_request_integrity_object();
                T::compare(
                    req_integrity_object,
                    res_integrity_object,
                    connector_refund_id,
                    AmountMismatchTolerance::from_accept_amount_mismatch(accept_amount_mismatch),
                )
            }
            None => Ok(()),
        }
    }
}

impl<T, Request> CheckIntegrity<Request, T> for PaymentsAuthorizeData
where
    T: FlowIntegrity,
    Request: GetIntegrityObject<T>,
{
    fn check_integrity(
        &self,
        request: &Request,
        connector_transaction_id: Option<String>,
        accept_amount_mismatch: AcceptAmountMismatchBool,
    ) -> Result<(), IntegrityCheckError> {
        match request.get_response_integrity_object() {
            Some(res_integrity_object) => {
                let req_integrity_object = request.get_request_integrity_object();
                // Partial authorization: the connector may legitimately authorize less than
                // requested. Authorizing more is only accepted when the merchant has opted into
                // `accept_amount_mismatch`, which permits a difference in either direction.
                let amount_tolerance = AmountMismatchTolerance {
                    allow_lower: AllowLowerAmount::new(
                        *accept_amount_mismatch
                            || self
                                .enable_partial_authorization
                                .is_some_and(|enabled| enabled.is_true()),
                    ),
                    allow_higher: AllowHigherAmount::new(*accept_amount_mismatch),
                };
                T::compare(
                    req_integrity_object,
                    res_integrity_object,
                    connector_transaction_id,
                    amount_tolerance,
                )
            }
            None => Ok(()),
        }
    }
}

impl<T, Request> CheckIntegrity<Request, T> for PaymentsCaptureData
where
    T: FlowIntegrity,
    Request: GetIntegrityObject<T>,
{
    fn check_integrity(
        &self,
        request: &Request,
        connector_transaction_id: Option<String>,
        accept_amount_mismatch: AcceptAmountMismatchBool,
    ) -> Result<(), IntegrityCheckError> {
        match request.get_response_integrity_object() {
            Some(res_integrity_object) => {
                let req_integrity_object = request.get_request_integrity_object();
                // Overcapture: the merchant may be allowed to capture more than the originally
                // requested amount. Capturing less is only accepted when the merchant has opted
                // into `accept_amount_mismatch`, which permits a difference in either direction.
                let amount_tolerance = AmountMismatchTolerance {
                    allow_lower: AllowLowerAmount::new(*accept_amount_mismatch),
                    allow_higher: AllowHigherAmount::new(
                        *accept_amount_mismatch
                            || self.is_overcapture_enabled.is_some_and(|enabled| *enabled),
                    ),
                };
                T::compare(
                    req_integrity_object,
                    res_integrity_object,
                    connector_transaction_id,
                    amount_tolerance,
                )
            }
            None => Ok(()),
        }
    }
}

impl<T, Request> CheckIntegrity<Request, T> for PaymentsSyncData
where
    T: FlowIntegrity,
    Request: GetIntegrityObject<T>,
{
    fn check_integrity(
        &self,
        request: &Request,
        connector_transaction_id: Option<String>,
        accept_amount_mismatch: AcceptAmountMismatchBool,
    ) -> Result<(), IntegrityCheckError> {
        match request.get_response_integrity_object() {
            Some(res_integrity_object) => {
                let req_integrity_object = request.get_request_integrity_object();
                // A sync may report a partially authorized (lower) or overcaptured (higher)
                // amount, so both feature tolerances apply here, in addition to the merchant's
                // `accept_amount_mismatch` opt-in which permits a difference in either direction.
                let amount_tolerance = AmountMismatchTolerance {
                    allow_lower: AllowLowerAmount::new(
                        *accept_amount_mismatch
                            || self
                                .enable_partial_authorization
                                .is_some_and(|enabled| enabled.is_true()),
                    ),
                    allow_higher: AllowHigherAmount::new(
                        *accept_amount_mismatch
                            || self.is_overcapture_enabled.is_some_and(|enabled| *enabled),
                    ),
                };
                T::compare(
                    req_integrity_object,
                    res_integrity_object,
                    connector_transaction_id,
                    amount_tolerance,
                )
            }
            None => Ok(()),
        }
    }
}

impl FlowIntegrity for RefundIntegrityObject {
    type IntegrityObject = Self;
    fn compare(
        req_integrity_object: Self,
        res_integrity_object: Self,
        connector_transaction_id: Option<String>,
        amount_tolerance: AmountMismatchTolerance,
    ) -> Result<(), IntegrityCheckError> {
        let mut mismatched_fields = Vec::new();

        if req_integrity_object.currency != res_integrity_object.currency {
            mismatched_fields.push(format_mismatch(
                "currency",
                &req_integrity_object.currency.to_string(),
                &res_integrity_object.currency.to_string(),
            ));
        }

        if !amount_tolerance.permits(
            req_integrity_object.refund_amount,
            res_integrity_object.refund_amount,
        ) {
            mismatched_fields.push(format_mismatch(
                "refund_amount",
                &req_integrity_object.refund_amount.to_string(),
                &res_integrity_object.refund_amount.to_string(),
            ));
        }

        if mismatched_fields.is_empty() {
            Ok(())
        } else {
            let field_names = mismatched_fields.join(", ");

            Err(IntegrityCheckError {
                field_names,
                connector_transaction_id,
                // TODO: Currently the refund amount returned by the connector is
                // not captured in the refund data. Hence validate_refund_amount keeps summing the originally
                // requested amount for ManualReview/ VoidPostRefund flows instead of what was actually refunded.
                amount: Some(res_integrity_object.refund_amount),
            })
        }
    }
}

impl FlowIntegrity for AuthoriseIntegrityObject {
    type IntegrityObject = Self;
    fn compare(
        req_integrity_object: Self,
        res_integrity_object: Self,
        connector_transaction_id: Option<String>,
        amount_tolerance: AmountMismatchTolerance,
    ) -> Result<(), IntegrityCheckError> {
        let mut mismatched_fields = Vec::new();

        if !amount_tolerance.permits(req_integrity_object.amount, res_integrity_object.amount) {
            mismatched_fields.push(format_mismatch(
                "amount",
                &req_integrity_object.amount.to_string(),
                &res_integrity_object.amount.to_string(),
            ));
        }

        if req_integrity_object.currency != res_integrity_object.currency {
            mismatched_fields.push(format_mismatch(
                "currency",
                &req_integrity_object.currency.to_string(),
                &res_integrity_object.currency.to_string(),
            ));
        }

        if mismatched_fields.is_empty() {
            Ok(())
        } else {
            let field_names = mismatched_fields.join(", ");

            Err(IntegrityCheckError {
                field_names,
                connector_transaction_id,
                // TODO: Handle amount when there is a currency mismatch.
                // We store the amount actually received/reported by the connector even in case of a currency mismatch,
                // so that further operations on the payment are not blocked.
                // If the connector ignored the currency provided in the payment request, currently we assume the same behavior
                // from the connector for subsequent operations.
                amount: Some(res_integrity_object.amount),
            })
        }
    }
}

impl FlowIntegrity for SyncIntegrityObject {
    type IntegrityObject = Self;
    fn compare(
        req_integrity_object: Self,
        res_integrity_object: Self,
        connector_transaction_id: Option<String>,
        amount_tolerance: AmountMismatchTolerance,
    ) -> Result<(), IntegrityCheckError> {
        let mut mismatched_fields = Vec::new();

        res_integrity_object
            .amount
            .zip(req_integrity_object.amount)
            .map(|(res_amount, req_amount)| {
                if !amount_tolerance.permits(req_amount, res_amount) {
                    mismatched_fields.push(format_mismatch(
                        "amount",
                        &req_amount.to_string(),
                        &res_amount.to_string(),
                    ));
                }
            });

        res_integrity_object
            .currency
            .zip(req_integrity_object.currency)
            .map(|(res_currency, req_currency)| {
                if res_currency != req_currency {
                    mismatched_fields.push(format_mismatch(
                        "currency",
                        &req_currency.to_string(),
                        &res_currency.to_string(),
                    ));
                }
            });

        if mismatched_fields.is_empty() {
            Ok(())
        } else {
            let field_names = mismatched_fields.join(", ");

            Err(IntegrityCheckError {
                field_names,
                connector_transaction_id,
                amount: res_integrity_object.amount,
            })
        }
    }
}

impl FlowIntegrity for CaptureIntegrityObject {
    type IntegrityObject = Self;
    fn compare(
        req_integrity_object: Self,
        res_integrity_object: Self,
        connector_transaction_id: Option<String>,
        amount_tolerance: AmountMismatchTolerance,
    ) -> Result<(), IntegrityCheckError> {
        let mut mismatched_fields = Vec::new();

        res_integrity_object
            .capture_amount
            .zip(req_integrity_object.capture_amount)
            .map(|(res_amount, req_amount)| {
                if !amount_tolerance.permits(req_amount, res_amount) {
                    mismatched_fields.push(format_mismatch(
                        "capture_amount",
                        &req_amount.to_string(),
                        &res_amount.to_string(),
                    ));
                }
            });

        if req_integrity_object.currency != res_integrity_object.currency {
            mismatched_fields.push(format_mismatch(
                "currency",
                &req_integrity_object.currency.to_string(),
                &res_integrity_object.currency.to_string(),
            ));
        }

        if mismatched_fields.is_empty() {
            Ok(())
        } else {
            let field_names = mismatched_fields.join(", ");

            Err(IntegrityCheckError {
                field_names,
                connector_transaction_id,
                amount: res_integrity_object.capture_amount,
            })
        }
    }
}

impl GetIntegrityObject<CaptureIntegrityObject> for PaymentsCaptureData {
    fn get_response_integrity_object(&self) -> Option<CaptureIntegrityObject> {
        self.integrity_object.clone()
    }

    fn get_request_integrity_object(&self) -> CaptureIntegrityObject {
        CaptureIntegrityObject {
            capture_amount: Some(self.minor_amount_to_capture),
            currency: self.currency,
        }
    }
}

impl GetIntegrityObject<RefundIntegrityObject> for RefundsData {
    fn get_response_integrity_object(&self) -> Option<RefundIntegrityObject> {
        self.integrity_object.clone()
    }

    fn get_request_integrity_object(&self) -> RefundIntegrityObject {
        RefundIntegrityObject {
            currency: self.currency,
            refund_amount: self.minor_refund_amount,
        }
    }
}

impl GetIntegrityObject<AuthoriseIntegrityObject> for PaymentsAuthorizeData {
    fn get_response_integrity_object(&self) -> Option<AuthoriseIntegrityObject> {
        self.integrity_object.clone()
    }

    fn get_request_integrity_object(&self) -> AuthoriseIntegrityObject {
        AuthoriseIntegrityObject {
            amount: self.minor_amount,
            currency: self.currency,
        }
    }
}

impl GetIntegrityObject<SyncIntegrityObject> for PaymentsSyncData {
    fn get_response_integrity_object(&self) -> Option<SyncIntegrityObject> {
        self.integrity_object.clone()
    }

    fn get_request_integrity_object(&self) -> SyncIntegrityObject {
        SyncIntegrityObject {
            amount: Some(self.amount),
            currency: Some(self.currency),
        }
    }
}

#[inline]
fn format_mismatch(field: &str, expected: &str, found: &str) -> String {
    format!("{field} expected {expected} but found {found}")
}
