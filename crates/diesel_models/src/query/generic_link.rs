use std::str::FromStr;

use common_utils::{errors, ext_traits::ValueExt};
use diesel::{associations::HasTable, ExpressionMethods};
use error_stack::{report, Report, ResultExt};

use super::generics;
use crate::{
    errors as db_errors,
    generic_link::{GenericLink, GenericLinkData, GenericLinkNew, GenericLinkS},
    schema::generic_link::dsl,
    PgPooledConn, StorageResult,
};

impl GenericLinkNew {
    pub async fn insert(self, conn: &PgPooledConn) -> StorageResult<GenericLinkS> {
        let res: Result<GenericLink, Report<db_errors::DatabaseError>> =
            generics::generic_insert(conn, self).await;

        match res {
            Err(e) => Err(e),
            Ok(res) => GenericLinkS::try_from(res)
                .change_context(db_errors::DatabaseError::Others)
                .attach_printable("failed to parse generic link datafrom DB for id - {link_id}"),
        }
    }
}

impl GenericLink {
    pub async fn find_generic_link_by_link_id(
        conn: &PgPooledConn,
        link_id: &str,
    ) -> StorageResult<GenericLinkS> {
        let res: Result<GenericLink, Report<db_errors::DatabaseError>> =
            generics::generic_find_one::<<Self as HasTable>::Table, _, _>(
                conn,
                dsl::link_id.eq(link_id.to_owned()),
            )
            .await;

        match res {
            Err(e) => Err(e),
            Ok(res) => GenericLinkS::try_from(res)
                .change_context(db_errors::DatabaseError::Others)
                .attach_printable("failed to parse generic link datafrom DB for id - {link_id}"),
        }
    }
}

impl TryFrom<GenericLink> for GenericLinkS {
    type Error = Report<errors::ParsingError>;
    fn try_from(db_val: GenericLink) -> Result<Self, Self::Error> {
        let (link_data, link_status) = match db_val.link_type {
            common_enums::GenericLinkType::PaymentMethodCollect => {
                let link_data = db_val
                    .link_data
                    .parse_value("PaymentMethodCollectLinkData")?;
                let link_status =
                    common_enums::PaymentMethodCollectStatus::from_str(&db_val.link_status)
                        .map_err(|_| {
                            report!(errors::ParsingError::EnumParseFailure(
                                "PaymentMethodCollectStatus"
                            ))
                        })
                        .attach_printable(format!(
                            "Failed to parse link_status - {} for id - {}",
                            db_val.link_status, db_val.link_id
                        ))?;
                (
                    GenericLinkData::PaymentMethodCollect(link_data),
                    common_enums::GenericLinkStatus::PaymentMethodCollect(link_status),
                )
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
            link_status,
            link_type: db_val.link_type,
            url: db_val.url,
        })
    }
}
