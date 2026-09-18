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
use time::{Duration, OffsetDateTime};

use crate::{
    domain::{
        cloudwatch::{Comparison, State, Transition},
        notifier::{chat::ChatNotification, Outcome as DeliveryOutcome},
    },
    logger,
    settings::cloudwatch::{ComparisonOperator, MissingDataPolicy, Statistic},
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
        let message = render(transition, &state.conf.cloudwatch.client.region);
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

/// The message an operator reads. It contains the complete metric query and rule, followed by the
/// datapoints CloudWatch returned, so the state can be investigated without making another call.
fn render(transition: &Transition, region: &str) -> String {
    let dimensions = transition
        .dimensions
        .iter()
        .map(|(name, value)| format!("    {name}: {value}"))
        .collect::<Vec<_>>()
        .join("\n");
    let dimensions = if dimensions.is_empty() {
        "    (none)".to_owned()
    } else {
        dimensions
    };
    let datapoints_to_alarm = transition
        .datapoints_to_alarm
        .unwrap_or(transition.evaluation_periods);
    let readings = transition
        .readings
        .iter()
        .enumerate()
        .map(|(index, reading)| {
            let offset = i64::try_from(index)
                .unwrap_or(i64::MAX)
                .saturating_mul(i64::from(transition.period));
            let at = transition.range_start + Duration::seconds(offset);

            match reading {
                Some(value) => format!(
                    "  {}   {}   {}",
                    timestamp(at),
                    value,
                    if transition
                        .comparison_operator
                        .breaches(*value, transition.threshold)
                    {
                        "BREACHING"
                    } else {
                        "OK"
                    }
                ),
                None => format!("  {}   missing", timestamp(at)),
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    let heading = state_heading(transition.to);

    format!(
        "*{heading}* · {name}\n\
         *Severity:* {severity}\n\
         _State: {movement} · Evaluated {evaluated_at}_\n\n\
         {description}\n\n\
         *Metric*\n\
         *Namespace:* {namespace}\n\
         *Name:* {metric} · *Statistic:* {statistic}\n\
         *Dimensions:*\n{dimensions}\n\
         *Region:* {region}\n\n\
         *Condition*\n\
         {statistic} {metric} {operator} {threshold}\n\
         {datapoints_to_alarm} of {evaluation_periods} datapoints · {period}-second periods\n\
         *Missing data:* {missing_data}\n\n\
         *Datapoints* · {range_start} to {range_end}\n\
         {readings}\n\n\
         *Result:* {breaching} of {evaluation_periods} effective datapoints breached; state is {state}.\n\
         *Definition:* {definition_id} · *Classification:* {classification}",
        heading = heading,
        name = transition.name,
        severity = transition.severity.to_uppercase(),
        movement = movement(transition),
        evaluated_at = timestamp(transition.range_end),
        description = transition.description,
        namespace = transition.namespace,
        metric = transition.metric_name,
        statistic = statistic(transition.statistic),
        dimensions = dimensions,
        region = region,
        operator = operator(transition.comparison_operator),
        threshold = transition.threshold,
        datapoints_to_alarm = datapoints_to_alarm,
        evaluation_periods = transition.evaluation_periods,
        period = transition.period,
        missing_data = missing_data(transition.treat_missing_data),
        range_start = timestamp(transition.range_start),
        range_end = timestamp(transition.range_end),
        readings = readings,
        breaching = transition.breaching_datapoints,
        state = label(transition.to),
        definition_id = transition.definition_id,
        classification = transition.classification,
    )
}

/// The heading describes state, not severity. Severity is a free-form configuration key and has no
/// ordering semantics in this service, so it must never decide presentation or wording.
fn state_heading(state: State) -> &'static str {
    match state {
        State::Alarm => "ALARM",
        State::Ok => "RESOLVED",
        State::InsufficientData => "INSUFFICIENT DATA",
    }
}

fn timestamp(value: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        value.year(),
        u8::from(value.month()),
        value.day(),
        value.hour(),
        value.minute(),
        value.second(),
    )
}

fn statistic(value: Statistic) -> &'static str {
    match value {
        Statistic::Average => "Average",
        Statistic::Maximum => "Maximum",
        Statistic::Minimum => "Minimum",
        Statistic::Sum => "Sum",
    }
}

fn operator(value: ComparisonOperator) -> &'static str {
    match value {
        ComparisonOperator::GreaterThanThreshold => ">",
        ComparisonOperator::GreaterThanOrEqualToThreshold => ">=",
        ComparisonOperator::LessThanThreshold => "<",
        ComparisonOperator::LessThanOrEqualToThreshold => "<=",
    }
}

fn missing_data(value: MissingDataPolicy) -> &'static str {
    match value {
        MissingDataPolicy::Breaching => "breaching",
        MissingDataPolicy::NotBreaching => "notBreaching",
        MissingDataPolicy::Ignore => "ignore",
        MissingDataPolicy::Missing => "missing",
    }
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
