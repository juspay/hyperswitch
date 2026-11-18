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

    /// Resolve metadata for a payment instrument, including the no-lookup cases.
    pub async fn for_payment_method<S>(
        store: &S,
        payment_method_data: Option<&crate::payment_method_data::PaymentMethodData>,
    ) -> Self
    where
        S: TrackedCardsInfoInterface + ?Sized,
        S::Error: std::fmt::Debug,
    {
        let Some(payment_method_data) = payment_method_data else {
            return Self { record: None };
        };
        let (card_number, issuer, network, card_type, country, bank_code, co_badged) =
        match payment_method_data {
            crate::payment_method_data::PaymentMethodData::Card(card) => (
                &card.card_number,
                &card.card_issuer,
                &card.card_network,
                &card.card_type,
                &card.card_issuing_country,
                &card.bank_code,
                Some(card.co_badged_card_data.is_some()),
            ),
            crate::payment_method_data::PaymentMethodData::CardWithOptionalCVC(card) => (
                &card.card_number,
                &card.card_issuer,
                &card.card_network,
                &card.card_type,
                &card.card_issuing_country,
                &card.bank_code,
                Some(card.co_badged_card_data.is_some()),
            ),
            crate::payment_method_data::PaymentMethodData::CardWithNetworkTokenDetails(data) => {
                let card = &data.card_details;
                (
                    &card.card_number,
                    &card.card_issuer,
                    &card.card_network,
                    &card.card_type,
                    &card.card_issuing_country,
                    &card.bank_code,
                    Some(card.co_badged_card_data.is_some()),
                )
            }
            crate::payment_method_data::PaymentMethodData::CardDetailsForNetworkTransactionId(card) => (
                &card.card_number,
                &card.card_issuer,
                &card.card_network,
                &card.card_type,
                &card.card_issuing_country,
                &card.bank_code,
                Some(false),
            ),
            crate::payment_method_data::PaymentMethodData::CardWithLimitedDetails(card) => (
                &card.card_number,
                &card.card_issuer,
                &card.card_network,
                &card.card_type,
                &card.card_issuing_country,
                &card.bank_code,
                None,
            ),
            _ => return Self { record: None },
        };
        // Mirror enrichment's shortcut when all required metadata is already supplied.
        let has_usable_network = network.is_some()
            && co_badged.is_none_or(|from_metadata| {
                from_metadata
                    || card_number
                        .is_cobadged_card()
                        .is_ok_and(|is_co_badged| is_co_badged)
            });
        if issuer.is_some()
            && has_usable_network
            && card_type.is_some()
            && country.is_some()
            && bank_code.is_some()
        {
            return Self { record: None };
        }
        let card_isin = card_number.get_card_isin();
        match store.get_card_info(&card_isin).await {
            Ok(resolved) => resolved,
            Err(error) => {
                router_env::logger::warn!(card_info_error=?error);
                Self { record: None }
            }
        }
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
