use common_utils::errors;
use diesel_models::cards_info;

/// The result of a completed BIN lookup. A missing database record is valid.
///
/// The contents are private so enrichment requires an explicit resolved result.
#[derive(Clone)]
pub struct ResolvedCardInfo {
    record: Option<cards_info::CardInfo>,
}

impl ResolvedCardInfo {
    pub fn from_lookup_result<E>(
        result: errors::CustomResult<Option<cards_info::CardInfo>, E>,
    ) -> errors::CustomResult<Self, E> {
        result.map(|record| Self { record })
    }

    pub fn as_ref(&self) -> Option<&cards_info::CardInfo> {
        self.record.as_ref()
    }

    pub fn into_record(self) -> Option<cards_info::CardInfo> {
        self.record
    }
}

#[router_derive::track_async_interface]
#[async_trait::async_trait]
pub trait CardsInfoInterface {
    type Error;
    async fn get_card_info(
        &self,
        card_iin: &str,
    ) -> errors::CustomResult<ResolvedCardInfo, Self::Error>;

    async fn add_card_info(
        &self,
        data: cards_info::CardInfo,
    ) -> errors::CustomResult<cards_info::CardInfo, Self::Error>;
    async fn update_card_info(
        &self,
        card_iin: String,
        data: cards_info::UpdateCardInfo,
    ) -> errors::CustomResult<cards_info::CardInfo, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::ResolvedCardInfo;

    #[test]
    fn a_missing_bin_is_a_successful_resolution() {
        let resolved = ResolvedCardInfo::from_lookup_result::<std::io::Error>(Ok(None));
        assert!(resolved.as_ref().is_ok_and(|info| info.as_ref().is_none()));
        assert!(matches!(
            resolved.map(ResolvedCardInfo::into_record),
            Ok(None)
        ));
    }

    #[test]
    fn a_failed_lookup_remains_an_error() {
        let failure = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "lookup failed");
        let result = ResolvedCardInfo::from_lookup_result(Err(error_stack::Report::new(failure)));
        assert!(
            matches!(result, Err(error) if error.current_context().kind() == std::io::ErrorKind::ConnectionReset)
        );
    }
}
