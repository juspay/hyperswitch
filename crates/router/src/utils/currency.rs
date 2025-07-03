use std::{
    collections::HashMap,
    ops::Deref,
    str::FromStr,
    sync::{Arc, LazyLock},
};

use api_models::enums;
use common_utils::{date_time, errors::CustomResult, events::ApiEventMetric, ext_traits::AsyncExt};
use currency_conversion::types::{CurrencyFactors, ExchangeRates};
use error_stack::ResultExt;
use masking::PeekInterface;
use redis_interface::DelReply;
use router_env::{instrument, tracing};
use rust_decimal::Decimal;
use strum::IntoEnumIterator;
use tokio::sync::RwLock;
use tracing_futures::Instrument;

use crate::{
    logger,
    routes::app::settings::{Conversion, DefaultExchangeRates},
    services, SessionState,
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
    pub data: Arc<ExchangeRates>,
    timestamp: i64,
}

static FX_EXCHANGE_RATES_CACHE: LazyLock<RwLock<Option<FxExchangeRatesCacheEntry>>> =
    LazyLock::new(|| RwLock::new(None));

impl ApiEventMetric for FxExchangeRatesCacheEntry {}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ForexError {
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
    #[error("Forex configuration error: {0}")]
    ConfigurationError(String),
    #[error("Incorrect entries in default Currency response")]
    DefaultCurrencyParsingError,
    #[error("Entry not found in cache")]
    EntryNotFound,
    #[error("Forex data unavailable")]
    ForexDataUnavailable,
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
    fn is_expired(&self, data_expiration_delay: u32) -> bool {
        self.timestamp + i64::from(data_expiration_delay) < date_time::now_unix_timestamp()
    }
}

async fn retrieve_forex_from_local_cache() -> Option<FxExchangeRatesCacheEntry> {
    FX_EXCHANGE_RATES_CACHE.read().await.clone()
}

async fn save_forex_data_to_local_cache(
    exchange_rates_cache_entry: FxExchangeRatesCacheEntry,
) -> CustomResult<(), ForexError> {
    let mut local = FX_EXCHANGE_RATES_CACHE.write().await;
    *local = Some(exchange_rates_cache_entry);
    logger::debug!("forex_log: forex saved in cache");
    Ok(())
}

impl TryFrom<DefaultExchangeRates> for ExchangeRates {
    type Error = error_stack::Report<ForexError>;
    fn try_from(value: DefaultExchangeRates) -> Result<Self, Self::Error> {
        let mut conversion_usable: HashMap<enums::Currency, CurrencyFactors> = HashMap::new();
        for (curr, conversion) in value.conversion {
            let enum_curr = enums::Currency::from_str(curr.as_str())
                .change_context(ForexError::ConversionError)
                .attach_printable("Unable to Convert currency received")?;
            conversion_usable.insert(enum_curr, CurrencyFactors::from(conversion));
        }
        let base_curr = enums::Currency::from_str(value.base_currency.as_str())
            .change_context(ForexError::ConversionError)
            .attach_printable("Unable to convert base currency")?;
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

#[instrument(skip_all)]
pub async fn get_forex_rates(
    state: &SessionState,
    data_expiration_delay: u32,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexError> {
    if let Some(local_rates) = retrieve_forex_from_local_cache().await {
        if local_rates.is_expired(data_expiration_delay) {
            // expired local data
            logger::debug!("forex_log: Forex stored in cache is expired");
            call_forex_api_and_save_data_to_cache_and_redis(state, Some(local_rates)).await
        } else {
            // Valid data present in local
            logger::debug!("forex_log: forex found in cache");
            Ok(local_rates)
        }
    } else {
        // No data in local
        call_api_if_redis_forex_data_expired(state, data_expiration_delay).await
    }
}

async fn call_api_if_redis_forex_data_expired(
    state: &SessionState,
    data_expiration_delay: u32,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexError> {
    match retrieve_forex_data_from_redis(state).await {
        Ok(Some(data)) => {
            call_forex_api_if_redis_data_expired(state, data, data_expiration_delay).await
        }
        Ok(None) => {
            // No data in local as well as redis
            call_forex_api_and_save_data_to_cache_and_redis(state, None).await?;
            Err(ForexError::ForexDataUnavailable.into())
        }
        Err(error) => {
            // Error in deriving forex rates from redis
            logger::error!("forex_error: {:?}", error);
            call_forex_api_and_save_data_to_cache_and_redis(state, None).await?;
            Err(ForexError::ForexDataUnavailable.into())
        }
    }
}

async fn call_forex_api_and_save_data_to_cache_and_redis(
    state: &SessionState,
    stale_redis_data: Option<FxExchangeRatesCacheEntry>,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexError> {
    // spawn a new thread and do the api fetch and write operations on redis.
    let forex_api_key = state.conf.forex_api.get_inner().api_key.peek();
    if forex_api_key.is_empty() {
        Err(ForexError::ConfigurationError("api_keys not provided".into()).into())
    } else {
        let state = state.clone();
        tokio::spawn(
            async move {
                acquire_redis_lock_and_call_forex_api(&state)
                    .await
                    .map_err(|err| {
                        logger::error!(forex_error=?err);
                    })
                    .ok();
            }
            .in_current_span(),
        );
        stale_redis_data.ok_or(ForexError::EntryNotFound.into())
    }
}

async fn acquire_redis_lock_and_call_forex_api(
    state: &SessionState,
) -> CustomResult<(), ForexError> {
    let lock_acquired = acquire_redis_lock(state).await?;
    if !lock_acquired {
        Err(ForexError::CouldNotAcquireLock.into())
    } else {
        logger::debug!("forex_log: redis lock acquired");
        let api_rates = fetch_forex_rates_from_primary_api(state).await;
        match api_rates {
            Ok(rates) => save_forex_data_to_cache_and_redis(state, rates).await,
            Err(error) => {
                logger::error!(forex_error=?error,"primary_forex_error");
                // API not able to fetch data call secondary service
                let secondary_api_rates = fetch_forex_rates_from_fallback_api(state).await;
                match secondary_api_rates {
                    Ok(rates) => save_forex_data_to_cache_and_redis(state, rates).await,
                    Err(error) => {
                        release_redis_lock(state).await?;
                        Err(error)
                    }
                }
            }
        }
    }
}

async fn save_forex_data_to_cache_and_redis(
    state: &SessionState,
    forex: FxExchangeRatesCacheEntry,
) -> CustomResult<(), ForexError> {
    save_forex_data_to_redis(state, &forex)
        .await
        .async_and_then(|_rates| release_redis_lock(state))
        .await
        .async_and_then(|_val| save_forex_data_to_local_cache(forex.clone()))
        .await
}

async fn call_forex_api_if_redis_data_expired(
    state: &SessionState,
    redis_data: FxExchangeRatesCacheEntry,
    data_expiration_delay: u32,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexError> {
    match is_redis_expired(Some(redis_data.clone()).as_ref(), data_expiration_delay).await {
        Some(redis_forex) => {
            // Valid data present in redis
            let exchange_rates = FxExchangeRatesCacheEntry::new(redis_forex.as_ref().clone());
            logger::debug!("forex_log: forex response found in redis");
            save_forex_data_to_local_cache(exchange_rates.clone()).await?;
            Ok(exchange_rates)
        }
        None => {
            // redis expired
            call_forex_api_and_save_data_to_cache_and_redis(state, Some(redis_data)).await
        }
    }
}

async fn fetch_forex_rates_from_primary_api(
    state: &SessionState,
) -> Result<FxExchangeRatesCacheEntry, error_stack::Report<ForexError>> {
    let forex_api_key = state.conf.forex_api.get_inner().api_key.peek();

    logger::debug!("forex_log: Primary api call for forex fetch");
    let forex_url: String = format!("{FOREX_BASE_URL}{forex_api_key}{FOREX_BASE_CURRENCY}");
    let forex_request = services::RequestBuilder::new()
        .method(services::Method::Get)
        .url(&forex_url)
        .build();

    logger::info!(primary_forex_request=?forex_request,"forex_log: Primary api call for forex fetch");
    let response = state
        .api_client
        .send_request(
            &state.clone(),
            forex_request,
            Some(FOREX_API_TIMEOUT),
            false,
        )
        .await
        .change_context(ForexError::ApiUnresponsive)
        .attach_printable("Primary forex fetch api unresponsive")?;
    let forex_response = response
        .json::<ForexResponse>()
        .await
        .change_context(ForexError::ParsingError)
        .attach_printable(
            "Unable to parse response received from primary api into ForexResponse",
        )?;

    logger::info!(primary_forex_response=?forex_response,"forex_log");

    let mut conversions: HashMap<enums::Currency, CurrencyFactors> = HashMap::new();
    for enum_curr in enums::Currency::iter() {
        match forex_response.rates.get(&enum_curr.to_string()) {
            Some(rate) => {
                let from_factor = match Decimal::new(1, 0).checked_div(**rate) {
                    Some(rate) => rate,
                    None => {
                        logger::error!(
                            "forex_error: Rates for {} not received from API",
                            &enum_curr
                        );
                        continue;
                    }
                };
                let currency_factors = CurrencyFactors::new(**rate, from_factor);
                conversions.insert(enum_curr, currency_factors);
            }
            None => {
                logger::error!(
                    "forex_error: Rates for {} not received from API",
                    &enum_curr
                );
            }
        };
    }

    Ok(FxExchangeRatesCacheEntry::new(ExchangeRates::new(
        enums::Currency::USD,
        conversions,
    )))
}

pub async fn fetch_forex_rates_from_fallback_api(
    state: &SessionState,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexError> {
    let fallback_forex_api_key = state.conf.forex_api.get_inner().fallback_api_key.peek();

    let fallback_forex_url: String = format!("{FALLBACK_FOREX_BASE_URL}{fallback_forex_api_key}");
    let fallback_forex_request = services::RequestBuilder::new()
        .method(services::Method::Get)
        .url(&fallback_forex_url)
        .build();

    logger::info!(fallback_forex_request=?fallback_forex_request,"forex_log: Fallback api call for forex fetch");
    let response = state
        .api_client
        .send_request(
            &state.clone(),
            fallback_forex_request,
            Some(FOREX_API_TIMEOUT),
            false,
        )
        .await
        .change_context(ForexError::ApiUnresponsive)
        .attach_printable("Fallback forex fetch api unresponsive")?;

    let fallback_forex_response = response
        .json::<FallbackForexResponse>()
        .await
        .change_context(ForexError::ParsingError)
        .attach_printable(
            "Unable to parse response received from fallback api into ForexResponse",
        )?;

    logger::info!(fallback_forex_response=?fallback_forex_response,"forex_log");

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
                        logger::error!(
                            "forex_error: Rates for {} not received from API",
                            &enum_curr
                        );
                        continue;
                    }
                };
                let currency_factors = CurrencyFactors::new(**rate, from_factor);
                conversions.insert(enum_curr, currency_factors);
            }
            None => {
                if enum_curr == enums::Currency::USD {
                    let currency_factors =
                        CurrencyFactors::new(Decimal::new(1, 0), Decimal::new(1, 0));
                    conversions.insert(enum_curr, currency_factors);
                } else {
                    logger::error!(
                        "forex_error: Rates for {} not received from API",
                        &enum_curr
                    );
                }
            }
        };
    }

    let rates =
        FxExchangeRatesCacheEntry::new(ExchangeRates::new(enums::Currency::USD, conversions));
    match acquire_redis_lock(state).await {
        Ok(_) => {
            save_forex_data_to_cache_and_redis(state, rates.clone()).await?;
            Ok(rates)
        }
        Err(e) => Err(e),
    }
}

async fn release_redis_lock(
    state: &SessionState,
) -> Result<DelReply, error_stack::Report<ForexError>> {
    logger::debug!("forex_log: Releasing redis lock");
    state
        .store
        .get_redis_conn()
        .change_context(ForexError::RedisConnectionError)?
        .delete_key(&REDIX_FOREX_CACHE_KEY.into())
        .await
        .change_context(ForexError::RedisLockReleaseFailed)
        .attach_printable("Unable to release redis lock")
}

async fn acquire_redis_lock(state: &SessionState) -> CustomResult<bool, ForexError> {
    let forex_api = state.conf.forex_api.get_inner();
    logger::debug!("forex_log: Acquiring redis lock");
    state
        .store
        .get_redis_conn()
        .change_context(ForexError::RedisConnectionError)?
        .set_key_if_not_exists_with_expiry(
            &REDIX_FOREX_CACHE_KEY.into(),
            "",
            Some(i64::from(forex_api.redis_lock_timeout_in_seconds)),
        )
        .await
        .map(|val| matches!(val, redis_interface::SetnxReply::KeySet))
        .change_context(ForexError::CouldNotAcquireLock)
        .attach_printable("Unable to acquire redis lock")
}

async fn save_forex_data_to_redis(
    app_state: &SessionState,
    forex_exchange_cache_entry: &FxExchangeRatesCacheEntry,
) -> CustomResult<(), ForexError> {
    let forex_api = app_state.conf.forex_api.get_inner();
    logger::debug!("forex_log: Saving forex to redis");
    app_state
        .store
        .get_redis_conn()
        .change_context(ForexError::RedisConnectionError)?
        .serialize_and_set_key_with_expiry(
            &REDIX_FOREX_CACHE_DATA.into(),
            forex_exchange_cache_entry,
            i64::from(forex_api.redis_ttl_in_seconds),
        )
        .await
        .change_context(ForexError::RedisWriteError)
        .attach_printable("Unable to save forex data to redis")
}

async fn retrieve_forex_data_from_redis(
    app_state: &SessionState,
) -> CustomResult<Option<FxExchangeRatesCacheEntry>, ForexError> {
    logger::debug!("forex_log: Retrieving forex from redis");
    app_state
        .store
        .get_redis_conn()
        .change_context(ForexError::RedisConnectionError)?
        .get_and_deserialize_key(&REDIX_FOREX_CACHE_DATA.into(), "FxExchangeRatesCache")
        .await
        .change_context(ForexError::EntryNotFound)
        .attach_printable("Forex entry not found in redis")
}

async fn is_redis_expired(
    redis_cache: Option<&FxExchangeRatesCacheEntry>,
    data_expiration_delay: u32,
) -> Option<Arc<ExchangeRates>> {
    redis_cache.and_then(|cache| {
        if cache.timestamp + i64::from(data_expiration_delay) > date_time::now_unix_timestamp() {
            Some(cache.data.clone())
        } else {
            logger::debug!("forex_log: Forex stored in redis is expired");
            None
        }
    })
}

#[instrument(skip_all)]
pub async fn convert_currency(
    state: SessionState,
    amount: i64,
    to_currency: String,
    from_currency: String,
) -> CustomResult<api_models::currency::CurrencyConversionResponse, ForexError> {
    let forex_api = state.conf.forex_api.get_inner();
    let rates = get_forex_rates(&state, forex_api.data_expiration_delay_in_seconds)
        .await
        .change_context(ForexError::ApiError)?;

    let to_currency = enums::Currency::from_str(to_currency.as_str())
        .change_context(ForexError::CurrencyNotAcceptable)
        .attach_printable("The provided currency is not acceptable")?;

    let from_currency = enums::Currency::from_str(from_currency.as_str())
        .change_context(ForexError::CurrencyNotAcceptable)
        .attach_printable("The provided currency is not acceptable")?;

    let converted_amount =
        currency_conversion::conversion::convert(&rates.data, from_currency, to_currency, amount)
            .change_context(ForexError::ConversionError)
            .attach_printable("Unable to perform currency conversion")?;

    Ok(api_models::currency::CurrencyConversionResponse {
        converted_amount: converted_amount.to_string(),
        currency: to_currency.to_string(),
    })
}
