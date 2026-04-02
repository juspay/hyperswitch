//! Conversion implementations for Config types

use crate::transformers::ForeignFrom;

impl ForeignFrom<hyperswitch_domain_models::configs::ConfigUpdate>
    for diesel_models::configs::ConfigUpdate
{
    fn foreign_from(from: hyperswitch_domain_models::configs::ConfigUpdate) -> Self {
        match from {
            hyperswitch_domain_models::configs::ConfigUpdate::Update { config } => {
                Self::Update { config }
            }
        }
    }
}

impl ForeignFrom<hyperswitch_domain_models::configs::ConfigUpdate>
    for diesel_models::configs::ConfigUpdateInternal
{
    fn foreign_from(from: hyperswitch_domain_models::configs::ConfigUpdate) -> Self {
        let diesel_update = diesel_models::configs::ConfigUpdate::foreign_from(from);
        Self::from(diesel_update)
    }
}

impl ForeignFrom<hyperswitch_domain_models::configs::ConfigNew>
    for diesel_models::configs::ConfigNew
{
    fn foreign_from(from: hyperswitch_domain_models::configs::ConfigNew) -> Self {
        Self {
            key: from.key,
            config: from.config,
        }
    }
}

impl ForeignFrom<hyperswitch_domain_models::configs::Config> for diesel_models::configs::Config {
    fn foreign_from(from: hyperswitch_domain_models::configs::Config) -> Self {
        Self {
            key: from.key,
            config: from.config,
        }
    }
}

impl ForeignFrom<diesel_models::configs::Config> for hyperswitch_domain_models::configs::Config {
    fn foreign_from(from: diesel_models::configs::Config) -> Self {
        Self {
            key: from.key,
            config: from.config,
        }
    }
}
