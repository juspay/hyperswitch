use std::{collections::HashMap, ops::Deref, str::FromStr, sync::Arc, time::Duration};

use api_models::enums;
use common_utils::{date_time, errors::CustomResult, events::ApiEventMetric, ext_traits::AsyncExt};
use currency_conversion::types::{CurrencyFactors, ExchangeRates};
use error_stack::ResultExt;
use masking::PeekInterface;
use once_cell::sync::Lazy;
use redis_interface::DelReply;
use rust_decimal::Decimal;
use strum::IntoEnumIterator;
use tokio::{sync::RwLock, time::sleep};

use crate::{
    logger,
    routes::app::settings::{Conversion, DefaultExchangeRates},
    services, AppState,
};
const REDIX_FOREX_CACHE_KEY: &str = "{forex_cache}_lock";
const REDIX_FOREX_CACHE_DATA: &str = "{forex_cache}_data";
const FOREX_API_TIMEOUT: u64 = 5;
const FOREX_BASE_URL: &str = "https://openexchangerates.org/api/latest.json?app_id=";
const FOREX_BASE_CURRENCY: &str = "&base=USD";
const FALLBACK_FOREX_BASE_URL: &str = "http://apilayer.net/api/live?access_key=";
const FALLBACK_FOREX_API_CURRENCY_PREFIX: &str = "USD";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FxExchangeRatesCacheEntry {
    data: Arc<ExchangeRates>,
    timestamp: i64,
}

static FX_EXCHANGE_RATES_CACHE: Lazy<RwLock<Option<FxExchangeRatesCacheEntry>>> =
    Lazy::new(|| RwLock::new(None));

impl ApiEventMetric for FxExchangeRatesCacheEntry {}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ForexCacheError {
    #[error("API error")]
    ApiError,
    #[error("API timeout")]
    ApiTimeout,
    #[error("API unresponsive")]
    ApiUnresponsive,
    #[error("Conversion error")]
    ConversionError,
    #[error("Could not acquire the lock for cache entry")]
    CouldNotAcquireLock,
    #[error("Provided currency not acceptable")]
    CurrencyNotAcceptable,
    #[error("Incorrect entries in default Currency response")]
    DefaultCurrencyParsingError,
    #[error("Entry not found in cache")]
    EntryNotFound,
    #[error("Expiration time invalid")]
    InvalidLogExpiry,
    #[error("Error reading local")]
    LocalReadError,
    #[error("Error writing to local cache")]
    LocalWriteError,
    #[error("Json Parsing error")]
    ParsingError,
    #[error("Aws Kms decryption error")]
    AwsKmsDecryptionFailed,
    #[error("Error connecting to redis")]
    RedisConnectionError,
    #[error("Not able to release write lock")]
    RedisLockReleaseFailed,
    #[error("Error writing to redis")]
    RedisWriteError,
    #[error("Not able to acquire write lock")]
    WriteLockNotAcquired,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ForexResponse {
    pub rates: HashMap<String, FloatDecimal>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct FallbackForexResponse {
    pub quotes: HashMap<String, FloatDecimal>,
}

#[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
struct FloatDecimal(#[serde(with = "rust_decimal::serde::float")] Decimal);

impl Deref for FloatDecimal {
    type Target = Decimal;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FxExchangeRatesCacheEntry {
    fn new(exchange_rate: ExchangeRates) -> Self {
        Self {
            data: Arc::new(exchange_rate),
            timestamp: date_time::now_unix_timestamp(),
        }
    }
    fn is_expired(&self, call_delay: i64) -> bool {
        self.timestamp + call_delay < date_time::now_unix_timestamp()
    }
}

async fn retrieve_forex_from_local() -> Option<FxExchangeRatesCacheEntry> {
    FX_EXCHANGE_RATES_CACHE.read().await.clone()
}

async fn save_forex_to_local(
    exchange_rates_cache_entry: FxExchangeRatesCacheEntry,
) -> CustomResult<(), ForexCacheError> {
    let mut local = FX_EXCHANGE_RATES_CACHE.write().await;
    *local = Some(exchange_rates_cache_entry);
    Ok(())
}

// Alternative handler for handling the case, When no data in local as well as redis
#[allow(dead_code)]
async fn waited_fetch_and_update_caches(
    state: &AppState,
    local_fetch_retry_delay: u64,
    local_fetch_retry_count: u64,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexCacheError> {
    for _n in 1..local_fetch_retry_count {
        sleep(Duration::from_millis(local_fetch_retry_delay)).await;
        //read from redis and update local plus break the loop and return
        match retrieve_forex_from_redis(state).await {
            Ok(Some(rates)) => {
                save_forex_to_local(rates.clone()).await?;
                return Ok(rates.clone());
            }
            Ok(None) => continue,
            Err(e) => {
                logger::error!(?e);
                continue;
            }
        }
    }
    //acquire lock one last time and try to fetch and update local & redis
    successive_fetch_and_save_forex(state, None).await
}

impl TryFrom<DefaultExchangeRates> for ExchangeRates {
    type Error = error_stack::Report<ForexCacheError>;
    fn try_from(value: DefaultExchangeRates) -> Result<Self, Self::Error> {
        let mut conversion_usable: HashMap<enums::Currency, CurrencyFactors> = HashMap::new();
        for (curr, conversion) in value.conversion {
            let enum_curr = enums::Currency::from_str(curr.as_str())
                .change_context(ForexCacheError::ConversionError)?;
            conversion_usable.insert(enum_curr, CurrencyFactors::from(conversion));
        }
        let base_curr = enums::Currency::from_str(value.base_currency.as_str())
            .change_context(ForexCacheError::ConversionError)?;
        Ok(Self {
            base_currency: base_curr,
            conversion: conversion_usable,
        })
    }
}

impl From<Conversion> for CurrencyFactors {
    fn from(value: Conversion) -> Self {
        Self {
            to_factor: value.to_factor,
            from_factor: value.from_factor,
        }
    }
}
pub async fn get_forex_rates(
    state: &AppState,
    call_delay: i64,
    local_fetch_retry_delay: u64,
    local_fetch_retry_count: u64,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexCacheError> {
    if let Some(local_rates) = retrieve_forex_from_local().await {
        if local_rates.is_expired(call_delay) {
            // expired local data
            handler_local_expired(state, call_delay, local_rates).await
        } else {
            // Valid data present in local
            Ok(local_rates)
        }
    } else {
        // No data in local
        handler_local_no_data(
            state,
            call_delay,
            local_fetch_retry_delay,
            local_fetch_retry_count,
        )
        .await
    }
}

async fn handler_local_no_data(
    state: &AppState,
    call_delay: i64,
    _local_fetch_retry_delay: u64,
    _local_fetch_retry_count: u64,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexCacheError> {
    match retrieve_forex_from_redis(state).await {
        Ok(Some(data)) => fallback_forex_redis_check(state, data, call_delay).await,
        Ok(None) => {
            // No data in local as well as redis
            Ok(successive_fetch_and_save_forex(state, None).await?)
        }
        Err(err) => {
            logger::error!(?err);
            Ok(successive_fetch_and_save_forex(state, None).await?)
        }
    }
}

async fn successive_fetch_and_save_forex(
    state: &AppState,
    stale_redis_data: Option<FxExchangeRatesCacheEntry>,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexCacheError> {
    match acquire_redis_lock(state).await {
        Ok(lock_acquired) => {
            if !lock_acquired {
                return stale_redis_data.ok_or(ForexCacheError::CouldNotAcquireLock.into());
            }
            let api_rates = fetch_forex_rates(state).await;
            match api_rates {
                Ok(rates) => successive_save_data_to_redis_local(state, rates).await,
                Err(err) => {
                    // API not able to fetch data call secondary service
                    logger::error!(?err);
                    let secondary_api_rates = fallback_fetch_forex_rates(state).await;
                    match secondary_api_rates {
                        Ok(rates) => Ok(successive_save_data_to_redis_local(state, rates).await?),
                        Err(err) => stale_redis_data.ok_or({
                            logger::error!(?err);
                            ForexCacheError::ApiUnresponsive.into()
                        }),
                    }
                }
            }
        }
        Err(e) => stale_redis_data.ok_or({
            logger::error!(?e);
            ForexCacheError::ApiUnresponsive.into()
        }),
    }
}

async fn successive_save_data_to_redis_local(
    state: &AppState,
    forex: FxExchangeRatesCacheEntry,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexCacheError> {
    Ok(save_forex_to_redis(state, &forex)
        .await
        .async_and_then(|_rates| async { release_redis_lock(state).await })
        .await
        .async_and_then(|_val| async { Ok(save_forex_to_local(forex.clone()).await) })
        .await
        .map_or_else(
            |e| {
                logger::error!(?e);
                forex.clone()
            },
            |_| forex.clone(),
        ))
}

async fn fallback_forex_redis_check(
    state: &AppState,
    redis_data: FxExchangeRatesCacheEntry,
    call_delay: i64,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexCacheError> {
    match is_redis_expired(Some(redis_data.clone()).as_ref(), call_delay).await {
        Some(redis_forex) => {
            // Valid data present in redis
            let exchange_rates = FxExchangeRatesCacheEntry::new(redis_forex.as_ref().clone());
            save_forex_to_local(exchange_rates.clone()).await?;
            Ok(exchange_rates)
        }
        None => {
            // redis expired
            successive_fetch_and_save_forex(state, Some(redis_data)).await
        }
    }
}

async fn handler_local_expired(
    state: &AppState,
    call_delay: i64,
    local_rates: FxExchangeRatesCacheEntry,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexCacheError> {
    match retrieve_forex_from_redis(state).await {
        Ok(redis_data) => {
            match is_redis_expired(redis_data.as_ref(), call_delay).await {
                Some(redis_forex) => {
                    // Valid data present in redis
                    let exchange_rates =
                        FxExchangeRatesCacheEntry::new(redis_forex.as_ref().clone());
                    save_forex_to_local(exchange_rates.clone()).await?;
                    Ok(exchange_rates)
                }
                None => {
                    // Redis is expired going for API request
                    successive_fetch_and_save_forex(state, Some(local_rates)).await
                }
            }
        }
        Err(e) => {
            //  data  not present in redis waited fetch
            logger::error!(?e);
            successive_fetch_and_save_forex(state, Some(local_rates)).await
        }
    }
}

async fn fetch_forex_rates(
    state: &AppState,
) -> Result<FxExchangeRatesCacheEntry, error_stack::Report<ForexCacheError>> {
    let forex_api_key = state.conf.forex_api.get_inner().api_key.peek();

    let forex_url: String = format!("{}{}{}", FOREX_BASE_URL, forex_api_key, FOREX_BASE_CURRENCY);
    let forex_request = services::RequestBuilder::new()
        .method(services::Method::Get)
        .url(&forex_url)
        .build();

    logger::info!(?forex_request);
    let response = state
        .api_client
        .send_request(
            &state.clone(),
            forex_request,
            Some(FOREX_API_TIMEOUT),
            false,
        )
        .await
        .change_context(ForexCacheError::ApiUnresponsive)?;
    let forex_response = response
        .json::<ForexResponse>()
        .await
        .change_context(ForexCacheError::ParsingError)?;

    logger::info!("{:?}", forex_response);

    let mut conversions: HashMap<enums::Currency, CurrencyFactors> = HashMap::new();
    for enum_curr in enums::Currency::iter() {
        match forex_response.rates.get(&enum_curr.to_string()) {
            Some(rate) => {
                let from_factor = match Decimal::new(1, 0).checked_div(**rate) {
                    Some(rate) => rate,
                    None => {
                        logger::error!("Rates for {} not received from API", &enum_curr);
                        continue;
                    }
                };
                let currency_factors = CurrencyFactors::new(**rate, from_factor);
                conversions.insert(enum_curr, currency_factors);
            }
            None => {
                logger::error!("Rates for {} not received from API", &enum_curr);
            }
        };
    }

    Ok(FxExchangeRatesCacheEntry::new(ExchangeRates::new(
        enums::Currency::USD,
        conversions,
    )))
}

pub async fn fallback_fetch_forex_rates(
    state: &AppState,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexCacheError> {
    let fallback_forex_api_key = state.conf.forex_api.get_inner().fallback_api_key.peek();

    let fallback_forex_url: String =
        format!("{}{}", FALLBACK_FOREX_BASE_URL, fallback_forex_api_key,);
    let fallback_forex_request = services::RequestBuilder::new()
        .method(services::Method::Get)
        .url(&fallback_forex_url)
        .build();

    logger::info!(?fallback_forex_request);
    let response = state
        .api_client
        .send_request(
            &state.clone(),
            fallback_forex_request,
            Some(FOREX_API_TIMEOUT),
            false,
        )
        .await
        .change_context(ForexCacheError::ApiUnresponsive)?;
    let fallback_forex_response = response
        .json::<FallbackForexResponse>()
        .await
        .change_context(ForexCacheError::ParsingError)?;

    logger::info!("{:?}", fallback_forex_response);
    let mut conversions: HashMap<enums::Currency, CurrencyFactors> = HashMap::new();
    for enum_curr in enums::Currency::iter() {
        match fallback_forex_response.quotes.get(
            format!(
                "{}{}",
                FALLBACK_FOREX_API_CURRENCY_PREFIX,
                &enum_curr.to_string()
            )
            .as_str(),
        ) {
            Some(rate) => {
                let from_factor = match Decimal::new(1, 0).checked_div(**rate) {
                    Some(rate) => rate,
                    None => {
                        logger::error!("Rates for {} not received from API", &enum_curr);
                        continue;
                    }
                };
                let currency_factors = CurrencyFactors::new(**rate, from_factor);
                conversions.insert(enum_curr, currency_factors);
            }
            None => {
                logger::error!("Rates for {} not received from API", &enum_curr);
            }
        };
    }

    let rates =
        FxExchangeRatesCacheEntry::new(ExchangeRates::new(enums::Currency::USD, conversions));
    match acquire_redis_lock(state).await {
        Ok(_) => Ok(successive_save_data_to_redis_local(state, rates).await?),
        Err(e) => {
            logger::error!(?e);
            Ok(rates)
        }
    }
}

async fn release_redis_lock(
    state: &AppState,
) -> Result<DelReply, error_stack::Report<ForexCacheError>> {
    state
        .store
        .get_redis_conn()
        .change_context(ForexCacheError::RedisConnectionError)?
        .delete_key(REDIX_FOREX_CACHE_KEY)
        .await
        .change_context(ForexCacheError::RedisLockReleaseFailed)
}

async fn acquire_redis_lock(app_state: &AppState) -> CustomResult<bool, ForexCacheError> {
    let forex_api = app_state.conf.forex_api.get_inner();
    app_state
        .store
        .get_redis_conn()
        .change_context(ForexCacheError::RedisConnectionError)?
        .set_key_if_not_exists_with_expiry(
            REDIX_FOREX_CACHE_KEY,
            "",
            Some(
                i64::try_from(
                    forex_api.local_fetch_retry_count * forex_api.local_fetch_retry_delay
                        + forex_api.api_timeout,
                )
                .change_context(ForexCacheError::ConversionError)?,
            ),
        )
        .await
        .map(|val| matches!(val, redis_interface::SetnxReply::KeySet))
        .change_context(ForexCacheError::CouldNotAcquireLock)
}

async fn save_forex_to_redis(
    app_state: &AppState,
    forex_exchange_cache_entry: &FxExchangeRatesCacheEntry,
) -> CustomResult<(), ForexCacheError> {
    app_state
        .store
        .get_redis_conn()
        .change_context(ForexCacheError::RedisConnectionError)?
        .serialize_and_set_key(REDIX_FOREX_CACHE_DATA, forex_exchange_cache_entry)
        .await
        .change_context(ForexCacheError::RedisWriteError)
}

async fn retrieve_forex_from_redis(
    app_state: &AppState,
) -> CustomResult<Option<FxExchangeRatesCacheEntry>, ForexCacheError> {
    app_state
        .store
        .get_redis_conn()
        .change_context(ForexCacheError::RedisConnectionError)?
        .get_and_deserialize_key(REDIX_FOREX_CACHE_DATA, "FxExchangeRatesCache")
        .await
        .change_context(ForexCacheError::EntryNotFound)
}

async fn is_redis_expired(
    redis_cache: Option<&FxExchangeRatesCacheEntry>,
    call_delay: i64,
) -> Option<Arc<ExchangeRates>> {
    redis_cache.and_then(|cache| {
        if cache.timestamp + call_delay > date_time::now_unix_timestamp() {
            Some(cache.data.clone())
        } else {
            None
        }
    })
}

pub async fn convert_currency(
    state: AppState,
    amount: i64,
    to_currency: String,
    from_currency: String,
) -> CustomResult<api_models::currency::CurrencyConversionResponse, ForexCacheError> {
    let forex_api = state.conf.forex_api.get_inner();
    let rates = get_forex_rates(
        &state,
        forex_api.call_delay,
        forex_api.local_fetch_retry_delay,
        forex_api.local_fetch_retry_count,
    )
    .await
    .change_context(ForexCacheError::ApiError)?;

    let to_currency = enums::Currency::from_str(to_currency.as_str())
        .change_context(ForexCacheError::CurrencyNotAcceptable)?;

    let from_currency = enums::Currency::from_str(from_currency.as_str())
        .change_context(ForexCacheError::CurrencyNotAcceptable)?;

    let converted_amount =
        currency_conversion::conversion::convert(&rates.data, from_currency, to_currency, amount)
            .change_context(ForexCacheError::ConversionError)?;

    Ok(api_models::currency::CurrencyConversionResponse {
        converted_amount: converted_amount.to_string(),
        currency: to_currency.to_string(),
    })
}
