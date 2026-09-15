//! Turning state transitions into messages and handing them to the Notifier.
//!
//! Only a rule that says something *different* from what it said one evaluation ago is announced,
//! which is what keeps a sustained breach from being reported every minute. Recovery and
//! `InsufficientData` are transitions too, so they are announced alongside the breaches.
//!
//! Nothing is remembered between calls: the previous evaluation is fetched from CloudWatch rather
//! than stored, so two callers running at once will both see the same transition and both announce
//! it. Duplicates are accepted; a store to prevent them would be alert management, which belongs
//! to a different effort.
//!
//! One failed send does not stop the rest. Nothing is retried or queued — delivery recovery is
//! [its own ticket](https://github.com/juspay/hyperswitch-cloud/issues/23485).

use hyperswitch_masking::Secret;
use time::OffsetDateTime;

use crate::{
    domain::{
        cloudwatch::{Comparison, State, Transition},
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
    /// A dry run reached this point and stopped.
    Skipped,
}

/// What one request produced: both evaluations, what changed, and what was said about it.
pub struct Announced {
    pub comparison: Comparison,
    pub announcements: Vec<Announcement>,
}

/// Compare the catalogue against its previous evaluation and announce what changed.
///
/// `deliver` is what separates the notify route from its dry run: messages are rendered either
/// way, so a dry run shows exactly what would have been sent.
pub async fn evaluate_and_announce(
    state: &AppState,
    now: OffsetDateTime,
    deliver: bool,
) -> Announced {
    let comparison = super::compare_catalogue(state, now).await;
    let announcements = announce(state, &comparison.transitions, deliver).await;

    Announced {
        comparison,
        announcements,
    }
}

async fn announce(
    state: &AppState,
    transitions: &[Transition],
    deliver: bool,
) -> Vec<Announcement> {
    let mut announcements = Vec::with_capacity(transitions.len());

    for transition in transitions {
        let message = render(transition);
        let destination = state.conf.cloudwatch.destinations.get(&transition.severity);

        let delivery = match (destination, deliver) {
            (None, _) => {
                logger::error!(
                    definition = %transition.definition_id,
                    severity = %transition.severity,
                    "A transition has no destination configured"
                );
                Delivery::UnknownDestination
            }
            (Some(_), false) => Delivery::Skipped,
            (Some(destination), true) => deliver_to(state, destination, &message).await,
        };

        logger::info!(
            definition = %transition.definition_id,
            severity = %transition.severity,
            from = ?transition.from,
            to = ?transition.to,
            delivery = ?delivery,
            "CloudWatch rule changed state"
        );

        announcements.push(Announcement {
            definition_id: transition.definition_id.clone(),
            severity: transition.severity.clone(),
            destination: destination.cloned().unwrap_or_default(),
            message,
            delivery,
        });
    }

    announcements
}

async fn deliver_to(state: &AppState, destination: &str, message: &str) -> Delivery {
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
/// what the description cannot say: which way the rule moved, which stream, and what it was
/// compared against.
fn render(transition: &Transition) -> String {
    let dimensions = transition
        .dimensions
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "[{severity}] {name} {movement}\n{description}\n\n{metric} against a threshold of \
         {threshold} ({dimensions}, {period}s periods)",
        severity = transition.severity.to_uppercase(),
        name = transition.name,
        movement = movement(transition),
        description = transition.description,
        metric = transition.metric_name,
        threshold = transition.threshold,
        period = transition.period,
    )
}

fn movement(transition: &Transition) -> String {
    match transition.from {
        Some(from) => format!("{} → {}", label(from), label(transition.to)),
        // The previous evaluation could not be read, so only the destination is known.
        None => label(transition.to).to_owned(),
    }
}

fn label(state: State) -> &'static str {
    match state {
        State::Ok => "OK",
        State::Alarm => "ALARM",
        State::InsufficientData => "INSUFFICIENT_DATA",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn transition(from: Option<State>, to: State) -> Transition {
        Transition {
            definition_id: "rds_primary_cpu".to_owned(),
            name: "rds-primary-cpu".to_owned(),
            metric_name: "CPUUtilization".to_owned(),
            dimensions: BTreeMap::from([(
                "DBInstanceIdentifier".to_owned(),
                "hyperswitchdb-primary".to_owned(),
            )]),
            period: 60,
            severity: "sev2".to_owned(),
            from,
            to,
            threshold: 85.0,
            description: "SEV2: RDS primary database CPU utilization is above 85%.".to_owned(),
        }
    }

    #[test]
    fn a_message_leads_with_the_operators_own_words_and_says_which_way_it_moved() {
        let message = render(&transition(Some(State::Ok), State::Alarm));

        assert_eq!(
            message,
            "[SEV2] rds-primary-cpu OK → ALARM\n\
             SEV2: RDS primary database CPU utilization is above 85%.\n\n\
             CPUUtilization against a threshold of 85 \
             (DBInstanceIdentifier=hyperswitchdb-primary, 60s periods)"
        );
    }

    #[test]
    fn a_recovery_reads_as_one() {
        let message = render(&transition(Some(State::Alarm), State::Ok));

        assert!(
            message.starts_with("[SEV2] rds-primary-cpu ALARM → OK"),
            "{message}"
        );
    }

    /// Nothing is known about where it came from, so the message does not invent a previous state.
    #[test]
    fn an_unknown_previous_state_names_only_where_the_rule_is_now() {
        let message = render(&transition(None, State::Alarm));

        assert!(
            message.starts_with("[SEV2] rds-primary-cpu ALARM\n"),
            "{message}"
        );
    }
}
