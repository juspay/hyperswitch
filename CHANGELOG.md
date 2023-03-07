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