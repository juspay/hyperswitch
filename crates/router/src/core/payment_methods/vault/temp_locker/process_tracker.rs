use error_stack::{report, ResultExt};
use scheduler::{types::process_data, utils as process_tracker_utils};

use crate::{
    core::errors::{self, RouterResult},
    db, logger,
    routes::{self, metrics},
    types::storage::{self, enums},
};

// ********************************************** PROCESS TRACKER **********************************************

pub async fn add_delete_tokenized_data_task(
    db: &dyn db::StorageInterface,
    lookup_key: &str,
    pm: enums::PaymentMethod,
    application_source: common_enums::ApplicationSource,
) -> RouterResult<()> {
    let runner = storage::ProcessTrackerRunner::DeleteTokenizeDataWorkflow;
    let process_tracker_id = format!("{runner}_{lookup_key}");
    let task = runner.to_string();
    let tag = ["BASILISK-V3"];
    let tracking_data = storage::TokenizeCoreWorkflow {
        lookup_key: lookup_key.to_owned(),
        pm,
    };
    let schedule_time = get_delete_tokenize_schedule_time(db, pm, 0)
        .await
        .ok_or(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to obtain initial process tracker schedule time")?;

    let process_tracker_entry = storage::ProcessTrackerNew::new(
        process_tracker_id,
        &task,
        runner,
        tag,
        tracking_data,
        None,
        schedule_time,
        common_types::consts::API_VERSION,
        application_source,
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable("Failed to construct delete tokenized data process tracker task")?;

    let response = db.insert_process(process_tracker_entry).await;
    response.map(|_| ()).or_else(|err| {
        if err.current_context().is_db_unique_violation() {
            Ok(())
        } else {
            Err(report!(errors::ApiErrorResponse::InternalServerError))
        }
    })
}

pub async fn start_tokenize_data_workflow(
    state: &routes::SessionState,
    tokenize_tracker: &storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let db = &*state.store;
    let delete_tokenize_data = serde_json::from_value::<storage::TokenizeCoreWorkflow>(
        tokenize_tracker.tracking_data.clone(),
    )
    .change_context(errors::ApiErrorResponse::InternalServerError)
    .attach_printable_lazy(|| {
        format!(
            "unable to convert into DeleteTokenizeByTokenRequest {:?}",
            tokenize_tracker.tracking_data
        )
    })?;

    match super::delete_tokenized_data(state, &delete_tokenize_data.lookup_key).await {
        Ok(()) => {
            logger::info!("Card From locker deleted Successfully");
            //mark task as finished
            db.as_scheduler()
                .finish_process_with_business_status(
                    tokenize_tracker.clone(),
                    diesel_models::process_tracker::business_status::COMPLETED_BY_PT,
                )
                .await?;
        }
        Err(err) => {
            logger::error!("Err: Deleting Card From Locker : {:?}", err);
            retry_delete_tokenize(db, delete_tokenize_data.pm, tokenize_tracker.to_owned()).await?;
            metrics::RETRIED_DELETE_DATA_COUNT.add(1, &[]);
        }
    }
    Ok(())
}

pub async fn get_delete_tokenize_schedule_time(
    db: &dyn db::StorageInterface,
    pm: enums::PaymentMethod,
    retry_count: i32,
) -> Option<time::PrimitiveDateTime> {
    let redis_mapping = db::get_and_deserialize_key(
        db,
        &format!("pt_mapping_delete_{pm}_tokenize_data"),
        "PaymentMethodsPTMapping",
    )
    .await;
    let mapping = match redis_mapping {
        Ok(x) => x,
        Err(error) => {
            logger::info!(?error, "Redis Mapping Error");
            process_data::PaymentMethodsPTMapping::default()
        }
    };
    let time_delta = process_tracker_utils::get_pm_schedule_time(mapping, pm, retry_count + 1);

    process_tracker_utils::get_time_from_delta(time_delta)
}

pub async fn retry_delete_tokenize(
    db: &dyn db::StorageInterface,
    pm: enums::PaymentMethod,
    pt: storage::ProcessTracker,
) -> Result<(), errors::ProcessTrackerError> {
    let schedule_time = get_delete_tokenize_schedule_time(db, pm, pt.retry_count).await;

    match schedule_time {
        Some(s_time) => {
            let retry_schedule = db
                .as_scheduler()
                .retry_process(pt, s_time)
                .await
                .map_err(Into::into);
            metrics::TASKS_RESET_COUNT.add(
                1,
                router_env::metric_attributes!(("flow", "DeleteTokenizeData")),
            );
            retry_schedule
        }
        None => db
            .as_scheduler()
            .finish_process_with_business_status(
                pt,
                diesel_models::process_tracker::business_status::RETRIES_EXCEEDED,
            )
            .await
            .map_err(Into::into),
    }
}

// Fallback logic of old temp locker needs to be removed later
