use common_utils::{errors, ext_traits::ValueExt, link_utils::GenericLinkStatus};
use diesel::{associations::HasTable, ExpressionMethods};
use error_stack::{report, Report, ResultExt};

use super::generics;
use crate::{
    errors as db_errors,
    generic_link::{
        GenericLink, GenericLinkData, GenericLinkNew, GenericLinkState, GenericLinkUpdateInternal,
        PaymentMethodCollectLink, PayoutLink, PayoutLinkUpdate,
    },
    schema::generic_link::dsl,
    PgPooledConn, StorageResult,
};

impl GenericLinkNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<GenericLinkState> {
        generics::generic_insert(conn, self)
            .await
            .and_then(|res: GenericLink| {
                GenericLinkState::try_from(res)
                    .change_context(db_errors::DatabaseError::Others)
                    .attach_printable("failed to parse generic link data from DB")
            })
    }

    pub async fn insert_pm_collect_link(
        self,
        conn: &PgPooledConn,
    ) -> StorageResult<PaymentMethodCollectLink> {
        generics::generic_insert(conn, self)
            .await
            .and_then(|res: GenericLink| {
                PaymentMethodCollectLink::try_from(res)
                    .change_context(db_errors::DatabaseError::Others)
                    .attach_printable("failed to parse payment method collect link data from DB")
            })
    }

    pub async fn insert_payout_link(self, conn: &PgPooledConn) -> StorageResult<PayoutLink> {
        generics::generic_insert(conn, self)
            .await
            .and_then(|res: GenericLink| {
                PayoutLink::try_from(res)
                    .change_context(db_errors::DatabaseError::Others)
                    .attach_printable("failed to parse payout link data from DB")
            })
    }
}

impl GenericLink {
    pub async fn find_generic_link_by_link_id(
        conn: &PgPooledConn,
        link_id: &str,
    ) -> StorageResult<GenericLinkState> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::link_id.eq(link_id.to_owned()),
        )
        .await
        .and_then(|res: Self| {
            GenericLinkState::try_from(res)
                .change_context(db_errors::DatabaseError::Others)
                .attach_printable("failed to parse generic link data from DB")
        })
    }

    pub async fn find_pm_collect_link_by_link_id(
        conn: &PgPooledConn,
        link_id: &str,
    ) -> StorageResult<PaymentMethodCollectLink> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::link_id.eq(link_id.to_owned()),
        )
        .await
        .and_then(|res: Self| {
            PaymentMethodCollectLink::try_from(res)
                .change_context(db_errors::DatabaseError::Others)
                .attach_printable("failed to parse payment method collect link data from DB")
        })
    }

    pub async fn find_payout_link_by_link_id(
        conn: &PgPooledConn,
        link_id: &str,
    ) -> StorageResult<PayoutLink> {
        generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
            conn,
            dsl::link_id.eq(link_id.to_owned()),
        )
        .await
        .and_then(|res: Self| {
            PayoutLink::try_from(res)
                .change_context(db_errors::DatabaseError::Others)
                .attach_printable("failed to parse payout link data from DB")
        })
    }
}

impl PayoutLink {
    pub async fn update_payout_link(
        self,
        conn: &PgPooledConn,
        payout_link_update: PayoutLinkUpdate,
    ) -> StorageResult<Self> {
        generics::generic_update_with_results::<<Self as HasTable>::Table, _, _, _>(
            conn,
            dsl::link_id.eq(self.link_id.to_owned()),
            GenericLinkUpdateInternal::from(payout_link_update),
        )
        .await
        .and_then(|mut payout_links| {
            payout_links
                .pop()
                .ok_or(error_stack::report!(db_errors::DatabaseError::NotFound))
        })
        .or_else(|error| match error.current_context() {
            db_errors::DatabaseError::NoFieldsToUpdate => Ok(self),
            _ => Err(error),
        })
    }
}

impl TryFrom<GenericLink> for GenericLinkState {
    type Error = Report<errors::ParsingError>;
    fn try_from(db_val: GenericLink) -> Result<Self, Self::Error> {
        let link_data = match db_val.link_type {
            common_enums::GenericLinkType::PaymentMethodCollect => {
                let link_data = db_val
                    .link_data
                    .parse_value("PaymentMethodCollectLinkData")?;
                GenericLinkData::PaymentMethodCollect(link_data)
            }
            common_enums::GenericLinkType::PayoutLink => {
                let link_data = db_val.link_data.parse_value("PayoutLinkData")?;
                GenericLinkData::PayoutLink(link_data)
            }
        };

        Ok(Self {
            link_id: db_val.link_id,
            primary_reference: db_val.primary_reference,
            merchant_id: db_val.merchant_id,
            created_at: db_val.created_at,
            last_modified_at: db_val.last_modified_at,
            expiry: db_val.expiry,
            link_data,
            link_status: db_val.link_status,
            link_type: db_val.link_type,
            url: db_val.url,
            return_url: db_val.return_url,
        })
    }
}

impl TryFrom<GenericLink> for PaymentMethodCollectLink {
    type Error = Report<errors::ParsingError>;
    fn try_from(db_val: GenericLink) -> Result<Self, Self::Error> {
        let (link_data, link_status) = match db_val.link_type {
            common_enums::GenericLinkType::PaymentMethodCollect => {
                let link_data = db_val
                    .link_data
                    .parse_value("PaymentMethodCollectLinkData")?;
                let link_status = match db_val.link_status {
                    GenericLinkStatus::PaymentMethodCollect(status) => Ok(status),
                    _ => Err(report!(errors::ParsingError::EnumParseFailure(
                        "GenericLinkStatus"
                    )))
                    .attach_printable_lazy(|| {
                        format!(
                            "Invalid status for PaymentMethodCollectLink - {:?}",
                            db_val.link_status
                        )
                    }),
                }?;
                (link_data, link_status)
            }
            _ => Err(report!(errors::ParsingError::UnknownError)).attach_printable_lazy(|| {
                format!(
                    "Invalid link_type for PaymentMethodCollectLink - {}",
                    db_val.link_type
                )
            })?,
        };

        Ok(Self {
            link_id: db_val.link_id,
            primary_reference: db_val.primary_reference,
            merchant_id: db_val.merchant_id,
            created_at: db_val.created_at,
            last_modified_at: db_val.last_modified_at,
            expiry: db_val.expiry,
            link_data,
            link_status,
            link_type: db_val.link_type,
            url: db_val.url,
            return_url: db_val.return_url,
        })
    }
}

impl TryFrom<GenericLink> for PayoutLink {
    type Error = Report<errors::ParsingError>;
    fn try_from(db_val: GenericLink) -> Result<Self, Self::Error> {
        let (link_data, link_status) = match db_val.link_type {
            common_enums::GenericLinkType::PayoutLink => {
                let link_data = db_val.link_data.parse_value("PayoutLinkData")?;
                let link_status = match db_val.link_status {
                    GenericLinkStatus::PayoutLink(status) => Ok(status),
                    _ => Err(report!(errors::ParsingError::EnumParseFailure(
                        "GenericLinkStatus"
                    )))
                    .attach_printable_lazy(|| {
                        format!("Invalid status for PayoutLink - {:?}", db_val.link_status)
                    }),
                }?;
                (link_data, link_status)
            }
            _ => Err(report!(errors::ParsingError::UnknownError)).attach_printable_lazy(|| {
                format!("Invalid link_type for PayoutLink - {}", db_val.link_type)
            })?,
        };

        Ok(Self {
            link_id: db_val.link_id,
            primary_reference: db_val.primary_reference,
            merchant_id: db_val.merchant_id,
            created_at: db_val.created_at,
            last_modified_at: db_val.last_modified_at,
            expiry: db_val.expiry,
            link_data,
            link_status,
            link_type: db_val.link_type,
            url: db_val.url,
            return_url: db_val.return_url,
        })
    }
}
