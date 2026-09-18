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

    format!(
        "[{severity}] {name}\n\
         State: {movement}\n\
         Evaluated at: {evaluated_at}\n\
         Definition: {definition_id}\n\
         Classification: {classification}\n\n\
         {description}\n\n\
         Metric\n\
           Namespace: {namespace}\n\
           Name: {metric}\n\
           Statistic: {statistic}\n\
           Dimensions:\n{dimensions}\n\
           Region: {region}\n\n\
         Condition\n\
           Expression: {statistic} {metric} {operator} {threshold}\n\
           Datapoints to alarm: {datapoints_to_alarm} of {evaluation_periods}\n\
           Period: {period} seconds\n\
           Missing data: {missing_data}\n\n\
         Datapoints fetched, oldest first ({range_start} to {range_end}):\n\
         {readings}\n\n\
         Result: {breaching} of {evaluation_periods} effective datapoints breached; state is {state}.",
        severity = transition.severity.to_uppercase(),
        name = transition.name,
        movement = movement(transition),
        evaluated_at = timestamp(transition.range_end),
        definition_id = transition.definition_id,
        classification = transition.classification,
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
    )
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use std::collections::BTreeMap;

    use time::macros::datetime;

    use super::*;

    fn transition(from: Option<State>, to: State) -> Transition {
        Transition {
            definition_id: "rds_primary_cpu".to_owned(),
            name: "rds-primary-cpu".to_owned(),
            classification: "rds-alerts".to_owned(),
            namespace: "AWS/RDS".to_owned(),
            metric_name: "CPUUtilization".to_owned(),
            statistic: Statistic::Average,
            dimensions: BTreeMap::from([(
                "DBInstanceIdentifier".to_owned(),
                "hyperswitchdb-primary".to_owned(),
            )]),
            period: 60,
            range_start: datetime!(2026-09-11 12:02:00 UTC),
            range_end: datetime!(2026-09-11 12:07:00 UTC),
            readings: vec![Some(70.0), Some(75.0), Some(82.1), Some(93.4), Some(96.2)],
            severity: "sev2".to_owned(),
            from,
            to,
            threshold: 85.0,
            comparison_operator: ComparisonOperator::GreaterThanOrEqualToThreshold,
            evaluation_periods: 3,
            datapoints_to_alarm: Some(2),
            treat_missing_data: MissingDataPolicy::NotBreaching,
            breaching_datapoints: 2,
            description: "SEV2: RDS primary database CPU utilization is above 85%.".to_owned(),
        }
    }

    #[test]
    fn a_message_leads_with_the_operators_own_words_and_says_which_way_it_moved() {
        let message = render(&transition(Some(State::Ok), State::Alarm), "ap-south-1");

        assert_eq!(
            message,
            concat!(
                "[SEV2] rds-primary-cpu\n",
                "State: OK → ALARM\n",
                "Evaluated at: 2026-09-11 12:07:00 UTC\n",
                "Definition: rds_primary_cpu\n",
                "Classification: rds-alerts\n\n",
                "SEV2: RDS primary database CPU utilization is above 85%.\n\n",
                "Metric\n",
                "Namespace: AWS/RDS\n",
                "Name: CPUUtilization\n",
                "Statistic: Average\n",
                "Dimensions:\n",
                "    DBInstanceIdentifier: hyperswitchdb-primary\n",
                "Region: ap-south-1\n\n",
                "Condition\n",
                "Expression: Average CPUUtilization >= 85\n",
                "Datapoints to alarm: 2 of 3\n",
                "Period: 60 seconds\n",
                "Missing data: notBreaching\n\n",
                "Datapoints fetched, oldest first ",
                "(2026-09-11 12:02:00 UTC to 2026-09-11 12:07:00 UTC):\n",
                "  2026-09-11 12:02:00 UTC   70   OK\n",
                "  2026-09-11 12:03:00 UTC   75   OK\n",
                "  2026-09-11 12:04:00 UTC   82.1   OK\n",
                "  2026-09-11 12:05:00 UTC   93.4   BREACHING\n",
                "  2026-09-11 12:06:00 UTC   96.2   BREACHING\n\n",
                "Result: 2 of 3 effective datapoints breached; state is ALARM."
            )
        );
    }

    #[test]
    fn a_recovery_reads_as_one() {
        let message = render(&transition(Some(State::Alarm), State::Ok), "ap-south-1");

        assert!(
            message.starts_with("[SEV2] rds-primary-cpu\nState: ALARM → OK"),
            "{message}"
        );
    }

    /// Nothing is known about where it came from, so the message does not invent a previous state.
    #[test]
    fn an_unknown_previous_state_names_only_where_the_rule_is_now() {
        let message = render(&transition(None, State::Alarm), "ap-south-1");

        assert!(
            message.starts_with("[SEV2] rds-primary-cpu\nState: ALARM\n"),
            "{message}"
        );
    }
}
