use std::collections::HashMap;

use crate::types::storage::revenue_recovery_redis_operation::{
    PaymentProcessorTokenStatus, PaymentProcessorTokenWithRetryInfo,
};

/// Which of the customer's cards a retry may be made with. Set by the card switching flag alone,
/// so it is the same whichever algorithm decides when the retry runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardSelection {
    /// Only the card the invoice last used.
    InvoiceCard,
    /// Any of the customer's active cards stored in redis.
    AnyCard,
}

impl CardSelection {
    pub fn from_card_switching_enabled(card_switching_enabled: bool) -> Self {
        if card_switching_enabled {
            Self::AnyCard
        } else {
            Self::InvoiceCard
        }
    }

    /// Whether the retry may be made with this card.
    pub fn admits(
        self,
        payment_processor_token: &str,
        token_status: &PaymentProcessorTokenStatus,
        invoice_token: Option<&str>,
    ) -> bool {
        match self {
            // By id alone, as the invoice's card has always been resolved
            Self::InvoiceCard => invoice_token == Some(payment_processor_token),
            Self::AnyCard => token_status.is_active != Some(false),
        }
    }
}

/// Where the cards a retry may be made with stand, once the algorithm has decided its time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardAvailability {
    /// This card can be retried at the decided time.
    Ready { payment_processor_token: String },
    /// No card can be retried yet; the soonest frees up after this many hours.
    WaitHours(i64),
    /// Every card the retry may be made with is hard declined.
    HardDecline,
    /// The retry has no card to be made with.
    NoCard,
}

/// Pick the card for a retry whose time the algorithm has already decided. A card that is hard
/// declined is never picked, and one still waiting out its network's retry limits is not ready.
/// Of the ready cards, one with no failure on record is preferred, then the invoice's own card,
/// then the one with the most retries left this month.
pub fn pick_card_for_retry_time(
    card_selection: CardSelection,
    tokens: &HashMap<String, PaymentProcessorTokenWithRetryInfo>,
    invoice_token: Option<&str>,
) -> CardAvailability {
    let admitted_cards: Vec<_> = tokens
        .iter()
        .filter(|(payment_processor_token, token)| {
            card_selection.admits(payment_processor_token, &token.token_status, invoice_token)
        })
        .collect();

    if admitted_cards.is_empty() {
        return CardAvailability::NoCard;
    }

    let retryable_cards: Vec<_> = admitted_cards
        .into_iter()
        .filter(|(_, token)| !token.token_status.is_hard_decline.unwrap_or(false))
        .collect();

    let ready_card = retryable_cards
        .iter()
        .filter(|(_, token)| token.retry_wait_time_hours <= 0)
        .min_by_key(|&&(payment_processor_token, token)| {
            (
                token.token_status.error_code.is_some(),
                invoice_token != Some(payment_processor_token.as_str()),
                std::cmp::Reverse(token.monthly_retry_remaining),
                // Last, so the pick does not depend on the map's order
                payment_processor_token.as_str(),
            )
        });

    match ready_card {
        Some((payment_processor_token, _)) => CardAvailability::Ready {
            payment_processor_token: (*payment_processor_token).clone(),
        },
        None => retryable_cards
            .iter()
            .map(|(_, token)| token.retry_wait_time_hours)
            .min()
            .map_or(CardAvailability::HardDecline, CardAvailability::WaitHours),
    }
}

#[cfg(test)]
mod tests {
    use common_utils::id_type;

    use super::*;
    use crate::types::storage::revenue_recovery_redis_operation::PaymentProcessorTokenDetails;

    const INVOICE_CARD: &str = "invoice_card";

    #[derive(Clone, Copy)]
    struct Card {
        failed: bool,
        hard_declined: bool,
        active: bool,
        wait_hours: i64,
        retries_left: i32,
    }

    /// A card that last failed softly and can be retried right away.
    const SOFT_DECLINED: Card = Card {
        failed: true,
        hard_declined: false,
        active: true,
        wait_hours: 0,
        retries_left: 10,
    };

    fn tokens(cards: &[(&str, Card)]) -> HashMap<String, PaymentProcessorTokenWithRetryInfo> {
        let cell_id = id_type::CellId::from_string("00").expect("valid cell id");
        cards
            .iter()
            .map(|&(payment_processor_token, card)| {
                let token_status = PaymentProcessorTokenStatus {
                    payment_processor_token_details: PaymentProcessorTokenDetails {
                        payment_processor_token: payment_processor_token.to_string(),
                        expiry_month: None,
                        expiry_year: None,
                        card_issuer: None,
                        last_four_digits: None,
                        card_network: None,
                        card_type: None,
                        card_isin: None,
                    },
                    inserted_by_attempt_id: id_type::GlobalAttemptId::generate(&cell_id),
                    error_code: card.failed.then(|| "card_declined".to_string()),
                    daily_retry_history: HashMap::new(),
                    scheduled_at: None,
                    is_hard_decline: Some(card.hard_declined),
                    modified_at: None,
                    is_active: Some(card.active),
                    account_update_history: None,
                    decision_threshold: None,
                };
                (
                    payment_processor_token.to_string(),
                    PaymentProcessorTokenWithRetryInfo {
                        token_status,
                        retry_wait_time_hours: card.wait_hours,
                        monthly_retry_remaining: card.retries_left,
                        total_30_day_retries: 0,
                    },
                )
            })
            .collect()
    }

    fn ready(payment_processor_token: &str) -> CardAvailability {
        CardAvailability::Ready {
            payment_processor_token: payment_processor_token.to_string(),
        }
    }

    fn pick(card_selection: CardSelection, cards: &[(&str, Card)]) -> CardAvailability {
        pick_card_for_retry_time(card_selection, &tokens(cards), Some(INVOICE_CARD))
    }

    #[test]
    fn invoice_card_selection_never_switches_cards() {
        let clean = Card {
            failed: false,
            ..SOFT_DECLINED
        };
        let hard_declined = Card {
            hard_declined: true,
            ..SOFT_DECLINED
        };
        let waiting = Card {
            wait_hours: 5,
            ..SOFT_DECLINED
        };

        for (cards, expected) in [
            (
                vec![(INVOICE_CARD, SOFT_DECLINED), ("other", clean)],
                ready(INVOICE_CARD),
            ),
            (
                vec![(INVOICE_CARD, hard_declined), ("other", clean)],
                CardAvailability::HardDecline,
            ),
            (
                vec![(INVOICE_CARD, waiting), ("other", clean)],
                CardAvailability::WaitHours(5),
            ),
            (vec![("other", clean)], CardAvailability::NoCard),
        ] {
            assert_eq!(pick(CardSelection::InvoiceCard, &cards), expected);
        }
    }

    #[test]
    fn invoice_card_selection_keeps_a_deactivated_invoice_card() {
        let deactivated = Card {
            active: false,
            ..SOFT_DECLINED
        };
        assert_eq!(
            pick(CardSelection::InvoiceCard, &[(INVOICE_CARD, deactivated)]),
            ready(INVOICE_CARD)
        );
    }

    #[test]
    fn any_card_selection_prefers_a_card_with_no_failure_then_the_invoice_card() {
        let clean = Card {
            failed: false,
            ..SOFT_DECLINED
        };
        let more_retries_left = Card {
            retries_left: 20,
            ..SOFT_DECLINED
        };

        assert_eq!(
            pick(
                CardSelection::AnyCard,
                &[(INVOICE_CARD, SOFT_DECLINED), ("clean", clean)]
            ),
            ready("clean")
        );
        assert_eq!(
            pick(
                CardSelection::AnyCard,
                &[(INVOICE_CARD, SOFT_DECLINED), ("other", more_retries_left)]
            ),
            ready(INVOICE_CARD)
        );
        assert_eq!(
            pick(
                CardSelection::AnyCard,
                &[("other", SOFT_DECLINED), ("more", more_retries_left)]
            ),
            ready("more")
        );
    }

    #[test]
    fn any_card_selection_switches_away_from_an_unusable_invoice_card() {
        for unusable in [
            Card {
                hard_declined: true,
                ..SOFT_DECLINED
            },
            Card {
                active: false,
                ..SOFT_DECLINED
            },
            Card {
                wait_hours: 5,
                ..SOFT_DECLINED
            },
        ] {
            assert_eq!(
                pick(
                    CardSelection::AnyCard,
                    &[(INVOICE_CARD, unusable), ("other", SOFT_DECLINED)]
                ),
                ready("other")
            );
        }
    }

    #[test]
    fn any_card_selection_waits_for_the_soonest_card_and_stops_on_hard_declines() {
        let waiting = |wait_hours| Card {
            wait_hours,
            ..SOFT_DECLINED
        };
        let hard_declined = Card {
            hard_declined: true,
            ..SOFT_DECLINED
        };
        let deactivated = Card {
            active: false,
            ..SOFT_DECLINED
        };

        assert_eq!(
            pick(
                CardSelection::AnyCard,
                &[(INVOICE_CARD, waiting(9)), ("other", waiting(3))]
            ),
            CardAvailability::WaitHours(3)
        );
        assert_eq!(
            pick(
                CardSelection::AnyCard,
                &[(INVOICE_CARD, hard_declined), ("other", hard_declined)]
            ),
            CardAvailability::HardDecline
        );
        assert_eq!(
            pick(CardSelection::AnyCard, &[(INVOICE_CARD, deactivated)]),
            CardAvailability::NoCard
        );
    }
}
