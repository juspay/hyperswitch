//! Turning breaching rules into messages and handing them to the Notifier.
//!
//! Every rule currently in `Alarm` is announced, every time this runs. There is no memory of what
//! was announced before, so a condition that stays breaching is announced again on the next call —
//! transitions are [a separate ticket](https://github.com/juspay/hyperswitch-cloud/issues/23496)
//! and until it lands the caller is a human, not a clock.
//!
//! Severities do not suppress one another. CPU at 92% breaches sev3, sev2 and sev1, and all three
//! are announced, because that is what the AWS alarms we run alongside do today.
//!
//! One failed send does not stop the rest. A definition is not retried and nothing is queued;
//! delivery recovery is [its own
//! ticket](https://github.com/juspay/hyperswitch-cloud/issues/23485).

use hyperswitch_masking::Secret;

use super::{Catalogue, Outcome, RuleState};
use crate::{
    domain::{
        cloudwatch::State,
        notifier::{chat::ChatNotification, Outcome as DeliveryOutcome},
    },
    logger,
    state::AppState,
};

#[derive(Debug, PartialEq)]
pub struct Announcement {
    pub definition_id: String,
    pub severity: String,
    pub destination: String,
    pub message: String,
    pub delivery: Delivery,
}

#[derive(Debug, PartialEq)]
pub enum Delivery {
    Delivered {
        message_id: Option<String>,
    },
    /// The destination was reached and said no, for a reason it named.
    Refused {
        code: String,
    },
    /// Nothing is known about whether it arrived.
    Failed,
    /// The severity names a destination the chat registry does not hold. Boot checks this, so it
    /// means configuration changed underneath a running process.
    UnknownDestination,
}

/// Announce every rule the catalogue found breaching.
pub async fn announce(state: &AppState, catalogue: &Catalogue) -> Vec<Announcement> {
    let mut announcements = Vec::new();

    for definition in &catalogue.definitions {
        let Outcome::Evaluated { readings, rules } = &definition.outcome else {
            continue;
        };

        for rule in rules.iter().filter(|rule| rule.state == State::Alarm) {
            let message = render(definition, rule, readings.iter().rev().flatten().next());

            let Some(destination) = state.conf.cloudwatch.destinations.get(&rule.severity) else {
                logger::error!(
                    definition = %definition.id,
                    severity = %rule.severity,
                    "A breaching rule has no destination configured"
                );
                announcements.push(Announcement {
                    definition_id: definition.id.clone(),
                    severity: rule.severity.clone(),
                    destination: String::new(),
                    message,
                    delivery: Delivery::UnknownDestination,
                });
                continue;
            };

            let delivery = deliver(state, destination, &message).await;
            if delivery != (Delivery::Delivered { message_id: None }) {
                logger::info!(
                    definition = %definition.id,
                    severity = %rule.severity,
                    destination = %destination,
                    delivery = ?delivery,
                    "Announced a breaching rule"
                );
            }

            announcements.push(Announcement {
                definition_id: definition.id.clone(),
                severity: rule.severity.clone(),
                destination: destination.clone(),
                message,
                delivery,
            });
        }
    }

    announcements
}

async fn deliver(state: &AppState, destination: &str, message: &str) -> Delivery {
    let Some(notifier) = state.chat.get(destination) else {
        return Delivery::UnknownDestination;
    };

    match notifier
        .notify(ChatNotification {
            text: Secret::new(message.to_owned()),
            reply_to: None,
        })
        .await
    {
        Ok(DeliveryOutcome::Delivered(receipt)) => Delivery::Delivered {
            message_id: receipt.message_id,
        },
        Ok(DeliveryOutcome::Refused(refusal)) => Delivery::Refused { code: refusal.code },
        Err(error) => {
            logger::error!(
                destination = %destination,
                error = ?error,
                "A CloudWatch announcement could not be delivered"
            );
            Delivery::Failed
        }
    }
}

/// The message an operator reads.
///
/// The catalogue's `description` is hand-written for exactly this and leads. Everything after it is
/// what the description cannot say: which stream, what the reading actually was, and what it was
/// compared against.
fn render(definition: &super::Evaluation, rule: &RuleState, observed: Option<&f64>) -> String {
    let dimensions = definition
        .dimensions
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(", ");

    let observed = match observed {
        Some(value) => format!("{value}"),
        None => "no reading".to_owned(),
    };

    format!(
        "[{severity}] {name}\n{description}\n\n{metric} observed {observed} against a threshold of \
         {threshold} ({dimensions}, {period}s periods)",
        severity = rule.severity.to_uppercase(),
        name = definition.name,
        description = rule.description,
        metric = definition.metric_name,
        threshold = rule.threshold,
        period = definition.period,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use std::collections::BTreeMap;

    use super::{super::Evaluation, *};

    fn evaluation() -> Evaluation {
        Evaluation {
            id: "rds_primary_cpu".to_owned(),
            name: "rds-primary-cpu".to_owned(),
            classification: "rds-alerts".to_owned(),
            metric_name: "CPUUtilization".to_owned(),
            dimensions: BTreeMap::from([(
                "DBInstanceIdentifier".to_owned(),
                "hyperswitchdb-primary".to_owned(),
            )]),
            period: 60,
            outcome: Outcome::Evaluated {
                readings: vec![],
                rules: vec![],
            },
        }
    }

    fn rule() -> RuleState {
        RuleState {
            severity: "sev2".to_owned(),
            state: State::Alarm,
            threshold: 85.0,
            description: "SEV2: RDS primary database CPU utilization is above 85%.".to_owned(),
        }
    }

    #[test]
    fn a_message_leads_with_the_operators_own_words() {
        let message = render(&evaluation(), &rule(), Some(&87.25));

        assert_eq!(
            message,
            "[SEV2] rds-primary-cpu\n\
             SEV2: RDS primary database CPU utilization is above 85%.\n\n\
             CPUUtilization observed 87.25 against a threshold of 85 \
             (DBInstanceIdentifier=hyperswitchdb-primary, 60s periods)"
        );
    }

    /// A rule can alarm on missing data alone, so there may be no reading to quote.
    #[test]
    fn a_breach_with_no_reading_says_so_rather_than_inventing_a_number() {
        let message = render(&evaluation(), &rule(), None);

        assert!(
            message.contains("observed no reading against a threshold of 85"),
            "{message}"
        );
    }

    /// The newest real reading, not the newest slot, which may be a gap.
    #[test]
    fn the_quoted_reading_is_the_most_recent_one_that_exists() {
        let readings = [Some(10.0), Some(87.25), None];
        let observed = readings.iter().rev().flatten().next();

        assert_eq!(observed, Some(&87.25));
    }
}
