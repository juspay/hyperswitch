use common_utils::errors::IntegrityCheckError;
use hyperswitch_domain_models::router_request_types::{
    AuthoriseIntegrityObject, PaymentsAuthorizeData, PaymentsSyncData, SyncIntegrityObject,
};

/// Connector Integrity trait to check connector data integrity
pub trait FlowIntegrity {
    /// Output type for the connector
    type IntegrityObject;
    /// helps in connector integrity check
    fn compare(
        req_integrity_object: Self::IntegrityObject,
        res_integrity_object: Self::IntegrityObject,
        connector_transaction_id: Option<String>,
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
    ) -> Result<(), IntegrityCheckError>;
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
    ) -> Result<(), IntegrityCheckError> {
        match request.get_response_integrity_object() {
            Some(res_integrity_object) => {
                let req_integrity_object = request.get_request_integrity_object();
                T::compare(
                    req_integrity_object,
                    res_integrity_object,
                    connector_transaction_id,
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
    ) -> Result<(), IntegrityCheckError> {
        match request.get_response_integrity_object() {
            Some(res_integrity_object) => {
                let req_integrity_object = request.get_request_integrity_object();
                T::compare(
                    req_integrity_object,
                    res_integrity_object,
                    connector_transaction_id,
                )
            }
            None => Ok(()),
        }
    }
}

impl FlowIntegrity for AuthoriseIntegrityObject {
    type IntegrityObject = Self;
    fn compare(
        req_integrity_object: Self,
        res_integrity_object: Self,
        connector_transaction_id: Option<String>,
    ) -> Result<(), IntegrityCheckError> {
        let mut mismatched_fields = Vec::new();

        if req_integrity_object.amount != res_integrity_object.amount {
            mismatched_fields.push("amount".to_string());
        }

        if req_integrity_object.currency != res_integrity_object.currency {
            mismatched_fields.push("currency".to_string());
        }

        if mismatched_fields.is_empty() {
            Ok(())
        } else {
            let field_names = mismatched_fields.join(", ");

            Err(IntegrityCheckError {
                field_names,
                connector_transaction_id,
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
    ) -> Result<(), IntegrityCheckError> {
        let mut mismatched_fields = Vec::new();

        if req_integrity_object.amount != res_integrity_object.amount {
            mismatched_fields.push("amount".to_string());
        }

        if req_integrity_object.currency != res_integrity_object.currency {
            mismatched_fields.push("currency".to_string());
        }

        if mismatched_fields.is_empty() {
            Ok(())
        } else {
            let field_names = mismatched_fields.join(", ");

            Err(IntegrityCheckError {
                field_names,
                connector_transaction_id,
            })
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
