//! Customer related types

use common_utils::errors::ValidationError;
use error_stack::Report;
use utoipa::ToSchema;
/// HashMap containing MerchantConnectorAccountId and corresponding customer id
#[cfg(feature = "v2")]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
#[serde(transparent)]
pub struct ConnectorCustomerMap(
    std::collections::HashMap<common_utils::id_type::MerchantConnectorAccountId, String>,
);

#[cfg(feature = "v2")]
impl ConnectorCustomerMap {
    /// Creates a new `ConnectorCustomerMap` from a HashMap
    pub fn new(
        map: std::collections::HashMap<common_utils::id_type::MerchantConnectorAccountId, String>,
    ) -> Self {
        Self(map)
    }
}

#[cfg(feature = "v2")]
common_utils::impl_to_sql_from_sql_json!(ConnectorCustomerMap);

#[cfg(feature = "v2")]
impl std::ops::Deref for ConnectorCustomerMap {
    type Target =
        std::collections::HashMap<common_utils::id_type::MerchantConnectorAccountId, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "v2")]
impl std::ops::DerefMut for ConnectorCustomerMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Represents the type of identification document used for validation.
#[derive(
    Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize, ToSchema,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum DocumentKind {
    /// Cadastro de Pessoas Físicas - The Brazilian individual taxpayer identifier.
    Cpf,
    /// Cadastro Nacional da Pessoa Jurídica - The Brazilian business identifier.
    Cnpj,
}

impl DocumentKind {
    /// Validation function for document number depending on document type
    pub fn validate(
        &self,
        doc_number: &str,
    ) -> common_utils::errors::CustomResult<(), ValidationError> {
        match self {
            Self::Cpf => self.validate_cpf(doc_number),
            Self::Cnpj => self.validate_cnpj(doc_number),
        }
    }

    fn validate_cpf(
        self,
        doc_number: &str,
    ) -> common_utils::errors::CustomResult<(), ValidationError> {
        (doc_number.len() == common_utils::consts::CPF_LENGTH)
            .then_some(())
            .ok_or_else(|| {
                self.length_error("CPF", common_utils::consts::CPF_LENGTH, doc_number.len())
            })?;

        Ok(())
    }

    fn validate_cnpj(
        self,
        doc_number: &str,
    ) -> common_utils::errors::CustomResult<(), ValidationError> {
        (doc_number.len() == common_utils::consts::CNPJ_LENGTH)
            .then_some(())
            .ok_or_else(|| {
                self.length_error("CNPJ", common_utils::consts::CNPJ_LENGTH, doc_number.len())
            })?;

        Ok(())
    }

    fn length_error(self, name: &str, expected: usize, actual: usize) -> Report<ValidationError> {
        let message = format!("{name} document_number (expected {expected}, got {actual})");
        Report::new(ValidationError::IncorrectValueProvided {
            field_name: Box::leak(message.into_boxed_str()),
        })
    }
}
