use std::{collections::HashMap, ops::Deref, str::FromStr, sync::Arc, time::Duration};

use api_models::enums;
use common_utils::{date_time, errors::CustomResult, events::ApiEventMetric, ext_traits::AsyncExt};
use currency_conversion::types::{CurrencyFactors, ExchangeRates};
use error_stack::{IntoReport, ResultExt};
#[cfg(feature = "hashicorp-vault")]
use external_services::hashicorp_vault::{self, decrypt::VaultFetch};
#[cfg(feature = "kms")]
use external_services::kms;
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
    #[error("Kms decryption error")]
    KmsDecryptionFailed,
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
        /// This method returns a reference to the target type of the Deref trait.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FxExchangeRatesCacheEntry {
        /// Creates a new instance of ExchangeRatesData with the given exchange rate and current timestamp.
    fn new(exchange_rate: ExchangeRates) -> Self {
        Self {
            data: Arc::new(exchange_rate),
            timestamp: date_time::now_unix_timestamp(),
        }
    }
        /// Checks if the timestamp of the object, plus the specified call delay in seconds,
    /// is less than the current Unix timestamp, indicating that the object has expired.
    fn is_expired(&self, call_delay: i64) -> bool {
        self.timestamp + call_delay < date_time::now_unix_timestamp()
    }
}

/// Asynchronously retrieves the forex exchange rates from the local cache. 
/// 
/// # Returns
/// 
/// An `Option` containing the `FxExchangeRatesCacheEntry` if available in the cache,
/// or `None` if the cache is empty.
async fn retrieve_forex_from_local() -> Option<FxExchangeRatesCacheEntry> {
    FX_EXCHANGE_RATES_CACHE.read().await.clone()
}

/// Asynchronously saves the given `FxExchangeRatesCacheEntry` to the local cache of exchange rates.
/// 
/// # Arguments
/// * `exchange_rates_cache_entry` - The entry to be saved to the local cache.
///
/// # Returns
/// * `CustomResult<(), ForexCacheError>` - A result indicating success or an error if the operation fails.
///
async fn save_forex_to_local(
    exchange_rates_cache_entry: FxExchangeRatesCacheEntry,
) -> CustomResult<(), ForexCacheError> {
    let mut local = FX_EXCHANGE_RATES_CACHE.write().await;
    *local = Some(exchange_rates_cache_entry);
    Ok(())
}


// Alternative handler for handling the case, When no data in local as well as redis
#[allow(dead_code)]
/// Asynchronously fetches forex rates from a local cache with retries and updates the local cache if necessary. If the local cache is not available, it attempts to retrieve the data from Redis. If successful, it saves the data to the local cache and returns the forex rates. If the local fetch fails, it retries based on the provided delay and retry count. If all retries fail, it attempts to fetch the forex rates from an external service (e.g., KMS or HashiCorp Vault) and saves the data to the local cache and Redis. Returns a result indicating success or failure along with the forex rates or an error.
async fn waited_fetch_and_update_caches(
    state: &AppState,
    local_fetch_retry_delay: u64,
    local_fetch_retry_count: u64,
    #[cfg(feature = "kms")] kms_config: &kms::KmsConfig,
    #[cfg(feature = "hashicorp-vault")]
    hc_config: &external_services::hashicorp_vault::HashiCorpVaultConfig,
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
    successive_fetch_and_save_forex(
        state,
        None,
        #[cfg(feature = "kms")]
        kms_config,
        #[cfg(feature = "hashicorp-vault")]
        hc_config,
    )
    .await
}

impl TryFrom<DefaultExchangeRates> for ExchangeRates {
    type Error = error_stack::Report<ForexCacheError>;
        /// Attempts to create a new instance of the struct from the provided DefaultExchangeRates instance.
    /// 
    /// # Arguments
    /// 
    /// * `value` - The DefaultExchangeRates instance to create the new instance from.
    /// 
    /// # Returns
    /// 
    /// * `Result<Self, Self::Error>` - A Result containing the new instance if successful, or an error if the conversion fails.
    /// 
    /// # Errors
    /// 
    /// An error will be returned if the conversion from DefaultExchangeRates to the new instance fails.
    fn try_from(value: DefaultExchangeRates) -> Result<Self, Self::Error> {
        let mut conversion_usable: HashMap<enums::Currency, CurrencyFactors> = HashMap::new();
        for (curr, conversion) in value.conversion {
            let enum_curr = enums::Currency::from_str(curr.as_str())
                .into_report()
                .change_context(ForexCacheError::ConversionError)?;
            conversion_usable.insert(enum_curr, CurrencyFactors::from(conversion));
        }
        let base_curr = enums::Currency::from_str(value.base_currency.as_str())
            .into_report()
            .change_context(ForexCacheError::ConversionError)?;
        Ok(Self {
            base_currency: base_curr,
            conversion: conversion_usable,
        })
    }
}

impl From<Conversion> for CurrencyFactors {
        /// Creates a new instance of `Self` from a `Conversion` value.
    fn from(value: Conversion) -> Self {
        Self {
            to_factor: value.to_factor,
            from_factor: value.from_factor,
        }
    }
}
/// Asynchronously retrieves forex exchange rates either from local cache or external services,
/// and handles expired or missing data by calling appropriate handlers.
pub async fn get_forex_rates(
    state: &AppState,
    call_delay: i64,
    local_fetch_retry_delay: u64,
    local_fetch_retry_count: u64,
    #[cfg(feature = "kms")] kms_config: &kms::KmsConfig,
    #[cfg(feature = "hashicorp-vault")]
    hc_config: &external_services::hashicorp_vault::HashiCorpVaultConfig,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexCacheError> {
    if let Some(local_rates) = retrieve_forex_from_local().await {
        if local_rates.is_expired(call_delay) {
            // expired local data
            handler_local_expired(
                state,
                call_delay,
                local_rates,
                #[cfg(feature = "kms")]
                kms_config,
                #[cfg(feature = "hashicorp-vault")]
                hc_config,
            )
            .await
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
            #[cfg(feature = "kms")]
            kms_config,
            #[cfg(feature = "hashicorp-vault")]
            hc_config,
        )
        .await
    }
}

/// Handles the retrieval of forex exchange rates from the local cache, fallback to redis, and successive fetch and save if data is not available in either place. 
async fn handler_local_no_data(
    state: &AppState,
    call_delay: i64,
    _local_fetch_retry_delay: u64,
    _local_fetch_retry_count: u64,
    #[cfg(feature = "kms")] kms_config: &kms::KmsConfig,
    #[cfg(feature = "hashicorp-vault")]
    hc_config: &external_services::hashicorp_vault::HashiCorpVaultConfig,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexCacheError> {
    match retrieve_forex_from_redis(state).await {
        Ok(Some(data)) => {
            fallback_forex_redis_check(
                state,
                data,
                call_delay,
                #[cfg(feature = "kms")]
                kms_config,
                #[cfg(feature = "hashicorp-vault")]
                hc_config,
            )
            .await
        }
        Ok(None) => {
            // No data in local as well as redis
            Ok(successive_fetch_and_save_forex(
                state,
                None,
                #[cfg(feature = "kms")]
                kms_config,
                #[cfg(feature = "hashicorp-vault")]
                hc_config,
            )
            .await?)
        }
        Err(err) => {
            logger::error!(?err);
            Ok(successive_fetch_and_save_forex(
                state,
                None,
                #[cfg(feature = "kms")]
                kms_config,
                #[cfg(feature = "hashicorp-vault")]
                hc_config,
            )
            .await?)
        }
    }
}

/// Asynchronously fetches forex rates from an API, saves the data to a local cache, and handles fallback to a secondary service if the primary API is unresponsive. It also acquires a lock in the Redis cache to ensure data consistency.
///
/// # Arguments
///
/// * `state` - The application state
/// * `stale_redis_data` - Optional stale data from the Redis cache
/// * `kms_config` - Configuration for Key Management Service, required if the "kms" feature is enabled
/// * `hc_config` - Configuration for HashiCorp Vault, required if the "hashicorp-vault" feature is enabled
///
/// # Returns
///
/// A `CustomResult` containing the fetched and saved forex exchange rates data, or a `ForexCacheError` if an error occurs.
///
async fn successive_fetch_and_save_forex(
    state: &AppState,
    stale_redis_data: Option<FxExchangeRatesCacheEntry>,
    #[cfg(feature = "kms")] kms_config: &kms::KmsConfig,
    #[cfg(feature = "hashicorp-vault")]
    hc_config: &external_services::hashicorp_vault::HashiCorpVaultConfig,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexCacheError> {
    match acquire_redis_lock(state).await {
        Ok(lock_acquired) => {
            if !lock_acquired {
                return stale_redis_data.ok_or(ForexCacheError::CouldNotAcquireLock.into());
            }
            let api_rates = fetch_forex_rates(
                state,
                #[cfg(feature = "kms")]
                kms_config,
                #[cfg(feature = "hashicorp-vault")]
                hc_config,
            )
            .await;
            match api_rates {
                Ok(rates) => successive_save_data_to_redis_local(state, rates).await,
                Err(err) => {
                    // API not able to fetch data call secondary service
                    logger::error!(?err);
                    let secondary_api_rates = fallback_fetch_forex_rates(
                        state,
                        #[cfg(feature = "kms")]
                        kms_config,
                        #[cfg(feature = "hashicorp-vault")]
                        hc_config,
                    )
                    .await;
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

/// Asynchronously saves forex exchange rates data to Redis and local cache, and releases a Redis lock if it was acquired.
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

/// Check if valid forex data is present in the Redis cache. If valid data is present, retrieve it and save it to the local cache. If the Redis cache is expired, fetch the forex data from external services and save it to the local cache.
async fn fallback_forex_redis_check(
    state: &AppState,
    redis_data: FxExchangeRatesCacheEntry,
    call_delay: i64,
    #[cfg(feature = "kms")] kms_config: &kms::KmsConfig,
    #[cfg(feature = "hashicorp-vault")]
    hc_config: &external_services::hashicorp_vault::HashiCorpVaultConfig,
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
            successive_fetch_and_save_forex(
                state,
                Some(redis_data),
                #[cfg(feature = "kms")]
                kms_config,
                #[cfg(feature = "hashicorp-vault")]
                hc_config,
            )
            .await
        }
    }
}

/// Handles the case where the local forex data has expired. It retrieves the forex data from Redis, checks if it is expired, and then either returns the valid data from Redis or makes an API request to fetch new data if the Redis data is expired. It also logs any errors encountered during the process. If the Redis data is valid, it saves the data to the local cache and returns it. If the Redis data is expired or not present, it makes a successive API request to fetch and save the forex data.
async fn handler_local_expired(
    state: &AppState,
    call_delay: i64,
    local_rates: FxExchangeRatesCacheEntry,
    #[cfg(feature = "kms")] kms_config: &kms::KmsConfig,
    #[cfg(feature = "hashicorp-vault")]
    hc_config: &external_services::hashicorp_vault::HashiCorpVaultConfig,
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
                    successive_fetch_and_save_forex(
                        state,
                        Some(local_rates),
                        #[cfg(feature = "kms")]
                        kms_config,
                        #[cfg(feature = "hashicorp-vault")]
                        hc_config,
                    )
                    .await
                }
            }
        }
        Err(e) => {
            //  data  not present in redis waited fetch
            logger::error!(?e);
            successive_fetch_and_save_forex(
                state,
                Some(local_rates),
                #[cfg(feature = "kms")]
                kms_config,
                #[cfg(feature = "hashicorp-vault")]
                hc_config,
            )
            .await
        }
    }
}

/// Asynchronously fetches the forex exchange rates using the provided state and external service configurations (if enabled). It first retrieves the forex API key, then constructs the forex API URL and makes a request to the API using the provided API client. It then parses the API response and constructs a cache entry containing the exchange rates for various currencies relative to the base currency (USD). Returns a Result containing the FxExchangeRatesCacheEntry or an error report if any of the operations fail.
async fn fetch_forex_rates(
    state: &AppState,
    #[cfg(feature = "kms")] kms_config: &kms::KmsConfig,

    #[cfg(feature = "hashicorp-vault")]
    hc_config: &external_services::hashicorp_vault::HashiCorpVaultConfig,
) -> Result<FxExchangeRatesCacheEntry, error_stack::Report<ForexCacheError>> {
    let forex_api_key = async {
        #[cfg(feature = "hashicorp-vault")]
        let client = hashicorp_vault::get_hashicorp_client(hc_config)
            .await
            .change_context(ForexCacheError::KmsDecryptionFailed)?;

        #[cfg(not(feature = "hashicorp-vault"))]
        let output = state.conf.forex_api.api_key.clone();
        #[cfg(feature = "hashicorp-vault")]
        let output = state
            .conf
            .forex_api
            .api_key
            .clone()
            .fetch_inner::<hashicorp_vault::Kv2>(client)
            .await
            .change_context(ForexCacheError::KmsDecryptionFailed)?;

        Ok::<_, error_stack::Report<ForexCacheError>>(output)
    }
    .await?;
    #[cfg(feature = "kms")]
    let forex_api_key = kms::get_kms_client(kms_config)
        .await
        .decrypt(forex_api_key.peek())
        .await
        .change_context(ForexCacheError::KmsDecryptionFailed)?;

    #[cfg(not(feature = "kms"))]
    let forex_api_key = forex_api_key.peek();

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
        .into_report()
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

/// Asynchronously fetches forex exchange rates using fallback APIs and stores the data in a cache entry. If the "kms" feature is enabled, the method uses a KMS client to decrypt the fallback API key. If the "hashicorp-vault" feature is enabled, it fetches the client from HashiCorp Vault and uses it to fetch the fallback API key. The method then constructs the forex URL, sends a request to the API, parses the response, and saves the exchange rates to a cache entry in Redis. If an error occurs during any of these steps, the method returns a custom result containing the specific error.
pub async fn fallback_fetch_forex_rates(
    state: &AppState,
    #[cfg(feature = "kms")] kms_config: &kms::KmsConfig,
    #[cfg(feature = "hashicorp-vault")]
    hc_config: &external_services::hashicorp_vault::HashiCorpVaultConfig,
) -> CustomResult<FxExchangeRatesCacheEntry, ForexCacheError> {
    let fallback_api_key = async {
        #[cfg(feature = "hashicorp-vault")]
        let client = hashicorp_vault::get_hashicorp_client(hc_config)
            .await
            .change_context(ForexCacheError::KmsDecryptionFailed)?;

        #[cfg(not(feature = "hashicorp-vault"))]
        let output = state.conf.forex_api.fallback_api_key.clone();
        #[cfg(feature = "hashicorp-vault")]
        let output = state
            .conf
            .forex_api
            .fallback_api_key
            .clone()
            .fetch_inner::<hashicorp_vault::Kv2>(client)
            .await
            .change_context(ForexCacheError::KmsDecryptionFailed)?;

        Ok::<_, error_stack::Report<ForexCacheError>>(output)
    }
    .await?;
    #[cfg(feature = "kms")]
    let fallback_forex_api_key = kms::get_kms_client(kms_config)
        .await
        .decrypt(fallback_api_key.peek())
        .await
        .change_context(ForexCacheError::KmsDecryptionFailed)?;

    #[cfg(not(feature = "kms"))]
    let fallback_forex_api_key = fallback_api_key.peek();

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
        .into_report()
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


/// Asynchronously releases the Redis lock by deleting the key from the Redis store.
/// 
/// # Arguments
/// 
/// * `state` - The application state containing the Redis store connection.
/// 
/// # Returns
/// 
/// An `Ok` containing the result of the deletion operation, or an `Err` containing a `ForexCacheError` if the Redis connection or lock release failed.
/// 
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

/// Asynchronously attempts to acquire a lock in Redis for a specific key with an expiry, using the provided application state.
async fn acquire_redis_lock(app_state: &AppState) -> CustomResult<bool, ForexCacheError> {
    app_state
        .store
        .get_redis_conn()
        .change_context(ForexCacheError::RedisConnectionError)?
        .set_key_if_not_exists_with_expiry(
            REDIX_FOREX_CACHE_KEY,
            "",
            Some(
                (app_state.conf.forex_api.local_fetch_retry_count
                    * app_state.conf.forex_api.local_fetch_retry_delay
                    + app_state.conf.forex_api.api_timeout)
                    .try_into()
                    .into_report()
                    .change_context(ForexCacheError::ConversionError)?,
            ),
        )
        .await
        .map(|val| matches!(val, redis_interface::SetnxReply::KeySet))
        .change_context(ForexCacheError::CouldNotAcquireLock)
}

/// Saves the forex exchange rates cache entry to Redis in the given app state.
/// 
/// # Arguments
/// 
/// * `app_state` - The app state containing the Redis store.
/// * `forex_exchange_cache_entry` - The forex exchange rates cache entry to be saved.
/// 
/// # Returns
/// 
/// A `CustomResult` indicating success or a `ForexCacheError` if an error occurs.
///
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


/// Asynchronously retrieves forex exchange rates from Redis cache using the provided app state.
/// Returns a `CustomResult` containing an `Option` of `FxExchangeRatesCacheEntry` or a `ForexCacheError`.
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

/// Check if the provided redis cache entry is expired based on the given call delay.
/// If the cache entry is not expired, return a cloned reference to the exchange rates data.
/// If the cache entry is expired, return None.
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

/// Asynchronously converts the given amount from one currency to another using Forex rates. 
/// 
/// # Arguments
/// 
/// * `state` - The application state
/// * `amount` - The amount to convert
/// * `to_currency` - The currency to convert to
/// * `from_currency` - The currency to convert from
/// * `kms_config` - The KMS configuration (optional, requires "kms" feature)
/// * `hc_config` - The HashiCorp Vault configuration (optional, requires "hashicorp-vault" feature)
/// 
/// # Returns
/// 
/// A `CustomResult` containing a `CurrencyConversionResponse` on success, or a `ForexCacheError` on failure
pub async fn convert_currency(
    state: AppState,
    amount: i64,
    to_currency: String,
    from_currency: String,
    #[cfg(feature = "kms")] kms_config: &kms::KmsConfig,
    #[cfg(feature = "hashicorp-vault")]
    hc_config: &external_services::hashicorp_vault::HashiCorpVaultConfig,
) -> CustomResult<api_models::currency::CurrencyConversionResponse, ForexCacheError> {
    let rates = get_forex_rates(
        &state,
        state.conf.forex_api.call_delay,
        state.conf.forex_api.local_fetch_retry_delay,
        state.conf.forex_api.local_fetch_retry_count,
        #[cfg(feature = "kms")]
        kms_config,
        #[cfg(feature = "hashicorp-vault")]
        hc_config,
    )
    .await
    .change_context(ForexCacheError::ApiError)?;

    let to_currency = api_models::enums::Currency::from_str(to_currency.as_str())
        .into_report()
        .change_context(ForexCacheError::CurrencyNotAcceptable)?;

    let from_currency = api_models::enums::Currency::from_str(from_currency.as_str())
        .into_report()
        .change_context(ForexCacheError::CurrencyNotAcceptable)?;

    let converted_amount =
        currency_conversion::conversion::convert(&rates.data, from_currency, to_currency, amount)
            .into_report()
            .change_context(ForexCacheError::ConversionError)?;

    Ok(api_models::currency::CurrencyConversionResponse {
        converted_amount: converted_amount.to_string(),
        currency: to_currency.to_string(),
    })
}
