#### 2023-04-18

## New Features

* **connector:**
  *  [Shift4] add support for card 3DS payment (#828) (29999fe5)
  *  [Nuvei] add support for card mandates (#818) (298a0a49)
* **bank_redirects:**  modify api contract for sofort (#880) (fc2e4514)
  *  add template code for connector forte (#854) 
## Bug Fixes

* **connector:**  [coinbase] make metadata as option parameter (#887) (f5728955)
*  Update events table after notifying merchant (#871) (ba013026)
* **stripe:**  remove cancel reason validation for stripe (#876) (fa44c1f6)

## Enhancement

* **payments:**  make TokenizationAction clonable (#895) 

## Integration

*  Frm integration with hyperswitch (#857) 

## Refactors

*  use lowercase names for run environment and config files (#801) (ffaa8da0)
*  derive `Serialize` and `Deserialize` to `Country` enum (#882) (456c16fb)
* **storage_models, errors:**  impl StorageErrorExt for error_stack::Result<T, errors::StorageError> (#886) (b4020294)
* **router:**  KMS decrypt secrets when kms feature is enabled  (#868) (8905e663)

# 0.5.6 2023-04-14

## Build System / Dependencies

* **deps:**  bump `fred` from `5.2.0` to `6.0.0` (#869) (01bc162d)

## Continuous Integration

* **manual_release:**  add `multiple_mca` feature in ci (#872) (aebb4dca)

## New Features

* **core:**  add backwards compatibility for multiple mca (#866) (cf902f19)
* **router:**
  *  added dispute retrieve and dispute list apis (#842) (acab7671)
  *  separate straight through algorithm in separate column in payment attempt (#863) (01f86c49)
* **connector:**
  *  [Airwallex] add multiple redirect support for 3DS (#811) (d1d58e33)
  *  [Worldpay] add support for webhook (#820) (23511166)
  *  [Coinbase] [Opennode] Add support for crypto payments via PG redirection (#834) (b3d14737)
*  multiple connector account support for the same `country` (#816) (6188d515)
*  connector tokenization flow (#750) (29da1dfa)
* **process_tracker:**  changing runner selection to dyn dispatch (#853) (18b84c42)

## Bug Fixes

* **merchant_account:**  change `primary_business_details` to vec in update (#877) (396d24fe)
*  redis deserialization issue in tokenization call (#878) (5e9d7d6b)
*  duplication check fix in basilisk-hs (#881) (b12762e7)

## Refactors

* **Tokenization:**  remove ConnectorCallType from tokenization call (#862) (0d047e08)
* **router_env:**  improve logging setup (#847) (1b94d25f)
* **refund_type:** Feat/add copy derive (#849) (ccf03273)


# 0.5.5 (2023-04-10)

## New Features

* **api_models:**  derive `strum::Display` for `RefundStatus` (#846) (4524d4f5)
*  allow (de)serializing countries to/from alpha-2, alpha-3 and numeric country codes (#836) (899767cf)
* **connector:**  add authorize, capture, void, psync, refund, rsync for PayPal connector (#747) (36049c13)

## Bug Fixes

*  Add locker sign keyid in env (#844) (70dff140)

## Other Changes

* **common_utils:**  put the async ext trait behind a feature (#835) (de29eb68)
*  update ci workflows for common_enums crate (#843) (45111337)

## Refactors

* **scheduler:**  remove scheduler options & adding graceful shutdown to producer (#840) (11df8436)
* **router:**  refactor amount in PaymentsCaptureData from Option<i64> to i64 (#821) (b8bcba4e)

# 0.5.4 (2023-04-04)

## New Features

* **request:**  add `RequestBuilder` method to attach default request headers (#826) (6f61f830)
* **middleware:**  add middleware to attach default response headers (#824) (6d7b11a0)
* **core:**  added multiple payment_attempt support for payment_intent (#439) (35d3e277)
* **router:**  added incoming dispute webhooks flow (#769) (a733eafb)

## Bug Fixes

* **cards_info:**  add extra columns to cards_info struct (#813) (442bed0f)
* **connector:**  [Mollie] remove unsupported implementation of Void flow from mollie connector (#808) (eee8304b)

## Other Changes

* **common_utils:**  put the signals module behind a feature flag (#814) (fb4ec431)
* **core:**  replace string with enum for country (#735) (e18bfb2a)
* **api_models:**  put the errors module behind a feature flag (#815) (f14f87a1)
* **storage_models:**  delete client secret when status is succeeded, failed, cancelled (#724) (a05059b7)
## Refactors

* **drainer, router:**  KMS decrypt database password when `kms` feature is enabled (#733) (9d6e4ee3)

# 0.5.3 (2023-03-29)

## Documentation Changes

* **rfc:**  add rfc template & first RFC (#806) (01a5e0a0)

## New Features

*  cards info api (#749) (b15b8f7b)
* **connector:**  [Nuvei] add webhook support (#795) (20b4372b)

## Bug Fixes

* **compatibility:**  add last_payment_error in stripe payment response (#803) (97b95f0e)

## Refactors

* **api_models:**  enhance accepted countries/currencies types (#807) (f9ef3135)
* **services:**   make AppState impl generic using AppStateInfo (#805) (642c3f3a)

# 0.5.2 (2023-03-24)

## Chores

*  prepare for building production Docker images (#794) (6ddc30eb)

## Bug Fixes

* **connector:**  [Airwallex] Change Session Token to Init Payment (#798) (a3c00339)

## Other Changes

* **router:**  change MAX_ID_LENGTH to 64 (#792) (346bd954)

## Refactors

*  extract kms module to `external_services` crate (#793) (029e3894)

# 0.5.1 (2023-03-21)

## Documentation Changes

* **try_local_system:**
  *  add Ubuntu on WSL2 setup instructions (#767) (1d2166cf)
  *  add API key creation step (#765) (4b268068)

## New Features

* **pm_list:**  handle client secret check (#759) (82344fc4)
*  add in-memory cache support for config table (#751) (abedaae4)
*  compile time optimization (#775) (5b5557b7)
* **router:**
  *  add support for stateful straight through routing (#752) (568bf01a)
  *  adding metrics for tracking behavior throughout the `router` crate  (#768) (d302b286)
* **router_env:**
  *  making metric flow as a trait for extensibility (#797) (df699e2b)
* **core:**  accept gateway credentials in the request body in payments and refunds (#766) (cb188f92)
* **connector:**
  *  Add support to provide connector_payment_meta for capture and void request (#770) (6c008ae6)
  *  [Trustpay] add webhooks (payment and refund events) (#746) (853dfa16)

## Bug Fixes

*  process delete response from basilisk-v3 as plaintext instead of JWE (#791) (699ca4f)
* **storage:**  add serialization for primitivedatetime for diesel structs (#764) (f27732a6)

## Refactors

*  get connection pool based on olap/oltp features (#743) (a392fb16)

# 0.5.0 (2023-03-21)

## Build System / Dependencies

* **deps:**  update deps (#734) (16bc886c)

## Chores

* **merchant_account:**  remove `api_key` field (#713) (230fcdd4)
* **config:**  move connector base URLs under the `[connectors]` table (#723) (df8c8b5a)
*  address Rust 1.68 clippy lints (#728) (1ffabb40)

## Continuous Integration

* **release:**  specify `fetch-depth` for code checkout and use official Docker GitHub actions (#722) (c451368f)

## Documentation Changes

*  Update naming conventions and added examples (#709) (98415193)
* **openapi:**  document path parameters for API keys endpoints (#702) (9062dc80)

## New Features

* **connector:**
  *  [Mollie]: add authorize, void, refund, psync, rsync support for mollie connector (#740) (168fa32)
  *  [worldline] add webhook support for connector (#721) (13a8ce8e)
  *  [Trustpay] add authorize (cards 3ds, no3ds and bank redirects), refund, psync, rsync (#717) (e102cae7)
  *  [Fiserv] add Refunds, Cancel and Wallets flow along with Unit Tests (#593) (cd1c5409)
  *  Add support for complete authorize payment after 3DS redirection (#741) (ec2b1b18)
*  removing unnecessary logs from console (#753) (1021d1ae)
*  Time based deletion of temp card (#729) (db3d3164)
*  populate fields from payment attempt in payment list (#736) (b5b3d57c)
*  add generic in-memory cache interface (#737) (7f5e5d86)
*  Add HSTS headers to response (#725) (7ed665ec)
*  cache reverse lookup fetches on redis (#719) (1a27faca)
* **compatibility:**  add webhook support for stripe compatibility (#710) (79160504)

## Bug Fixes

* **docker-compose:**  remove port for hyperswitch-server-init in docker-compose.yml (#763) (20b93276)
*  fixing docker compose setup & adding redisinsight (#748) (5c9bec9f)
* **kms:**  log KMS SDK errors using the `Debug` impl (#720) (468aa87f)
* **errors:**
  *  Replace PaymentMethod with PaymentModethodData in test.rs (#716) (763ee094)
  *  use `Debug` impl instead of `Display` for error types wrapping `error_stack::Report` (#714) (45484752)

## Other Changes

*  card_fingerprint not sent by basilisk_hs (#754) (5ae2f63f)

## Refactors

* **merchant_account:**  add back `api_key` field for backward compatibility (#761) (661dd48a)
* **connector:**  update add_connector script (#762) (78794ed6)
* **metrics:**  use macros for constructing counter and histogram metrics (#755) (58106d91)
* **kms:**  share a KMS client for all KMS operations (#744) (a3ff2e8d)
*  Basilisk hs integration (#704) (585618e5)
*  Add service_name to get and delete request (#738) (8b7ae9c3)
*  Add secret to metadata (#706) (d36afbed)
* **client:**
  *  simplify HTTP client construction (#731) (1756d1c4)
  *  remove dependence on `ROUTER_HTTP_PROXY` and `ROUTER_HTTPS_PROXY` env vars (#730) (c085e460)
* **authentication:**  authenticate merchant by API keys from API keys table (#712) (afd08d42)
* **api_keys:**  use a KMS encrypted API key hashing key and remove key ID prefix from plaintext API keys (#639) (3a3b33ac)

## Tests

* **masking:**  add suitable feature gates for basic tests (#745) (4859b6e4)
# 0.3.0 (2023-03-05)

## Chores
* **connectors:**  log connector request and response at debug level (#624) (6a487b19)

## Continuous Integration

* **workflow:** adding build only sandbox feature to reduce build time (#664) (d1c9305e)
* **workflow:** run cargo hack only for code changes (#663) (f931c427)

## Documentation Changes

* **openapi:**  document security schemes (#676) (c5fda7ac)

## New Features

* **session_token:**  create session token only if pmt is enabled (#703) (e1afeb64)
* **router:**
  *  serve OpenAPI docs at `/docs` (#698) (ed2907e1)
  *  added incoming refund webhooks flow (#683) (f12abbce)
* **list:**  global filter mapping for payment methods via card network (#694) (adca6bca)
*  store card network for cards (#687) (bfca26d9)
*  add support for `ANG` currency (#681) (03096eff)
*  Add bank redirect mapping to adyen and stripe (#680) (e6f627d9)
*  api contract change for wallet (#628) (ff86417e)
*  Add support for a redis pubsub interface (#614) (aaf37250)
*  initial `nix` setup using `cargo2nix` (#599) (73d0538d)
* **connector:**
  *  [Bambora] Add support for cards Authorize, psync, capture, void, refund, Rsync (#677) (0de5d441)
  *  [MultiSafePay] Add support for cards Authorize, psync, capture, void, refund, Rsync  (#658) (79aa8f3d)
  *  [Dlocal] Add support for authorize, capture, void, refund, psync, rsync (#650) (7792de55)
* **pm_list:**  support for sending bank names (#678) (576f8e1f)
* **card_network:**  add additional enum variants in card network (#671) (db8bc164)
* **stripe:**
  *  eps, giropay and ideal using stripe (#529) (028e1401)
  *  get error message for failed redirect payments (#615) (12f25f05)

## Bug Fixes

*  Populate amount_captured in case of success (#700) (d622b743)
*  Error Mapping for Bluensap & Card Number for Airwallex (#686) (35a74baf)
*  add currency in verify request data (#619) (32de632d)
*  add zero-padded formatting for error code (#627) (63f9b612)
*  check if bank_pm exists and then send request (#679) (76a9b557)
* **connector:**
  *  convert cents to dollar before sending to connector (#699) (3e883192)
  *  fix wordline card number validation issue (#695) (1a875348)
  *  fix wordline tests and visa card issuer support (#688) (d0c9dded)
* **adyen:**  adyen psync fail fix (#691) (2e99152d)
* **customer:**  populate email from customer table if not present in request (#692) (cf71d7aa)
* **list:**
  *  remove enabled payment methods from list customer payment method (#689) (5c29f37a)
  *  fix card network filtering (#684) (718c8a42)
  *  adding config changes for filtering `pm` based on countries & currencies (#669) (060c5419)
* **compatibility:**
  *  change next_action type and customer request type (#675) (7f22c22c)
  *  map stripe country_code to payment_request country code (#667) (7044b80b)
* **core:**  send metadata in payments response (#670) (b80f19e2)
* **router:**  allow setup future usage to be updated in payment update and confirm requests (#610) (7fd82211)

## Other Changes

* **stripe:**  send statement descriptor to stripe (#707) (641c4d6d)
*  use connector error handler for 500 error messages. (#696) (9fe20932)
*  populate failed status and add bank_redirect (#674)
* **refunds:**  skip validate refunds for card (#672) (5cdbef04)
* **router/webhooks:**  expose additional incoming request details to webhooks flow (#637) (1b3b7f5b)
* **braintree:**  create basic auth for braintree (#602) (c47619b5)

## Refactors

*  add better log to parse struct (#621) (275155a8)
*  Pass country and currency as json format in MCA (#523) (d27e6be5)
*  use simple uuid instead of hyphens (#605) (c467a47a)
*  add payment_issuer and payment_experience in pa (#491) (66563595)
* **router:**  remove foreign wrapper type (#616) (7bd2008a)
* **core:**
  *  add payment method list route to payment_methods (#682) (5449ce46)
  *  make attempt id as mandatory in router_data (#604) (626e467e)
* **pm_list:**
  *  pm_list for bank redirects (#685) (2701cceb)
  *  modify pm list to support new api contract (#657) (a2616d87)
* **connector:**  remove `peek()` on PII info (#642) (46f77d07)
* **connector-template:**  raise errors instead of using `todo!()` (#620) (b1a6be5a)
* **redirection:**  `From` impl for redirection data for ease of use (#613) (e8255b4a)

# 0.3.0 (2023-02-25)

## Build System / Dependencies

* **docker-compose:**  increase docker health check interval for hyperswitch-server (#534)

## Chores

* **release:**  port release bug fixes to main branch (#612) (a8d6ce83)

## Continuous Integration

*  run CI checks on merge queue events (#530) (c7b9e9c1)

## Documentation Changes

* **add_connector:**  fix typo (#584) (a4f3abf3)

## New Features

* **router:**
  *  include eligible connectors list in list payment methods (#644) (92771b3b)
  *  API endpoints for managing API keys (#511) (1bdc8955)
* **connector:**
  *  [Airwallex] add authorize, capture, void, psync, Webhooks support (#646) (6a67dd8b)
  *  [Bluesnap] add authorize, capture, void, refund, psync, rsync and Webhooks support (#649) (7efdc3c5)
  *  add authorize, capture, void, refund, psync support for Nuvei (#645) (03a9f5a9)
*  Added applepay feature (#636) (1e84c07c)
*  add `track_caller` to functions that perform `change_context` (#592) (8d2e573a)
* Redis cache for MCA fetch and update (#515) (963cb528)
* **api_models:**  add error structs (#532) (d107b44f)

## Bug Fixes

* **connector:**  update Bluesnap in routable connectors  (#654) (64cb2ffc)
*  allow errors with status code 200 to pass (#601) (8a8767e9)
*  don't call connector if connector transaction id doesn't exist (#525) (326d6beb)
*  throw 500 error when redis goes down (#531) (aafb115a)
* **router:**
  *  allow setup future usage to be updated in payment update and confirm requests (#610) (#638) (6c128f82)
  *  feature gate openssl deps for basilisk feature (#536) (e4956820)
* **checkout:**  Error Response when wrong api key is passed (#596) (55b6d88a)
* **core:**  use guard for access token result (#522) (903b4521)

## Other Changes

* **router:**
  *  webhooks enhancement (#637) (#641) (3bc9feb0)
  *  api keys path params (#609) (effa7a00)

## Refactors

* **router:**
  *  update payments api contract to accept a list of connectors (#643) (8f1f626c)
  *  api-key routes refactoring (#600) (e6408276)
  *  appstate as trait in authentication (#588) (eaf98e66)
* **compatibility:**  add additional fields to stripe payment and refund response types (#618) (2ea09e34)
*  Throw 500 error on database connection error instead of panic (#527) (f1e3bf48)
*  send full payment object for payment sync (#526) (6c2a1fea)
* **middleware:**  change visibility to `pub` (#587) (4884a24d)

# 0.2.1 (2023-02-17)

## Fixes
- fix payment_status not updated when adding payment method ([#446])
- Decide connector only when the payment method is confirm ([10ea4919ba07d3198a6bbe3f3d4d817a23605924](https://github.com/juspay/hyperswitch/commit/10ea4919ba07d3198a6bbe3f3d4d817a23605924))
- Fix panics caused with empty diesel updates ([448595498114cd15158b4a78fc32d8e6dc1b67ee](https://github.com/juspay/hyperswitch/commit/448595498114cd15158b4a78fc32d8e6dc1b67ee))


# 0.2.0 (2023-01-23) - Initial Release

## Supported Connectors

- [ACI](https://www.aciworldwide.com/)
- [Adyen](https://www.adyen.com/)
- [Authorize.net](https://www.authorize.net/)
- [Braintree](https://www.braintreepayments.com/)
- [Checkout.com](https://www.checkout.com/)
- [Cybersource](https://www.cybersource.com)
- [Fiserv](https://www.fiserv.com/)
- [Global Payments](https://www.globalpayments.com)
- [Klarna](https://www.klarna.com/)
- [PayU](https://payu.in/)
- [Rapyd](https://www.rapyd.net/)
- [Shift4](https://www.shift4.com/)
- [Stripe](https://stripe.com/)
- [Wordline](https://worldline.com/)


## Supported Payment Methods

- Cards No 3DS
- Cards 3DS*
- [Apple Pay](https://www.apple.com/apple-pay/)*
- [Google Pay](https://pay.google.com)*
- [Klarna](https://www.klarna.com/)*
- [PayPal](https://www.paypal.com/)*

## Supported Payment Functionalities

- Payments (Authorize/Sync/Capture/Cancel)
- Refunds (Execute/Sync)
- Saved Cards
- Mandates (No 3DS)*
- Customers
- Merchants
- ConnectorAccounts

\*May not be supported on all connectors
