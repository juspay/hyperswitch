# Changelog

All notable changes to HyperSwitch will be documented here.

- - -

## 2024.05.24.1

### Features

- **payment_charges:** Add support for collecting and refunding charges on payments ([#4628](https://github.com/juspay/hyperswitch/pull/4628)) ([`55ccce6`](https://github.com/juspay/hyperswitch/commit/55ccce61898083992afeab03ba1690954b1b45ef))

### Bug Fixes

- **payment_methods:**
  - Log and ignore the apple pay metadata parsing error while fetching apple pay retry connectors ([#4747](https://github.com/juspay/hyperswitch/pull/4747)) ([`a7fc4c6`](https://github.com/juspay/hyperswitch/commit/a7fc4c6fcd2f031b92e36f40a14be641673b7422))
  - Revert the filter for getting the mcas which are disabled ([#4756](https://github.com/juspay/hyperswitch/pull/4756)) ([`9fb2a83`](https://github.com/juspay/hyperswitch/commit/9fb2a8301453b47e2d1c17e215f740bea8eaa91a))

**Full Changelog:** [`2024.05.24.0...2024.05.24.1`](https://github.com/juspay/hyperswitch/compare/2024.05.24.0...2024.05.24.1)

- - -

## 2024.05.24.0

### Features

- **analytics:** Added client columns in payments analytics ([#4658](https://github.com/juspay/hyperswitch/pull/4658)) ([`0b415dc`](https://github.com/juspay/hyperswitch/commit/0b415dcca67f2994727627990a9cc9db19885b34))
- **router:** Send message_version and directory_server_id in next_action block of three_ds_data for external 3ds flow ([#4715](https://github.com/juspay/hyperswitch/pull/4715)) ([`13f6efc`](https://github.com/juspay/hyperswitch/commit/13f6efc7e8c01b4a377f627b9cfe2319b518204d))
- **users:**
  - Create terminate 2fa API ([#4731](https://github.com/juspay/hyperswitch/pull/4731)) ([`42e5ef1`](https://github.com/juspay/hyperswitch/commit/42e5ef155128f4df717e8fb101da6e6929659a0a))
  - Add support to verify 2FA using recovery code ([#4737](https://github.com/juspay/hyperswitch/pull/4737)) ([`f04c6ac`](https://github.com/juspay/hyperswitch/commit/f04c6ac030485cb28ab09e85a0f2f3c13beb6df3))
- Authentication analytics ([#4684](https://github.com/juspay/hyperswitch/pull/4684)) ([`5e5eb5f`](https://github.com/juspay/hyperswitch/commit/5e5eb5fbae7de2e296899e0372c82906603526d6))

### Bug Fixes

- **kafka:** Fix kafka timestamps sent from application ([#4709](https://github.com/juspay/hyperswitch/pull/4709)) ([`c778af2`](https://github.com/juspay/hyperswitch/commit/c778af26ddb46ff98072e8934a9509ff6e00ddc5))
- **payment_methods:** Mask the email address being logged in the `payment_method_list` response logs ([#4749](https://github.com/juspay/hyperswitch/pull/4749)) ([`23c7395`](https://github.com/juspay/hyperswitch/commit/23c73951bbdd5e049b75ca6d8e3bcccfb629e6eb))

### Refactors

- **bank-redirect:** Dynamic field changes for bankredirect payment method ([#4650](https://github.com/juspay/hyperswitch/pull/4650)) ([`da2dc10`](https://github.com/juspay/hyperswitch/commit/da2dc10f3d7233a0a9eae7d23cb07f7e8fafad78))
- **payment_methods:** Use recurring enabled flag to decide which payment method supports MIT ([#4732](https://github.com/juspay/hyperswitch/pull/4732)) ([`ba624d0`](https://github.com/juspay/hyperswitch/commit/ba624d049840f65fc21a5e578f8d4ba8543e1420))

### Miscellaneous Tasks

- Move RouterData Request types to hyperswitch_domain_models crate ([#4723](https://github.com/juspay/hyperswitch/pull/4723)) ([`ae77373`](https://github.com/juspay/hyperswitch/commit/ae77373b4cac63979673fdac37c55986d954358e))

**Full Changelog:** [`2024.05.23.0...2024.05.24.0`](https://github.com/juspay/hyperswitch/compare/2024.05.23.0...2024.05.24.0)

- - -

## 2024.05.23.0

### Features

- **connector:**
  - Accept connector_transaction_id in 4xx error_response of connector ([#4720](https://github.com/juspay/hyperswitch/pull/4720)) ([`2ad7fc0`](https://github.com/juspay/hyperswitch/commit/2ad7fc0cd6c102bea4d671c98f7fe50fd709d4ec))
  - [AUTHORIZEDOTNET] Implement zero mandates ([#4704](https://github.com/juspay/hyperswitch/pull/4704)) ([`8afeda5`](https://github.com/juspay/hyperswitch/commit/8afeda54fc5e3f3d510c48c81c222387e9cacc0e))
- **payment_methods:** Enable auto-retries for apple pay ([#4721](https://github.com/juspay/hyperswitch/pull/4721)) ([`d942a31`](https://github.com/juspay/hyperswitch/commit/d942a31d60595d366977746be7215620da0ababd))
- **routing:** Use Moka cache for routing with cache invalidation ([#3216](https://github.com/juspay/hyperswitch/pull/3216)) ([`431560b`](https://github.com/juspay/hyperswitch/commit/431560b7fb4401d000c11dbb9c7eb70663591307))
- **users:** Create generate recovery codes API ([#4708](https://github.com/juspay/hyperswitch/pull/4708)) ([`8fa2cd5`](https://github.com/juspay/hyperswitch/commit/8fa2cd556bf898621a1a8722a0af99d174447485))
- **webhook:** Add frm webhook support ([#4662](https://github.com/juspay/hyperswitch/pull/4662)) ([`ae601e8`](https://github.com/juspay/hyperswitch/commit/ae601e8e1be9215488daaae7cb39ad5a030e98d9))

### Bug Fixes

- **core:** Fix failing token based MIT payments ([#4735](https://github.com/juspay/hyperswitch/pull/4735)) ([`1bd4061`](https://github.com/juspay/hyperswitch/commit/1bd406197b5baf1c041f0dffa5bc02dce10f1529))
- Added hget lookup for all updated_by existing cases ([#4716](https://github.com/juspay/hyperswitch/pull/4716)) ([`fabf80c`](https://github.com/juspay/hyperswitch/commit/fabf80c2b18ca690b7fb709c8c12d1ef7f24e5b6))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`ec50843`](https://github.com/juspay/hyperswitch/commit/ec508435a19c2942a5d66757a74dd06bed5b1a76))

**Full Changelog:** [`2024.05.22.0...2024.05.23.0`](https://github.com/juspay/hyperswitch/compare/2024.05.22.0...2024.05.23.0)

- - -

## 2024.05.22.0

### Features

- **core:** Add support for connectors having separate version call for pre authentication ([#4603](https://github.com/juspay/hyperswitch/pull/4603)) ([`528d692`](https://github.com/juspay/hyperswitch/commit/528d692a89f5cf9a82d1e5c28e4b3a1ef4bf6c6a))

### Refactors

- **graph:** Refactor the Knowledge Graph to include configs check, while eligibility analysis ([#4687](https://github.com/juspay/hyperswitch/pull/4687)) ([`a917776`](https://github.com/juspay/hyperswitch/commit/a917776bb8cd294f77c569e81ea4d665b6611c6d))

### Miscellaneous Tasks

- Move tracing to workspace deps and remove router_env as a dependency of redis_interface ([#4717](https://github.com/juspay/hyperswitch/pull/4717)) ([`fea2ea6`](https://github.com/juspay/hyperswitch/commit/fea2ea6d2cf4f3f68e4779e53b82120806748d7b))

**Full Changelog:** [`2024.05.21.1...2024.05.22.0`](https://github.com/juspay/hyperswitch/compare/2024.05.21.1...2024.05.22.0)

- - -

## 2024.05.21.1

### Features

- **Cypress:** Add response handler for Connector Testing ([#4624](https://github.com/juspay/hyperswitch/pull/4624)) ([`2e79ee0`](https://github.com/juspay/hyperswitch/commit/2e79ee0615292182111586fda7655dd9a796ef4f))
- **constraint_graph:** Add visualization functionality to the constraint graph ([#4701](https://github.com/juspay/hyperswitch/pull/4701)) ([`0f53f74`](https://github.com/juspay/hyperswitch/commit/0f53f74d26e829602519998c41a460dc9a4809af))

### Refactors

- **core:** Add support to enable pm_data and pm_id in payments response ([#4711](https://github.com/juspay/hyperswitch/pull/4711)) ([`2cd360e`](https://github.com/juspay/hyperswitch/commit/2cd360e6a9d6bbe4b91f7b501b6013db1f31d898))
- **router:** Added a new type minor unit to amount ([#4629](https://github.com/juspay/hyperswitch/pull/4629)) ([`443b7e6`](https://github.com/juspay/hyperswitch/commit/443b7e6ea2cf63f35a28a1cd24860399d96b15ba))

**Full Changelog:** [`2024.05.21.0...2024.05.21.1`](https://github.com/juspay/hyperswitch/compare/2024.05.21.0...2024.05.21.1)

- - -

## 2024.05.21.0

### Features

- **core:** Add a new endpoint for Complete Authorize flow ([#4686](https://github.com/juspay/hyperswitch/pull/4686)) ([`226c337`](https://github.com/juspay/hyperswitch/commit/226c337399a2e4c1fa50c4f3d0d4b237b5543426))

### Bug Fixes

- **router:** Handle connector authentication technical failures and skip confirm in authorize flow only when authentication_type is not challenge ([#4667](https://github.com/juspay/hyperswitch/pull/4667)) ([`842728e`](https://github.com/juspay/hyperswitch/commit/842728ef93241643d12170695ddf56cee4da45bd))

### Refactors

- **cache:** Remove `deref` impl on `Cache` type ([#4671](https://github.com/juspay/hyperswitch/pull/4671)) ([`36409bd`](https://github.com/juspay/hyperswitch/commit/36409bdc9185d4241971a30c55e1e331568abd2f))

### Documentation

- Update Docker Compose setup guide to checkout `latest` tag ([#4695](https://github.com/juspay/hyperswitch/pull/4695)) ([`40f6776`](https://github.com/juspay/hyperswitch/commit/40f6776c46abc4b9c89fb2aa195f4ce64b312cf6))

### Miscellaneous Tasks

- **docker-compose:** Specify `pull_policy` for hyperswitch services ([#4688](https://github.com/juspay/hyperswitch/pull/4688)) ([`909e75c`](https://github.com/juspay/hyperswitch/commit/909e75c71a6e3418b5d15396569d986eff852c06))

**Full Changelog:** [`2024.05.20.2...2024.05.21.0`](https://github.com/juspay/hyperswitch/compare/2024.05.20.2...2024.05.21.0)

- - -

## 2024.05.20.2

### Features

- Add an api for toggle KV for all merchants ([#4600](https://github.com/juspay/hyperswitch/pull/4600)) ([`7f53461`](https://github.com/juspay/hyperswitch/commit/7f5346169edc4266b7b08578aac7aef1ede630f3))

**Full Changelog:** [`2024.05.20.1...2024.05.20.2`](https://github.com/juspay/hyperswitch/compare/2024.05.20.1...2024.05.20.2)

- - -

## 2024.05.20.1

### Features

- Soft kill kv ([#4582](https://github.com/juspay/hyperswitch/pull/4582)) ([`3fa59d4`](https://github.com/juspay/hyperswitch/commit/3fa59d4bac01de8fa25e28340a57e578d9980032))

**Full Changelog:** [`2024.05.20.0...2024.05.20.1`](https://github.com/juspay/hyperswitch/compare/2024.05.20.0...2024.05.20.1)

- - -

## 2024.05.20.0

### Features

- Added client_source, client_version in payment_attempt from payments confirm request headers ([#4657](https://github.com/juspay/hyperswitch/pull/4657)) ([`7e44bbc`](https://github.com/juspay/hyperswitch/commit/7e44bbca63c1818c0fabdf2734d9b0ae5d639fe1))

### Bug Fixes

- **docker:** Fix stack overflow for docker images ([#4660](https://github.com/juspay/hyperswitch/pull/4660)) ([`a62f69d`](https://github.com/juspay/hyperswitch/commit/a62f69d447245273c73611309055d2341a47b783))
- Address non-digit character cases in card number validation ([#4649](https://github.com/juspay/hyperswitch/pull/4649)) ([`8c0d72e`](https://github.com/juspay/hyperswitch/commit/8c0d72e225c56b7bece733d9565fc8774deaa490))

### Refactors

- **FRM:** Refactor frm configs ([#4581](https://github.com/juspay/hyperswitch/pull/4581)) ([`853f3b4`](https://github.com/juspay/hyperswitch/commit/853f3b4854ff9ec1e169b7633f1e9bf8259e9ceb))

**Full Changelog:** [`2024.05.17.0...2024.05.20.0`](https://github.com/juspay/hyperswitch/compare/2024.05.17.0...2024.05.20.0)

- - -

## 2024.05.17.0

### Bug Fixes

- **core:** Use `realip_remote_addr` function to extract ip address ([#4653](https://github.com/juspay/hyperswitch/pull/4653)) ([`8427b60`](https://github.com/juspay/hyperswitch/commit/8427b60a1851f2d9d2f141f28eb122d42f680736))
- **recon:** Make recon status optional in merchant account ([#4654](https://github.com/juspay/hyperswitch/pull/4654)) ([`84cb2bc`](https://github.com/juspay/hyperswitch/commit/84cb2bcb6bbb82f54315c82c7421a222d2e37bc6))

### Refactors

- **access_token:** Handle network delays with expiry of access token ([#4617](https://github.com/juspay/hyperswitch/pull/4617)) ([`0d45f85`](https://github.com/juspay/hyperswitch/commit/0d45f854a2cc18cc421a3d449a6dc2c830ef9dd5))
- **cards,router:** Remove duplicated card number interface ([#4404](https://github.com/juspay/hyperswitch/pull/4404)) ([`27ae437`](https://github.com/juspay/hyperswitch/commit/27ae437a88492bf5b17ad2fbf4a083891602c07a))

### Miscellaneous Tasks

- Add deprecated flag to soon to be deprecated fields in payment request and response ([#4261](https://github.com/juspay/hyperswitch/pull/4261)) ([`9ac5d70`](https://github.com/juspay/hyperswitch/commit/9ac5d70e2ed0a036b5f2bfe7488f218b83fce7c3))

**Full Changelog:** [`2024.05.16.1...2024.05.17.0`](https://github.com/juspay/hyperswitch/compare/2024.05.16.1...2024.05.17.0)

- - -

## 2024.05.16.1

### Features

- **middleware:** Log content_length for 4xx ([#4655](https://github.com/juspay/hyperswitch/pull/4655)) ([`4b5b558`](https://github.com/juspay/hyperswitch/commit/4b5b558dae8d2fefb66b8b16c486f07e3e800758))

### Refactors

- **session_flow:** Remove the shipping and billing parameter fields if null for apple pay and google pay ([#4661](https://github.com/juspay/hyperswitch/pull/4661)) ([`0dee53e`](https://github.com/juspay/hyperswitch/commit/0dee53ecb2d5203285a819bc8e71111d2c133f03))

**Full Changelog:** [`2024.05.16.0...2024.05.16.1`](https://github.com/juspay/hyperswitch/compare/2024.05.16.0...2024.05.16.1)

- - -

## 2024.05.16.0

### Features

- **core:** Move RouterData to crate hyperswitch_domain_models ([#4524](https://github.com/juspay/hyperswitch/pull/4524)) ([`ff1c2dd`](https://github.com/juspay/hyperswitch/commit/ff1c2ddf8b9d8f35deee1ab41c2286cc5b349271))

### Bug Fixes

- **connector:** Accept state abbreviation in 2 letter ([#4646](https://github.com/juspay/hyperswitch/pull/4646)) ([`3cf840e`](https://github.com/juspay/hyperswitch/commit/3cf840e48678e56a443bc891c48589d4b53bc07a))
- **router:** Add `max_amount` validation in payment flows ([#4645](https://github.com/juspay/hyperswitch/pull/4645)) ([`df865d7`](https://github.com/juspay/hyperswitch/commit/df865d76be1c867b9ee4d9cbb92a98dca4ecf229))

### Refactors

- **bank-redirect:** Remove billing from bankredirect payment data ([#4362](https://github.com/juspay/hyperswitch/pull/4362)) ([`0958d94`](https://github.com/juspay/hyperswitch/commit/0958d948f98bc41df64d8ea18cb1a8d3a0eb80fe))
- **db:** Add TenantID field to KafkaEvent struct ([#4598](https://github.com/juspay/hyperswitch/pull/4598)) ([`24214bc`](https://github.com/juspay/hyperswitch/commit/24214bcfcd0a34acd39dba88f6c015ac6b1edbc4))
- **router:** Remove default case handling in bambora connector ([#4473](https://github.com/juspay/hyperswitch/pull/4473)) ([`1a27ba5`](https://github.com/juspay/hyperswitch/commit/1a27ba576427126cc6a3fe2be86489abc9af63d8))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`f2ff7a2`](https://github.com/juspay/hyperswitch/commit/f2ff7a211b9f7ca16352061768e9b7c0a38a3845))

**Full Changelog:** [`2024.05.15.0...2024.05.16.0`](https://github.com/juspay/hyperswitch/compare/2024.05.15.0...2024.05.16.0)

- - -

## 2024.05.15.0

### Features

- **payment_methods:** Pass required shipping details field for wallets session call based on `business_profile` config ([#4616](https://github.com/juspay/hyperswitch/pull/4616)) ([`650f3fa`](https://github.com/juspay/hyperswitch/commit/650f3fa25c4130a2148862863ff444d16b41d2f3))
- **router:** Send `openurl_if_required` post_message in external 3ds flow for return_url redirection from sdk ([#4642](https://github.com/juspay/hyperswitch/pull/4642)) ([`bf06a5b`](https://github.com/juspay/hyperswitch/commit/bf06a5b51161365af7a3570a986455fefdf2c61b))

### Bug Fixes

- **config:** Include gpayments base url in deployment config files ([#4637](https://github.com/juspay/hyperswitch/pull/4637)) ([`03ed6dc`](https://github.com/juspay/hyperswitch/commit/03ed6dc0d6abc06ecfbbffe3111581fb4a0754da))

### Refactors

- **connector:** [BOA/CYBS] refund error handling ([#4632](https://github.com/juspay/hyperswitch/pull/4632)) ([`99702ed`](https://github.com/juspay/hyperswitch/commit/99702ed8f99cb03fc4452c067131aebf368de054))
- **payment_methods:** Update api contract for update payment method endpoint ([#4641](https://github.com/juspay/hyperswitch/pull/4641)) ([`e43ae65`](https://github.com/juspay/hyperswitch/commit/e43ae653a02cf453f8492630819e505c1f529f47))
- Remove `Ctx` generic from payments core ([#4574](https://github.com/juspay/hyperswitch/pull/4574)) ([`6b509c7`](https://github.com/juspay/hyperswitch/commit/6b509c7bec43fdd4332848498ce31023a26486e6))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`45b8814`](https://github.com/juspay/hyperswitch/commit/45b88140a2e43dfccfb5875a14dca5cd8b74b3fc))

**Full Changelog:** [`2024.05.14.0...2024.05.15.0`](https://github.com/juspay/hyperswitch/compare/2024.05.14.0...2024.05.15.0)

- - -

## 2024.05.14.0

### Features

- **connector:** Generate connector template code for gpayments authenticaition connector ([#4584](https://github.com/juspay/hyperswitch/pull/4584)) ([`2a302eb`](https://github.com/juspay/hyperswitch/commit/2a302eb5973c64d8b77f8110fdbeb536ccbe1488))
- **payment_methods:** Pass `required_billing_contact_fields` field in `/session` call based on dynamic fields ([#4601](https://github.com/juspay/hyperswitch/pull/4601)) ([`348cd74`](https://github.com/juspay/hyperswitch/commit/348cd744dca20c54c6ed47c8036f43f16429c8f3))
- **payments_update:** Update payment_method_billing in payment update ([#4614](https://github.com/juspay/hyperswitch/pull/4614)) ([`2692995`](https://github.com/juspay/hyperswitch/commit/26929956172e9f0e1e3fb41f5e4dbb19d866abf2))
- **refunds:** Update refunds filters ([#4409](https://github.com/juspay/hyperswitch/pull/4409)) ([`cfab2af`](https://github.com/juspay/hyperswitch/commit/cfab2af7d4a2478d7609a1bd34dd0579dad194c2))

### Bug Fixes

- **connector_token:** Move config redis ([#4540](https://github.com/juspay/hyperswitch/pull/4540)) ([`1602eb5`](https://github.com/juspay/hyperswitch/commit/1602eb541d317d9b155cbcbffed3d54f7d0b5acd))

### Refactors

- **bank-transfer:** Remove billing from banktransfer payment data ([#4377](https://github.com/juspay/hyperswitch/pull/4377)) ([`0f5a370`](https://github.com/juspay/hyperswitch/commit/0f5a370b55140fd63aeab4ca8427bd371f5e5ec4))
- **card_details:** Added missing card data fields for connectors ([#4571](https://github.com/juspay/hyperswitch/pull/4571)) ([`41655ba`](https://github.com/juspay/hyperswitch/commit/41655ba300567455a5b28b85584d990981a24167))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`22210b0`](https://github.com/juspay/hyperswitch/commit/22210b0ff44014b885842c804879267d9a83ab1b))

**Full Changelog:** [`2024.05.13.0...2024.05.14.0`](https://github.com/juspay/hyperswitch/compare/2024.05.13.0...2024.05.14.0)

- - -

## 2024.05.13.0

### Features

- **Connectors:** Add mandate validation for auth flow ([#4089](https://github.com/juspay/hyperswitch/pull/4089)) ([`fef28c3`](https://github.com/juspay/hyperswitch/commit/fef28c3345ae60046f46a2bdf9eca6b38278d75b))
- **analytics:** Authentication analytics ([#4429](https://github.com/juspay/hyperswitch/pull/4429)) ([`24d1542`](https://github.com/juspay/hyperswitch/commit/24d154248c8814e729206208f096aba68dcff8c0))

### Bug Fixes

- **connector:** [BOA/CYBS] add cancelled status to refund response ([#4620](https://github.com/juspay/hyperswitch/pull/4620)) ([`cf0e3da`](https://github.com/juspay/hyperswitch/commit/cf0e3daeaa1dfdfa00d4cccdff5b845ac368bcb9))
- **router:** Fix QR data into image conversion ([#4619](https://github.com/juspay/hyperswitch/pull/4619)) ([`28ab368`](https://github.com/juspay/hyperswitch/commit/28ab36873b2a475f1de95819b3d81aae954a2cfc))

### Refactors

- **payment_method_data:** Send optional billing details in response ([#4569](https://github.com/juspay/hyperswitch/pull/4569)) ([`86e0550`](https://github.com/juspay/hyperswitch/commit/86e05501cbea53fd85e2bc67a1c2be4cba47d0ff))

**Full Changelog:** [`2024.05.10.0...2024.05.13.0`](https://github.com/juspay/hyperswitch/compare/2024.05.10.0...2024.05.13.0)

- - -

## 2024.05.10.0

### Features

- **connector:** [Payone] add connector template code ([#4469](https://github.com/juspay/hyperswitch/pull/4469)) ([`f386f42`](https://github.com/juspay/hyperswitch/commit/f386f423c0e5fac55a24756d7ee7a3ce1c20fb13))
- **users:**
  - Create API to Verify TOTP ([#4597](https://github.com/juspay/hyperswitch/pull/4597)) ([`9135423`](https://github.com/juspay/hyperswitch/commit/91354232e03a8dbd9ad9eccc8620eac321765dd7))
  - New routes to accept invite and list merchants ([#4591](https://github.com/juspay/hyperswitch/pull/4591)) ([`e70d58a`](https://github.com/juspay/hyperswitch/commit/e70d58afc941d436aae0aaa683c2e8b5db2ade33))

### Bug Fixes

- **connector:**
  - [iatapay]handle empty error response in case of 401 ([#4291](https://github.com/juspay/hyperswitch/pull/4291)) ([`d1404d9`](https://github.com/juspay/hyperswitch/commit/d1404d9aff2aea513a2ffd422c7e10e760b7382c))
  - [BAMBORA] Audit Fixes for Bambora ([#4604](https://github.com/juspay/hyperswitch/pull/4604)) ([`366596f`](https://github.com/juspay/hyperswitch/commit/366596f14d6c874a8e2d418a99beb90046c5b040))
- **router:** [NETCETERA] skip sending browser_information in authentication request for app device_channel ([#4613](https://github.com/juspay/hyperswitch/pull/4613)) ([`d2a496c`](https://github.com/juspay/hyperswitch/commit/d2a496cf4ddab94efa5ad1127a94687d45bed027))
- **users:** Fix bugs caused by the new token only flows ([#4607](https://github.com/juspay/hyperswitch/pull/4607)) ([`a0f11d7`](https://github.com/juspay/hyperswitch/commit/a0f11d79add17e0bc19d8677c90f8a35d6c99c97))

### Refactors

- **billing:** Store `payment_method_data_billing` for recurring payments ([#4513](https://github.com/juspay/hyperswitch/pull/4513)) ([`55ae0fc`](https://github.com/juspay/hyperswitch/commit/55ae0fc5f704d8b35815fcd2170befb4a726ea8d))

**Full Changelog:** [`2024.05.09.0...2024.05.10.0`](https://github.com/juspay/hyperswitch/compare/2024.05.09.0...2024.05.10.0)

- - -

## 2024.05.09.0

### Features

- **business_profile:** Feature add a config to use `billing` as `payment_method_billing` ([#4557](https://github.com/juspay/hyperswitch/pull/4557)) ([`3e1c7eb`](https://github.com/juspay/hyperswitch/commit/3e1c7eba49de3110a2d71cea8e0540c7182d2058))
- **connector-configs:** [Cashtocode] add CNY currency for evoucher ([#4578](https://github.com/juspay/hyperswitch/pull/4578)) ([`c47cac8`](https://github.com/juspay/hyperswitch/commit/c47cac815792df865e416d5ffc6c46faf6662053))
- **users:** Create `user_key_store` table and `begin_totp` API ([#4577](https://github.com/juspay/hyperswitch/pull/4577)) ([`a97016f`](https://github.com/juspay/hyperswitch/commit/a97016fea41c3b74149d8eaa5c0271ec1347bc39))

### Bug Fixes

- **connector:** [BOA/CYBS] make rsync status optional ([#4570](https://github.com/juspay/hyperswitch/pull/4570)) ([`339da8b`](https://github.com/juspay/hyperswitch/commit/339da8b0c9a1e388b65ff5d82a162e758c85ec6b))
- **core:** Drop three_dsserver_trans_id from authentication table ([#4587](https://github.com/juspay/hyperswitch/pull/4587)) ([`ec3b60e`](https://github.com/juspay/hyperswitch/commit/ec3b60e37c0b178c3e5e3fe79db88f83fd195722))
- **users:** Correct the condition for `verify_email` flow in decision manger ([#4580](https://github.com/juspay/hyperswitch/pull/4580)) ([`3db5b82`](https://github.com/juspay/hyperswitch/commit/3db5b82d0de45130695e1a47b9e71473020fd84d))

### Refactors

- **bank-debit:** Remove billingdetails from bankdebit pmd ([#4371](https://github.com/juspay/hyperswitch/pull/4371)) ([`625b531`](https://github.com/juspay/hyperswitch/commit/625b53182e20b50fde5def338e122a43457da0f2))
- **db:** Add TenantId field to the KafkaStore struct ([#4512](https://github.com/juspay/hyperswitch/pull/4512)) ([`dca15ae`](https://github.com/juspay/hyperswitch/commit/dca15aeeb501a499fd7334d5cc68b8053757cad4))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`d85f245`](https://github.com/juspay/hyperswitch/commit/d85f245182875dcc59d8355cd07c91bfaaf08e1a))

**Full Changelog:** [`2024.05.08.0...2024.05.09.0`](https://github.com/juspay/hyperswitch/compare/2024.05.08.0...2024.05.09.0)

- - -

## 2024.05.08.0

### Features

- **FRM:** Add missing fields in Signifyd payment request ([#4554](https://github.com/juspay/hyperswitch/pull/4554)) ([`df2c2ca`](https://github.com/juspay/hyperswitch/commit/df2c2ca22dc4cea986cbbf30850311d3e85000c5))
- **connector:**
  - [Cybersource] Add payout flows for Card ([#4511](https://github.com/juspay/hyperswitch/pull/4511)) ([`a72f040`](https://github.com/juspay/hyperswitch/commit/a72f040d9281744bceb928ef2e8d3a26783aae9e))
  - [MiFinity] add connector template code ([#4447](https://github.com/juspay/hyperswitch/pull/4447)) ([`d974e6e`](https://github.com/juspay/hyperswitch/commit/d974e6e7c2e1e3cd99607183b647c420f4b14d20))
- **router:** Add an api to enable `connector_agnostic_mit` feature ([#4480](https://github.com/juspay/hyperswitch/pull/4480)) ([`e769abe`](https://github.com/juspay/hyperswitch/commit/e769abe501470185fcca29e0abede0654579da06))
- **users:**
  - Create Token only support for pre-login user flow APIs ([#4558](https://github.com/juspay/hyperswitch/pull/4558)) ([`5ec00d9`](https://github.com/juspay/hyperswitch/commit/5ec00d96de49ae0e0f76c5b19e22db11e7db6dd2))
  - Implement force set and force change password ([#4564](https://github.com/juspay/hyperswitch/pull/4564)) ([`59e79ff`](https://github.com/juspay/hyperswitch/commit/59e79ff205dfc2fded993b7a9130b9953bdd07e2))

### Bug Fixes

- **payment_methods:** Fix deserialization errors for `sdk_eligible_payment_methods` ([#4565](https://github.com/juspay/hyperswitch/pull/4565)) ([`f63a970`](https://github.com/juspay/hyperswitch/commit/f63a97024c755fd30a3403e2146812fe4edb8067))
- **users:** Add password validations ([#4555](https://github.com/juspay/hyperswitch/pull/4555)) ([`25fe4de`](https://github.com/juspay/hyperswitch/commit/25fe4deb8e9152b37467ac1fea18b3074f0e7624))

### Refactors

- **core:** Refactor authentication core to fetch authentication only within it ([#4138](https://github.com/juspay/hyperswitch/pull/4138)) ([`71a070e`](https://github.com/juspay/hyperswitch/commit/71a070e26989f080031d92a88aa0143836d1ea7b))
- Remove `configs/pg_agnostic_mit` api as it will not be used ([#4486](https://github.com/juspay/hyperswitch/pull/4486)) ([`99bbc39`](https://github.com/juspay/hyperswitch/commit/99bbc3982fa30f6ffd43334b1fa5da963975fe93))
- Store `card_cvc` in extended_card_info and extend max ttl ([#4568](https://github.com/juspay/hyperswitch/pull/4568)) ([`1b5b566`](https://github.com/juspay/hyperswitch/commit/1b5b566387da83a2582216e05be4ceb1aa7251be))

### Miscellaneous Tasks

- Address Rust 1.78 clippy lints ([#4545](https://github.com/juspay/hyperswitch/pull/4545)) ([`2216a88`](https://github.com/juspay/hyperswitch/commit/2216a88d25c42ede9862f6d036e7b0586a2e7c28))

**Full Changelog:** [`2024.05.07.0...2024.05.08.0`](https://github.com/juspay/hyperswitch/compare/2024.05.07.0...2024.05.08.0)

- - -

## 2024.05.07.0

### Features

- **clickhouse:** Init Clickhouse container on startup ([#4365](https://github.com/juspay/hyperswitch/pull/4365)) ([`89e5884`](https://github.com/juspay/hyperswitch/commit/89e5884f9eb341026b09a0d1ab8b836de5ba0c19))
- **constraint_graph:** Make the constraint graph framework generic and move it into a separate crate ([#3071](https://github.com/juspay/hyperswitch/pull/3071)) ([`a23a365`](https://github.com/juspay/hyperswitch/commit/a23a365cdf3fc2a24f4e2a08996a5683dc4da89a))
- **payment_methods:** Filter payment methods based on pm client secret ([#4249](https://github.com/juspay/hyperswitch/pull/4249)) ([`575fac6`](https://github.com/juspay/hyperswitch/commit/575fac6f3ef94ef1856a77d778f822c0e97b0e9c))
- Add decision starter API for email flows ([#4533](https://github.com/juspay/hyperswitch/pull/4533)) ([`1335554`](https://github.com/juspay/hyperswitch/commit/1335554f5193f05ba512d75a8eb9bb8047a65466))

### Refactors

- **paylater:** Use payment_method_data.billing fields instead of payment_method_data ([#4333](https://github.com/juspay/hyperswitch/pull/4333)) ([`b878677`](https://github.com/juspay/hyperswitch/commit/b878677f1572dceb9cd1983c2fd0b3b05ed8a573))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`25cd685`](https://github.com/juspay/hyperswitch/commit/25cd6854f9d9162915e56213b6989eeb1d178b49))

### Build System / Dependencies

- **docker:** Add web client and control center services to docker compose setup ([#4197](https://github.com/juspay/hyperswitch/pull/4197)) ([`b1cfef2`](https://github.com/juspay/hyperswitch/commit/b1cfef257a54b3ebe4f48e56a48a932e2c758dc7))

**Full Changelog:** [`2024.05.06.0...2024.05.07.0`](https://github.com/juspay/hyperswitch/compare/2024.05.06.0...2024.05.07.0)

- - -

## 2024.05.06.0

### Features

- **core:** Add profile level config to toggle extended card bin ([#4445](https://github.com/juspay/hyperswitch/pull/4445)) ([`0304e8e`](https://github.com/juspay/hyperswitch/commit/0304e8e76a8ca1f602305991c4129107b20d148e))
- **euclid_wasm:** Add configs for new payout connectors ([#4528](https://github.com/juspay/hyperswitch/pull/4528)) ([`9f41919`](https://github.com/juspay/hyperswitch/commit/9f41919094638baf9ea405a5acb89d69ecf1e2b7))

### Bug Fixes

- **connector:** [BAMBORA] Restrict Card Expiry Year to 2 Digits and pass Amount in Decimal Format ([#4536](https://github.com/juspay/hyperswitch/pull/4536)) ([`d5d9006`](https://github.com/juspay/hyperswitch/commit/d5d9006fbd8e32f822f1e84d486b8a4483164baa))
- **users:** Revert add password validations ([#4542](https://github.com/juspay/hyperswitch/pull/4542)) ([`bcce8b0`](https://github.com/juspay/hyperswitch/commit/bcce8b0489aad8455748e0945127f0a7447e8fb1))

### Refactors

- **connector:** [NMI] Change fields for external auth due to API contract changes ([#4531](https://github.com/juspay/hyperswitch/pull/4531)) ([`7417250`](https://github.com/juspay/hyperswitch/commit/74172509e3b9c0af04bb2fe8a5192ab7f7fd37b5))

### Documentation

- **cypress:** Update cypress docs ([#4505](https://github.com/juspay/hyperswitch/pull/4505)) ([`17b369c`](https://github.com/juspay/hyperswitch/commit/17b369cfabc42d4d06f65b92e967057dba348731))

**Full Changelog:** [`2024.05.03.1...2024.05.06.0`](https://github.com/juspay/hyperswitch/compare/2024.05.03.1...2024.05.06.0)

- - -

## 2024.05.03.1

### Bug Fixes

- **api_request:** Make `payment_method_data` as optional ([#4527](https://github.com/juspay/hyperswitch/pull/4527)) ([`83a1924`](https://github.com/juspay/hyperswitch/commit/83a192466849c5fd201296e7554644a025ced888))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`e3af9d0`](https://github.com/juspay/hyperswitch/commit/e3af9d0cfbf5e73822f3a097d8a36736efae3d3a))

**Full Changelog:** [`2024.05.03.0...2024.05.03.1`](https://github.com/juspay/hyperswitch/compare/2024.05.03.0...2024.05.03.1)

- - -

## 2024.05.03.0

### Features

- **connector:**
  - [Ebanx] Add payout flows ([#4146](https://github.com/juspay/hyperswitch/pull/4146)) ([`4f4cbdf`](https://github.com/juspay/hyperswitch/commit/4f4cbdff21b956b5939cdbe6a4f88f663e6b1281))
  - [Paypal] Add payout flow for wallet(Paypal and Venmo) ([#4406](https://github.com/juspay/hyperswitch/pull/4406)) ([`e4ed1e6`](https://github.com/juspay/hyperswitch/commit/e4ed1e63951873f299f076332671f4a043aa86ab))
- **core:** Rename crate data_models to hyperswitch_domain_models ([#4504](https://github.com/juspay/hyperswitch/pull/4504)) ([`86e93cd`](https://github.com/juspay/hyperswitch/commit/86e93cd3a0f050c89a82be409b80dc2894143c9e))
- **opensearch:** Refactoring ([#4244](https://github.com/juspay/hyperswitch/pull/4244)) ([`22cb01a`](https://github.com/juspay/hyperswitch/commit/22cb01ac1ecc90eee464561e4e944aad5cb3ed61))
- **user:** Add route to get user details ([#4510](https://github.com/juspay/hyperswitch/pull/4510)) ([`be44447`](https://github.com/juspay/hyperswitch/commit/be44447c09ea8814dc8b88aa971e08cc749db5f3))
- **users:** Create Decision manager for User Flows ([#4518](https://github.com/juspay/hyperswitch/pull/4518)) ([`4b3faf6`](https://github.com/juspay/hyperswitch/commit/4b3faf6781f8ab3198ca86b924f3225256d34b52))
- Store encrypted extended card info in redis ([#4493](https://github.com/juspay/hyperswitch/pull/4493)) ([`6c59d24`](https://github.com/juspay/hyperswitch/commit/6c59d2434ce5067611d85d37b7ec6f551b7ad81a))

### Bug Fixes

- **users:** Add password validations ([#4489](https://github.com/juspay/hyperswitch/pull/4489)) ([`67794da`](https://github.com/juspay/hyperswitch/commit/67794da36ec25531cbf991034452369b17da8063))

### Refactors

- **Connectors:** [BOA] enhance response objects ([#4508](https://github.com/juspay/hyperswitch/pull/4508)) ([`3ed0e8b`](https://github.com/juspay/hyperswitch/commit/3ed0e8b764d1f1bc7d249122dba39be7dfdcac8b))
- **user:** Use single purpose token and auth to accept invite ([#4498](https://github.com/juspay/hyperswitch/pull/4498)) ([`4b0cf9c`](https://github.com/juspay/hyperswitch/commit/4b0cf9ce3b793c370e754c159f7f2bf2f8b2a310))

### Miscellaneous Tasks

- **payouts:** Update deployment configs for connector_customer ([#4499](https://github.com/juspay/hyperswitch/pull/4499)) ([`5a447af`](https://github.com/juspay/hyperswitch/commit/5a447afd749c170bfe9f1a106fa28a4819a671dc))

**Full Changelog:** [`2024.05.02.0...2024.05.03.0`](https://github.com/juspay/hyperswitch/compare/2024.05.02.0...2024.05.03.0)

- - -

## 2024.05.02.0

### Features

- **FRM:** Add shipping details for signifyd ([#4500](https://github.com/juspay/hyperswitch/pull/4500)) ([`bda749d`](https://github.com/juspay/hyperswitch/commit/bda749d097ee9cfe80bc509491bec229da3725c3))
- Add support for merchant to pass public key and ttl for encrypting payload ([#4456](https://github.com/juspay/hyperswitch/pull/4456)) ([`b562e62`](https://github.com/juspay/hyperswitch/commit/b562e62ac895c34574bcc8c3fcce8e5b49d0f923))
- Add an api for retrieving the extended card info from redis ([#4484](https://github.com/juspay/hyperswitch/pull/4484)) ([`dfa4b50`](https://github.com/juspay/hyperswitch/commit/dfa4b50dbd5cfc927fbbd6a68725d2c51625e6d1))

### Bug Fixes

- **access_token:** Use fallback to `connector_name` if `merchant_connector_id` is not present ([#4503](https://github.com/juspay/hyperswitch/pull/4503)) ([`632a00c`](https://github.com/juspay/hyperswitch/commit/632a00cb6803e3e6f94099e48fb4198a0ea49f99))
- **connector:** Send valid sdk information in authentication flow netcetera ([#4474](https://github.com/juspay/hyperswitch/pull/4474)) ([`8f0d4d4`](https://github.com/juspay/hyperswitch/commit/8f0d4d4191bb96efd8700fb115d91213c2872ad8))
- **euclid_wasm:** Connector config wasm metadata update ([#4460](https://github.com/juspay/hyperswitch/pull/4460)) ([`28df646`](https://github.com/juspay/hyperswitch/commit/28df646830f544179b7cf00eb8f51de2a606bdbc))

### Refactors

- **core:** Remove payment_method_id from RouterData struct ([#4485](https://github.com/juspay/hyperswitch/pull/4485)) ([`3077a0d`](https://github.com/juspay/hyperswitch/commit/3077a0d31e8d36f18e359f1edf9a742969601f6b))
- **cypress:** Read creds from env instead of hardcoding the path ([#4430](https://github.com/juspay/hyperswitch/pull/4430)) ([`0c9ba1e`](https://github.com/juspay/hyperswitch/commit/0c9ba1e848c757cf3e0708f2ed4694259a5344aa))
- **user:** Deprecate Signin, Verify email and Invite v1 APIs ([#4465](https://github.com/juspay/hyperswitch/pull/4465)) ([`b0133f3`](https://github.com/juspay/hyperswitch/commit/b0133f33693575f2145d295eff78dd07b61efcda))

### Miscellaneous Tasks

- Make client certificate and private key secret across codebase ([#4490](https://github.com/juspay/hyperswitch/pull/4490)) ([`dd7b10a`](https://github.com/juspay/hyperswitch/commit/dd7b10a8bdad4c509a4fbae429f3abd21a5d6758))

**Full Changelog:** [`2024.04.30.0...2024.05.02.0`](https://github.com/juspay/hyperswitch/compare/2024.04.30.0...2024.05.02.0)

- - -

## 2024.04.30.0

### Features

- **FRM:** Revise post FRM core flows ([#4394](https://github.com/juspay/hyperswitch/pull/4394)) ([`01ec7c6`](https://github.com/juspay/hyperswitch/commit/01ec7c64a4e0536b11052a6d5f3b398216d7b1e3))
- **router:**
  - Send poll_config in next_action of confirm response for external 3ds flow ([#4443](https://github.com/juspay/hyperswitch/pull/4443)) ([`c3a1db1`](https://github.com/juspay/hyperswitch/commit/c3a1db16f32bd0b5aa49dfc831156a10d6d15838))
  - Handle authorization for frictionless flow in external 3ds flow ([#4471](https://github.com/juspay/hyperswitch/pull/4471)) ([`79d8949`](https://github.com/juspay/hyperswitch/commit/79d8949413c8007e261b66b01596d257fb5959f9))
- **user:** Add single purpose token and auth ([#4470](https://github.com/juspay/hyperswitch/pull/4470)) ([`c20ecb8`](https://github.com/juspay/hyperswitch/commit/c20ecb855aa3c4b3ce1798dcc19910fb38345b46))
- Stripe connect integration for payouts ([#2041](https://github.com/juspay/hyperswitch/pull/2041)) ([`ac9d856`](https://github.com/juspay/hyperswitch/commit/ac9d856add0220701f809c8eb0668afe77003ef7))

**Full Changelog:** [`2024.04.29.0...2024.04.30.0`](https://github.com/juspay/hyperswitch/compare/2024.04.29.0...2024.04.30.0)

- - -

## 2024.04.29.0

### Features

- **connector:** [CRYPTOPAY] Report underpaid/overpaid amount in outgoing webhooks ([#4468](https://github.com/juspay/hyperswitch/pull/4468)) ([`cc1051d`](https://github.com/juspay/hyperswitch/commit/cc1051da99c1b4e007d7f730e2fe3cb08b078d80))
- **users:** Use cookie for auth ([#4434](https://github.com/juspay/hyperswitch/pull/4434)) ([`b2b9fab`](https://github.com/juspay/hyperswitch/commit/b2b9fab31dc838958e59a7a6755a0585d5a10302))

### Refactors

- **access_token:** Use `merchant_connector_id` for storing access token ([#4462](https://github.com/juspay/hyperswitch/pull/4462)) ([`d98551d`](https://github.com/juspay/hyperswitch/commit/d98551d80a14e2878fbac93e4ba0ecb537018802))
- **required_fields:** Change required fields for billing address ([#4258](https://github.com/juspay/hyperswitch/pull/4258)) ([`e730030`](https://github.com/juspay/hyperswitch/commit/e730030e24d177b3e696b446e5ccb964cc07ee37))
- **scheduler:** Join frequency and count in `RetryMapping` ([#4313](https://github.com/juspay/hyperswitch/pull/4313)) ([`3335195`](https://github.com/juspay/hyperswitch/commit/33351953baf32be96f6ec11982c05f2bb1edb989))

**Full Changelog:** [`2024.04.26.0...2024.04.29.0`](https://github.com/juspay/hyperswitch/compare/2024.04.26.0...2024.04.29.0)

- - -

## 2024.04.26.0

### Features

- **core:** [Payouts] Add access_token flow for Payout Create and Fulfill flow ([#4375](https://github.com/juspay/hyperswitch/pull/4375)) ([`7f0d04f`](https://github.com/juspay/hyperswitch/commit/7f0d04fe3782cf6777c67e40e708c7abb5c4f45e))
- Add an api for toggling extended card info feature ([#4444](https://github.com/juspay/hyperswitch/pull/4444)) ([`87d9fce`](https://github.com/juspay/hyperswitch/commit/87d9fced07e5cc1366eb6d16d2584bd920ad16fe))

### Bug Fixes

- **connector:** [CYBERSOURCE] Handle HTML Error Response and add Descriptor field in ApplePay payments request ([#4451](https://github.com/juspay/hyperswitch/pull/4451)) ([`dbd3160`](https://github.com/juspay/hyperswitch/commit/dbd3160fcf310b2942ef096bfb091881bfeec902))

### Refactors

- **configs:** Add comments to configs for deployments to environments ([#4458](https://github.com/juspay/hyperswitch/pull/4458)) ([`9d096e6`](https://github.com/juspay/hyperswitch/commit/9d096e6b4883e34908eca0d5aa134a88bec22b40))
- **connector:** Pass optional browser_info to stripe for increased trust ([#4374](https://github.com/juspay/hyperswitch/pull/4374)) ([`4c793c3`](https://github.com/juspay/hyperswitch/commit/4c793c3c00e93ebf4a4db3439a213474ff57b54d))
- **core:** Make save_payment_method as post_update_tracker trait function ([#4307](https://github.com/juspay/hyperswitch/pull/4307)) ([`5f40eee`](https://github.com/juspay/hyperswitch/commit/5f40eee3fa264390ea6ac7feaca7737d83dccb3a))
- **payment_methods:** Store `card_network` in locker ([#4425](https://github.com/juspay/hyperswitch/pull/4425)) ([`5b54d55`](https://github.com/juspay/hyperswitch/commit/5b54d55c5e0d2c8ae1090fb566434efb50120857))
- **voucher:** Remove billing details from voucher pmd ([#4361](https://github.com/juspay/hyperswitch/pull/4361)) ([`2dd0ee6`](https://github.com/juspay/hyperswitch/commit/2dd0ee68bf23e5f49d22011f0294f44f4e97423b))

### Documentation

- **cypress:** Update Cypress README Documentation ([#4380](https://github.com/juspay/hyperswitch/pull/4380)) ([`8ee1a58`](https://github.com/juspay/hyperswitch/commit/8ee1a58c386fc5f08025c6ac90c96468e6d26bc7))
- Add documentation page for building Docker images ([#4457](https://github.com/juspay/hyperswitch/pull/4457)) ([`705e827`](https://github.com/juspay/hyperswitch/commit/705e82779a2b7bfd0cb1cd856b4a760d487cd8c5))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`047f917`](https://github.com/juspay/hyperswitch/commit/047f9171a5ae5e8211e18b1c882672dab0c26d07))

**Full Changelog:** [`2024.04.25.0...2024.04.26.0`](https://github.com/juspay/hyperswitch/compare/2024.04.25.0...2024.04.26.0)

- - -

## 2024.04.25.0

### Features

- **router:** Handle authorize redirection after webhook processing for external 3ds flow ([#4452](https://github.com/juspay/hyperswitch/pull/4452)) ([`131e487`](https://github.com/juspay/hyperswitch/commit/131e487c662985737e9b50a8e62295ed9d23ca83))

### Bug Fixes

- **routing/tests:** Fix unit tests for routing ([#4438](https://github.com/juspay/hyperswitch/pull/4438)) ([`1d0d94d`](https://github.com/juspay/hyperswitch/commit/1d0d94d5e6528534ce461db39620f35490582ecb))

### Documentation

- **try_local_system:** Update WSL setup guide to address a memory issue ([#4431](https://github.com/juspay/hyperswitch/pull/4431)) ([`56f14b9`](https://github.com/juspay/hyperswitch/commit/56f14b935d5e9742a894408a714033318ecb6f2a))

### Miscellaneous Tasks

- Remove repetitive words ([#4448](https://github.com/juspay/hyperswitch/pull/4448)) ([`f49b0b3`](https://github.com/juspay/hyperswitch/commit/f49b0b3aabdf72030cb893ce479214eccd5a6e0f))

**Full Changelog:** [`2024.04.24.0...2024.04.25.0`](https://github.com/juspay/hyperswitch/compare/2024.04.24.0...2024.04.25.0)

- - -

## 2024.04.24.0

### Features

- **connector:**
  - Implement authentication flow for netcetera ([#4334](https://github.com/juspay/hyperswitch/pull/4334)) ([`5ce0535`](https://github.com/juspay/hyperswitch/commit/5ce0535bb6798af16057c1323541ee4789dbceb1))
  - Add webhook support for netcetera ([#4382](https://github.com/juspay/hyperswitch/pull/4382)) ([`776c1a7`](https://github.com/juspay/hyperswitch/commit/776c1a7a24b494bf767c5524d1b8ac90472d32e2))

### Bug Fixes

- **masking:** Mask email while logging SQL query ([#4436](https://github.com/juspay/hyperswitch/pull/4436)) ([`4c81a66`](https://github.com/juspay/hyperswitch/commit/4c81a664c90ad749e80c372296c844b39dded334))
- **user:** Blacklist token after delete user role ([#4428](https://github.com/juspay/hyperswitch/pull/4428)) ([`b67e07f`](https://github.com/juspay/hyperswitch/commit/b67e07fb9ee576c57dcbca21c52aa1e4ed2d2818))

### Refactors

- **router:** Enable saved payment method for payment link bug fix ([#4435](https://github.com/juspay/hyperswitch/pull/4435)) ([`213ff06`](https://github.com/juspay/hyperswitch/commit/213ff063a0f6182f9ccd7cdb268aad1ec0372cc9))

### Miscellaneous Tasks

- **configs:** Add wasm changes for pull_mechanism_enabled config for 3dsecureio connector ([#4433](https://github.com/juspay/hyperswitch/pull/4433)) ([`b2248fe`](https://github.com/juspay/hyperswitch/commit/b2248fe08b0b075a9d326e862a18f50e5bef12f8))

**Full Changelog:** [`2024.04.23.0...2024.04.24.0`](https://github.com/juspay/hyperswitch/compare/2024.04.23.0...2024.04.24.0)

- - -

## 2024.04.23.0

### Features

- **euclied_wasm:** [NMI] Add configs for extended 3DS ([#4422](https://github.com/juspay/hyperswitch/pull/4422)) ([`b8be10d`](https://github.com/juspay/hyperswitch/commit/b8be10de52e40d2327819d33c6c1ec40a459bdd5))
- **router:** Add poll ability in external 3ds authorization flow ([#4393](https://github.com/juspay/hyperswitch/pull/4393)) ([`4476553`](https://github.com/juspay/hyperswitch/commit/447655382bcf2fdd69a1ec6a56e5e4df8a8feef2))

### Refactors

- **wallet:** Use `billing.phone` instead of `telephone_number` ([#4329](https://github.com/juspay/hyperswitch/pull/4329)) ([`3e6bc19`](https://github.com/juspay/hyperswitch/commit/3e6bc191fd5804feface9ee1a0cb7ddbbe025569))

### Miscellaneous Tasks

- Add wasm toml configs for netcetera authnetication connector ([#4426](https://github.com/juspay/hyperswitch/pull/4426)) ([`4851da1`](https://github.com/juspay/hyperswitch/commit/4851da1595074dbb2760e76f83403e8ac9f7895f))

**Full Changelog:** [`2024.04.22.0...2024.04.23.0`](https://github.com/juspay/hyperswitch/compare/2024.04.22.0...2024.04.23.0)

- - -

## 2024.04.22.0

### Features

- **payment_methods:** Client secret implementation in payment method… ([#4134](https://github.com/juspay/hyperswitch/pull/4134)) ([`4330781`](https://github.com/juspay/hyperswitch/commit/43307815e0200caf2e9517ec1374d09696356fbc))
- **router:** [BOA/CYBS] add avs_response and cvv validation result in the response ([#4376](https://github.com/juspay/hyperswitch/pull/4376)) ([`e458e49`](https://github.com/juspay/hyperswitch/commit/e458e4907e39961f386900f21382c9ace3b7c392))

### Bug Fixes

- **connectors:** Mask fields for webhook_resource_object ([#4400](https://github.com/juspay/hyperswitch/pull/4400)) ([`110bf22`](https://github.com/juspay/hyperswitch/commit/110bf22511cf4994c7325fb105fee60f910c1210))
- **core:** Fix 3DS mandates, for the connector _mandate_details to be stored in the payment_methods table ([#4323](https://github.com/juspay/hyperswitch/pull/4323)) ([`f4e5784`](https://github.com/juspay/hyperswitch/commit/f4e5784f6ce57b4a205c164889242bfa1bc1fde2))
- **user:** Add onboarding_survey enum in dashboard metadata type ([#4353](https://github.com/juspay/hyperswitch/pull/4353)) ([`f6fccaf`](https://github.com/juspay/hyperswitch/commit/f6fccafb3d43ce4b2865cf4b3cba7ad8a9619e5b))

**Full Changelog:** [`2024.04.19.0...2024.04.22.0`](https://github.com/juspay/hyperswitch/compare/2024.04.19.0...2024.04.22.0)

- - -

## 2024.04.19.0

### Features

- **connector:** [NMI] External 3DS flow for Cards ([#4385](https://github.com/juspay/hyperswitch/pull/4385)) ([`4feda8f`](https://github.com/juspay/hyperswitch/commit/4feda8f89049b830f974e82f414720fd12608170))
- **payments:** Add amount and connector id filter in list ([#4354](https://github.com/juspay/hyperswitch/pull/4354)) ([`53e5307`](https://github.com/juspay/hyperswitch/commit/53e5307c3cc3ae2b9f1d93d6c1e4d8e7827def7c))

### Testing

- **cypress:** Update ConnectorAuth Details ([#4386](https://github.com/juspay/hyperswitch/pull/4386)) ([`ef1914e`](https://github.com/juspay/hyperswitch/commit/ef1914ec9b75240628ad0c6367499ec063d31e3d))

**Full Changelog:** [`2024.04.18.0...2024.04.19.0`](https://github.com/juspay/hyperswitch/compare/2024.04.18.0...2024.04.19.0)

- - -

## 2024.04.18.0

### Features

- **payment_link:** Add support for saved payment method option for payment link ([#4373](https://github.com/juspay/hyperswitch/pull/4373)) ([`14341ca`](https://github.com/juspay/hyperswitch/commit/14341cad27c635d35a7804752a7dd9db4ad45503))
- **router:** Add retrieve poll status api ([#4358](https://github.com/juspay/hyperswitch/pull/4358)) ([`ca47ea9`](https://github.com/juspay/hyperswitch/commit/ca47ea9b13ff29085f7cc4e408f2b6498b1d6e8a))

### Bug Fixes

- **config:** Remove `merchant_business_country` from the connector configs as enums can not be handled in this toml file ([#4383](https://github.com/juspay/hyperswitch/pull/4383)) ([`2f59143`](https://github.com/juspay/hyperswitch/commit/2f5914392be9bb4c59c9bf5be9f5d4b6c99ef682))
- **router:** Make payment_instrument optional ([#4389](https://github.com/juspay/hyperswitch/pull/4389)) ([`450dd0f`](https://github.com/juspay/hyperswitch/commit/450dd0fe0d7c2283fbb43b7dfbe0b6214265d124))

**Full Changelog:** [`2024.04.17.0...2024.04.18.0`](https://github.com/juspay/hyperswitch/compare/2024.04.17.0...2024.04.18.0)

- - -

## 2024.04.17.0

### Features

- **payment_link:** Added display_sdk_only option for displaying only sdk without payment details ([#4363](https://github.com/juspay/hyperswitch/pull/4363)) ([`4d99098`](https://github.com/juspay/hyperswitch/commit/4d9909899f493ee28fec08846fde9737867df52b))

### Refactors

- **payment_methods:** Revamp payment methods update endpoint ([#4305](https://github.com/juspay/hyperswitch/pull/4305)) ([`3333bbf`](https://github.com/juspay/hyperswitch/commit/3333bbfe7f5af30b872809629f9942a76a823638))

**Full Changelog:** [`2024.04.16.1...2024.04.17.0`](https://github.com/juspay/hyperswitch/compare/2024.04.16.1...2024.04.17.0)

- - -

## 2024.04.16.1

### Features

- **connector:** Integrate netcetera connector with pre authentication flow ([#4293](https://github.com/juspay/hyperswitch/pull/4293)) ([`d4dbaad`](https://github.com/juspay/hyperswitch/commit/d4dbaadb06f74835235c0deb53835a8f97fa26b6))
- **mandate_kv:** Add kv support for mandate ([#4275](https://github.com/juspay/hyperswitch/pull/4275)) ([`00340a3`](https://github.com/juspay/hyperswitch/commit/00340a3369a08d93b7fe7a2c1c7ba244ee5b6248))
- **payments:** Get new filters for payments list ([#4174](https://github.com/juspay/hyperswitch/pull/4174)) ([`c3361ef`](https://github.com/juspay/hyperswitch/commit/c3361ef5ebed09b24df73221faaa6d6178fda070))
- **pm_list:** Add dynamic fields for local bank transfer ([#4349](https://github.com/juspay/hyperswitch/pull/4349)) ([`60d244c`](https://github.com/juspay/hyperswitch/commit/60d244cbe860fd13749ac8b4f6adfd85aefb8dde))
- **router:** Add external authentication webhooks flow ([#4339](https://github.com/juspay/hyperswitch/pull/4339)) ([`00cd96d`](https://github.com/juspay/hyperswitch/commit/00cd96d0979244d71abfa0a20c7a5125997c73d6))

### Bug Fixes

- **address:** Use first_name if last_name is not passed ([#4360](https://github.com/juspay/hyperswitch/pull/4360)) ([`1b7cde2`](https://github.com/juspay/hyperswitch/commit/1b7cde2d1b687e9c5ca8e3c02eef5c7d3fb7da8f))
- Added find all support for pm kv ([#4357](https://github.com/juspay/hyperswitch/pull/4357)) ([`5b811aa`](https://github.com/juspay/hyperswitch/commit/5b811aac00493f2368716265418f1c547450222c))

**Full Changelog:** [`2024.04.16.0...2024.04.16.1`](https://github.com/juspay/hyperswitch/compare/2024.04.16.0...2024.04.16.1)

- - -

## 2024.04.16.0

### Features

- **events:** Add payment cancel events ([#4166](https://github.com/juspay/hyperswitch/pull/4166)) ([`dea21c6`](https://github.com/juspay/hyperswitch/commit/dea21c65ffc872394caa39e29bcd6674d2e4f174))
- **router:** Add `merchant_business_country` field in apple pay `session_token_data` ([#4236](https://github.com/juspay/hyperswitch/pull/4236)) ([`c3c8d09`](https://github.com/juspay/hyperswitch/commit/c3c8d094531df8092c1e9b772af75b22a7c2dccb))

### Miscellaneous Tasks

- **configs:** [Zsl] Add configs for wasm ([#4346](https://github.com/juspay/hyperswitch/pull/4346)) ([`2f7faca`](https://github.com/juspay/hyperswitch/commit/2f7faca97e0ad47341e73a261fb9faff9043de13))

**Full Changelog:** [`2024.04.15.0...2024.04.16.0`](https://github.com/juspay/hyperswitch/compare/2024.04.15.0...2024.04.16.0)

- - -

## 2024.04.15.0

### Bug Fixes

- **logger:** Use specified log level only for first-party crates ([#4356](https://github.com/juspay/hyperswitch/pull/4356)) ([`b204be0`](https://github.com/juspay/hyperswitch/commit/b204be0e919d0ffd97b383e6a654982f78f9fa3d))

### Refactors

- **router:** Change stack size ([#4355](https://github.com/juspay/hyperswitch/pull/4355)) ([`4c2e972`](https://github.com/juspay/hyperswitch/commit/4c2e97273ab07917477ce016f7f04400e7e5df9a))

**Full Changelog:** [`2024.04.12.1...2024.04.15.0`](https://github.com/juspay/hyperswitch/compare/2024.04.12.1...2024.04.15.0)

- - -

## 2024.04.12.1

### Features

- **core:** Create mandates with payment_token ([#4342](https://github.com/juspay/hyperswitch/pull/4342)) ([`53697fb`](https://github.com/juspay/hyperswitch/commit/53697fb472d6e236d57aef6883a6b11a0e9232f1))
- **customer:** Customer kv impl ([#4267](https://github.com/juspay/hyperswitch/pull/4267)) ([`c980f01`](https://github.com/juspay/hyperswitch/commit/c980f016918144290ea98df2860644249c7b2e03))

### Bug Fixes

- **connector:** [ZSL] Add base_url to Environments ([#4344](https://github.com/juspay/hyperswitch/pull/4344)) ([`91830f6`](https://github.com/juspay/hyperswitch/commit/91830f6d7965f1ba9c23925a1399fdf810a7b31a))
- **payouts:** Update payout's state in app after DB operations ([#4341](https://github.com/juspay/hyperswitch/pull/4341)) ([`0fe93d6`](https://github.com/juspay/hyperswitch/commit/0fe93d65b40acf169ec333bc238505e3381f9842))
- **router:** Capture billing country in payments request ([#4347](https://github.com/juspay/hyperswitch/pull/4347)) ([`986ed2a`](https://github.com/juspay/hyperswitch/commit/986ed2a923444a38960462ec03f5e7cd8a2c249a))
- Revert payment method kv changes ([#4351](https://github.com/juspay/hyperswitch/pull/4351)) ([`bb202e3`](https://github.com/juspay/hyperswitch/commit/bb202e39bfc10cfc5ea6e15805ba28e2699284c8))

### Refactors

- **payment_methods:** Add BankTransfer payment method data to new domain type to be used in connector module ([#4260](https://github.com/juspay/hyperswitch/pull/4260)) ([`08d0811`](https://github.com/juspay/hyperswitch/commit/08d08114be0792614ce8fb990d6a9f45420cae33))

**Full Changelog:** [`2024.04.12.0...2024.04.12.1`](https://github.com/juspay/hyperswitch/compare/2024.04.12.0...2024.04.12.1)

- - -

## 2024.04.12.0

### Features

- **connector:** [ZSL] add connector template code ([#4285](https://github.com/juspay/hyperswitch/pull/4285)) ([`086516b`](https://github.com/juspay/hyperswitch/commit/086516b7b307e074b4301bd14a4c65595b6e142c))
- **events:** Add events framework for registering events ([#4115](https://github.com/juspay/hyperswitch/pull/4115)) ([`3963219`](https://github.com/juspay/hyperswitch/commit/3963219e44bd771353d754aa356097e2d78a1392))
- **payment_methods:** Added kv support for payment_methods table ([#4311](https://github.com/juspay/hyperswitch/pull/4311)) ([`eb3cecd`](https://github.com/juspay/hyperswitch/commit/eb3cecdd74b4c758948f9de82727af76b9ba9fb0))
- **payouts:** Add kafka events ([#4264](https://github.com/juspay/hyperswitch/pull/4264)) ([`a2958c3`](https://github.com/juspay/hyperswitch/commit/a2958c33b5c4ed627c97e97e791ca2cfbfcecd5e))
- **router:**
  - Add `ApiKeyAuth` support for `upsert_connector_agnostic_mandate_config` ([#4335](https://github.com/juspay/hyperswitch/pull/4335)) ([`963a10c`](https://github.com/juspay/hyperswitch/commit/963a10c877cf7e63cef2c05093cc2c3d4eab66ec))
  - Add support for accepting an existing `payment_method_id` as the `payment_method_data` in `/payments` request ([#4328](https://github.com/juspay/hyperswitch/pull/4328)) ([`92e19af`](https://github.com/juspay/hyperswitch/commit/92e19af275c615db77e3ae398bfd487529210ba4))
- **users:** Add role specific fields for list merchants API ([#4326](https://github.com/juspay/hyperswitch/pull/4326)) ([`018c5b1`](https://github.com/juspay/hyperswitch/commit/018c5b10646a68a9898c47ade4874d52250231a8))

### Bug Fixes

- **compatibility:** Generate payment_id if not sent ([#4125](https://github.com/juspay/hyperswitch/pull/4125)) ([`9448673`](https://github.com/juspay/hyperswitch/commit/9448673c1c49fe1419f47c28f59e30268b9691c5))
- **connectors:** Amount received should be zero for `pending` and `failed` status ([#4331](https://github.com/juspay/hyperswitch/pull/4331)) ([`6aa66c4`](https://github.com/juspay/hyperswitch/commit/6aa66c4243fd2a55c9df5420fd2dc85ef156561b))
- **mandate:** Add validation for currency in MIT recurring payments ([#4308](https://github.com/juspay/hyperswitch/pull/4308)) ([`07c917c`](https://github.com/juspay/hyperswitch/commit/07c917c0559da1774848d0deb95a2725fc0d6503))

### Refactors

- **card:** Use `billing.first_name` instead of `card_holder_name` ([#4239](https://github.com/juspay/hyperswitch/pull/4239)) ([`8b66cda`](https://github.com/juspay/hyperswitch/commit/8b66cdaaf384bb0d5ce986334a7b32bb3cb13581))
- **connector:** [Ebanx] Add base_url to Integ Environment ([#4332](https://github.com/juspay/hyperswitch/pull/4332)) ([`13ba3cb`](https://github.com/juspay/hyperswitch/commit/13ba3cbd9627ff53b701084d8d4c0b800793a3e3))
- **connectors:** [ZSL] add Local bank Transfer ([#4337](https://github.com/juspay/hyperswitch/pull/4337)) ([`266a075`](https://github.com/juspay/hyperswitch/commit/266a075ab653b96505a4f8f26688153ced952c8f))
- **payment_methods:**
  - Add some payment method data to new domain type to be used in connector module ([#4234](https://github.com/juspay/hyperswitch/pull/4234)) ([`ce1e165`](https://github.com/juspay/hyperswitch/commit/ce1e165cecade481ce6002795049d6a9ffec96e2))
  - Add BankDebit payment method data to new domain type to be used in connector module ([#4238](https://github.com/juspay/hyperswitch/pull/4238)) ([`2bf775a`](https://github.com/juspay/hyperswitch/commit/2bf775a97e331cde2cad3e3d2a325850d969add9))
- **router:** Add `updated` field to `PaymentsResponse` ([#4292](https://github.com/juspay/hyperswitch/pull/4292)) ([`c99e038`](https://github.com/juspay/hyperswitch/commit/c99e038a4813aa68b7a0a6ad3458c93a0e3c27ba))

### Miscellaneous Tasks

- **deps:** Update time crate to 0.3.35 ([#4338](https://github.com/juspay/hyperswitch/pull/4338)) ([`44e7456`](https://github.com/juspay/hyperswitch/commit/44e7456a1088f8c413ff3694357822328bbc29bb))

**Full Changelog:** [`2024.04.10.0...2024.04.12.0`](https://github.com/juspay/hyperswitch/compare/2024.04.10.0...2024.04.12.0)

- - -

## 2024.04.10.0

### Features

- **connector:** [Ebanx] Template for ebanx payout ([#4141](https://github.com/juspay/hyperswitch/pull/4141)) ([`ed186a5`](https://github.com/juspay/hyperswitch/commit/ed186a5a9343c1d735749eb9ec90cb0d0f6094cd))
- **router:** Add local bank transfer payment method ([#4294](https://github.com/juspay/hyperswitch/pull/4294)) ([`06440eb`](https://github.com/juspay/hyperswitch/commit/06440eb6400adf166b203d7e4f587c1e2d5fe4f8))

### Bug Fixes

- **psync:** Log the error if payment method retrieve fails in the `psync flow` ([#4321](https://github.com/juspay/hyperswitch/pull/4321)) ([`5b89209`](https://github.com/juspay/hyperswitch/commit/5b89209b6f48691ee5ae2f9ede0d913abc9105f9))

### Refactors

- **payment_methods:** Add BankRedirect payment method data to new domain type to be used in connector module ([#4175](https://github.com/juspay/hyperswitch/pull/4175)) ([`e0e8437`](https://github.com/juspay/hyperswitch/commit/e0e843715cd02ac8b2eff2f645fe8471551ee914))

**Full Changelog:** [`2024.04.08.0...2024.04.10.0`](https://github.com/juspay/hyperswitch/compare/2024.04.08.0...2024.04.10.0)

- - -

## 2024.04.08.0

### Features

- **users:** Implemented cookie parsing for auth ([#4298](https://github.com/juspay/hyperswitch/pull/4298)) ([`2d394f9`](https://github.com/juspay/hyperswitch/commit/2d394f98e96d0beafca24abe2ac9f10a05460993))

### Bug Fixes

- **locker:** Handle card duplication in payouts flow ([#4013](https://github.com/juspay/hyperswitch/pull/4013)) ([`2fac436`](https://github.com/juspay/hyperswitch/commit/2fac436683060b8e7c81b210dfdf468f5194f24c))
- **mandates:** Store network transaction id only when `pg_agnostic` config is enabled in the `authorize_flow` ([#4318](https://github.com/juspay/hyperswitch/pull/4318)) ([`7b4c4fe`](https://github.com/juspay/hyperswitch/commit/7b4c4fea332d56f81a73b496fa0fefdbb64b3648))
- **redis_interface:** Remove mget function from redis interface ([#4303](https://github.com/juspay/hyperswitch/pull/4303)) ([`14035d2`](https://github.com/juspay/hyperswitch/commit/14035d2f838d88c56fe37f78caab6c88bc8b33e4))

### Refactors

- **payment_methods:** Add PayLater payment method data to new domain type to be used in connector module ([#4165](https://github.com/juspay/hyperswitch/pull/4165)) ([`6694852`](https://github.com/juspay/hyperswitch/commit/669485275db192b0e8e30f3528c0d61150d91847))

**Full Changelog:** [`2024.04.05.0...2024.04.08.0`](https://github.com/juspay/hyperswitch/compare/2024.04.05.0...2024.04.08.0)

- - -

## 2024.04.05.0

### Features

- **payout-events:** Add kafka events for payout analytics ([#4211](https://github.com/juspay/hyperswitch/pull/4211)) ([`bc25f3f`](https://github.com/juspay/hyperswitch/commit/bc25f3fa40e807cc92d2d53a2287b92eff727d3c))
- **router:**
  - Store `network_reference_id` against the `payment_method_id` in the `payment_method_table` ([#4041](https://github.com/juspay/hyperswitch/pull/4041)) ([`21e2d78`](https://github.com/juspay/hyperswitch/commit/21e2d78117a9e25708b8c6a2280f6a836ee86072))
  - Use `NTID` in `MIT` payments if the `pg_agnostic_mit` config is enabled ([#4113](https://github.com/juspay/hyperswitch/pull/4113)) ([`b58d7a8`](https://github.com/juspay/hyperswitch/commit/b58d7a8e62eef9880f717731063101bf92af3f34))
  - Add NTID flow for cybersource ([#4193](https://github.com/juspay/hyperswitch/pull/4193)) ([`071462f`](https://github.com/juspay/hyperswitch/commit/071462f2af8efeb16e48d351bbae68fd2fd64179))
- **webhooks:** Allow manually retrying delivery of outgoing webhooks ([#4176](https://github.com/juspay/hyperswitch/pull/4176)) ([`63d2b68`](https://github.com/juspay/hyperswitch/commit/63d2b6855acee1adeae2efff10f424e056af0bcb))

### Bug Fixes

- **payouts:** Persist status updates in payouts table ([#4280](https://github.com/juspay/hyperswitch/pull/4280)) ([`02ffe7e`](https://github.com/juspay/hyperswitch/commit/02ffe7e48068a43d319d67e0e976420d201776db))

### Refactors

- **connector:**
  - [Multisafepay] handle authorize and psync 2xx failure error response ([#4124](https://github.com/juspay/hyperswitch/pull/4124)) ([`9ebe0f4`](https://github.com/juspay/hyperswitch/commit/9ebe0f4371f13c7527972242424af2d926c84b5e))
  - Add support for GooglePay recurring payments ([#4300](https://github.com/juspay/hyperswitch/pull/4300)) ([`622aac3`](https://github.com/juspay/hyperswitch/commit/622aac3015e95de55e83abd047b5c680ecd8d662))
- **core:** Log the appropriate error message if the card fails to get saved in locker ([#4296](https://github.com/juspay/hyperswitch/pull/4296)) ([`9de3cdb`](https://github.com/juspay/hyperswitch/commit/9de3cdb7d37dd1d18c6a84368e70ceb52b7ae53a))
- **payment_link:** Decouple shimmer css from main payment_link css for better performance ([#4286](https://github.com/juspay/hyperswitch/pull/4286)) ([`9453e8f`](https://github.com/juspay/hyperswitch/commit/9453e8fcfac49fc399343ee7c4c1598412b370c7))

**Full Changelog:** [`2024.04.04.0...2024.04.05.0`](https://github.com/juspay/hyperswitch/compare/2024.04.04.0...2024.04.05.0)

- - -

## 2024.04.04.0

### Features

- **api:** Add browser information in payments response ([#3963](https://github.com/juspay/hyperswitch/pull/3963)) ([`4051cbb`](https://github.com/juspay/hyperswitch/commit/4051cbb4e7f708267b26439061e001bb00342cad))
- **core:** Update connector_mandate_details in payment_method ([#4155](https://github.com/juspay/hyperswitch/pull/4155)) ([`d8028ce`](https://github.com/juspay/hyperswitch/commit/d8028cefd53219ce15ba31ff3ea5ada3c0e217e7))
- **cypress:** Add cypress test cases ([#4271](https://github.com/juspay/hyperswitch/pull/4271)) ([`06e30e0`](https://github.com/juspay/hyperswitch/commit/06e30e04b06779862dd493ecaa7285875ffb402b))
- **router:** Create a merchant config for enable processor agnostic MIT ([#4025](https://github.com/juspay/hyperswitch/pull/4025)) ([`2a691a5`](https://github.com/juspay/hyperswitch/commit/2a691a5c05573b0a9caa8b2d7e57bc90c49280fe))

### Refactors

- **connector:** [Stripe] fix mandate flow ([#4281](https://github.com/juspay/hyperswitch/pull/4281)) ([`ea706f8`](https://github.com/juspay/hyperswitch/commit/ea706f81debd13a44a49ac3d1d3ef7f1882b683b))
- **core:** Locker call made synchronous for updation of pm_id ([#4289](https://github.com/juspay/hyperswitch/pull/4289)) ([`6e94a56`](https://github.com/juspay/hyperswitch/commit/6e94a5636462a8071e69f072ec058c6068e5d1f7))
- **mandates:** Add validations for recurring mandates using payment_method_id ([#4263](https://github.com/juspay/hyperswitch/pull/4263)) ([`49cfe72`](https://github.com/juspay/hyperswitch/commit/49cfe72cd2a20ba25c3323fca81bba7ea48b591b))
- **payment_methods:**
  - Add Wallets payment method data to new domain type to be used in connector module ([#4160](https://github.com/juspay/hyperswitch/pull/4160)) ([`8efd468`](https://github.com/juspay/hyperswitch/commit/8efd468ac150ff8d28f5b44b25701ba1837f243d))
  - Add `network_transaction_id` column in the `payment_methods` table ([#4005](https://github.com/juspay/hyperswitch/pull/4005)) ([`179f5ff`](https://github.com/juspay/hyperswitch/commit/179f5ff052aa530f2b429aaf20ea326bdc7f7ce0))
- **payout:** Handle saving wallet in temp locker ([#4230](https://github.com/juspay/hyperswitch/pull/4230)) ([`ae37b05`](https://github.com/juspay/hyperswitch/commit/ae37b059e09d9e6b597914536359fbdd5dd777d2))
- Fix typos ([#4277](https://github.com/juspay/hyperswitch/pull/4277)) ([`36f4112`](https://github.com/juspay/hyperswitch/commit/36f4112a6fa07c53a0b0c101539e4bf36d18893f))
- Fix typos in stripe transformers ([#4287](https://github.com/juspay/hyperswitch/pull/4287)) ([`4445a86`](https://github.com/juspay/hyperswitch/commit/4445a86207a31fd84553c70959a1341143759bc3))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`70eb294`](https://github.com/juspay/hyperswitch/commit/70eb2940ecef503cbf9a898d0f8382f9abe83057))

**Full Changelog:** [`2024.04.03.0...2024.04.04.0`](https://github.com/juspay/hyperswitch/compare/2024.04.03.0...2024.04.04.0)

- - -

## 2024.04.03.0

### Features

- **analytics:** Three_ds and authentication events in sdkevents ([#4251](https://github.com/juspay/hyperswitch/pull/4251)) ([`88b53b0`](https://github.com/juspay/hyperswitch/commit/88b53b0d5ccfb16b03fc17c453f2c7afa26ec92e))
- **payment_link:** Add payment info metadata to payment link ([#4270](https://github.com/juspay/hyperswitch/pull/4270)) ([`97fbc89`](https://github.com/juspay/hyperswitch/commit/97fbc899c12a0c66ac89a7feaa6d45d39239a746))
- **router:** [BOA] implement mandates for cards and wallets ([#4232](https://github.com/juspay/hyperswitch/pull/4232)) ([`2f304e6`](https://github.com/juspay/hyperswitch/commit/2f304e601607980e7e536d94411ddf0f9023c605))

### Bug Fixes

- **connector:** [Cryptopay]fix redirection for cryptopay ([#4272](https://github.com/juspay/hyperswitch/pull/4272)) ([`1023f46`](https://github.com/juspay/hyperswitch/commit/1023f46c885dc2b70ccbb3931e667740695f448e))

### Refactors

- **payment_methods:** Add a new domain type for payment method data to be used in connector module ([#4140](https://github.com/juspay/hyperswitch/pull/4140)) ([`9cce152`](https://github.com/juspay/hyperswitch/commit/9cce1520e3b0c7c1d1ae70ca8cc30787ff96dded))
- **postman:** Paypal test cases for Capture ([#4265](https://github.com/juspay/hyperswitch/pull/4265)) ([`a071463`](https://github.com/juspay/hyperswitch/commit/a071463b29f9794e7069d57057d3bcc3a238f89b))

### Build System / Dependencies

- **deps:** Update dependencies ([#4268](https://github.com/juspay/hyperswitch/pull/4268)) ([`1f0d60e`](https://github.com/juspay/hyperswitch/commit/1f0d60e64fc9379d8a07a0c72970afc7b491dafa))

**Full Changelog:** [`2024.04.02.0...2024.04.03.0`](https://github.com/juspay/hyperswitch/compare/2024.04.02.0...2024.04.03.0)

- - -

## 2024.04.02.0

### Features

- **connector:** [billwerk] implement payment and refund flows ([#4245](https://github.com/juspay/hyperswitch/pull/4245)) ([`aecf4ae`](https://github.com/juspay/hyperswitch/commit/aecf4aeacce33c3dc03e089ef6d62af93e29ca9a))
- Return customer details in payments response body ([#4237](https://github.com/juspay/hyperswitch/pull/4237)) ([`740749e`](https://github.com/juspay/hyperswitch/commit/740749e18ae4458726cdf2501f3d3b789c819f7a))

### Refactors

- **core:** Removed the processing status for payment_method_status ([#4213](https://github.com/juspay/hyperswitch/pull/4213)) ([`a843713`](https://github.com/juspay/hyperswitch/commit/a843713126cea102064b342b6fc82429d89d758a))

### Documentation

- **README:** Remove link to outdated early access form ([`78befb4`](https://github.com/juspay/hyperswitch/commit/78befb42a35b1f98b1bd47b253d3c06e50bb2ee4))

### Build System / Dependencies

- **deps:** Bump `error-stack` from version `0.3.1` to `0.4.1` ([#4188](https://github.com/juspay/hyperswitch/pull/4188)) ([`ea730d4`](https://github.com/juspay/hyperswitch/commit/ea730d4ffc712cdf264492db109836fcde9b2b03))

**Full Changelog:** [`2024.04.01.0...2024.04.02.0`](https://github.com/juspay/hyperswitch/compare/2024.04.01.0...2024.04.02.0)

- - -

## 2024.04.01.0

### Features

- **mandates:** Allow off-session payments using `payment_method_id` ([#4132](https://github.com/juspay/hyperswitch/pull/4132)) ([`7b337ac`](https://github.com/juspay/hyperswitch/commit/7b337ac39d72f90dd0ebe58133218896ff279313))
- **payment_method:** API to list countries and currencies supported by a country and payment method type ([#4126](https://github.com/juspay/hyperswitch/pull/4126)) ([`74cd4a7`](https://github.com/juspay/hyperswitch/commit/74cd4a79526eb1a2dead87855e6a39248ec5ccb7))

### Miscellaneous Tasks

- **config:** Add billwerk base URL in deployment configs ([#4243](https://github.com/juspay/hyperswitch/pull/4243)) ([`e8289f0`](https://github.com/juspay/hyperswitch/commit/e8289f061d4735478cb1521de50f696d2412ad33))

**Full Changelog:** [`2024.03.28.0...2024.04.01.0`](https://github.com/juspay/hyperswitch/compare/2024.03.28.0...2024.04.01.0)

- - -

## 2024.03.28.0

### Features

- **connector:** [billwerk] add connector template code ([#4123](https://github.com/juspay/hyperswitch/pull/4123)) ([`37be05d`](https://github.com/juspay/hyperswitch/commit/37be05d31d97651ddaa2c67b828d24563b35d37e))

### Bug Fixes

- **connectors:** Fix wallet token deserialization error ([#4133](https://github.com/juspay/hyperswitch/pull/4133)) ([`929848f`](https://github.com/juspay/hyperswitch/commit/929848f8713b45daf479ba24fb0a49b8e327b6fd))
- **core:** Amount capturable remain same for `processing` status in capture ([#4229](https://github.com/juspay/hyperswitch/pull/4229)) ([`9523cf4`](https://github.com/juspay/hyperswitch/commit/9523cf4bbac488503c31640cade326095937e33c))
- **euclid_wasm:** Checkout wasm metadata issue ([#4198](https://github.com/juspay/hyperswitch/pull/4198)) ([`246898f`](https://github.com/juspay/hyperswitch/commit/246898fbb00a67d5791827527ce45e01b01b232c))
- **log:** Adding span metadata to `tokio` spawned futures ([#4118](https://github.com/juspay/hyperswitch/pull/4118)) ([`0706221`](https://github.com/juspay/hyperswitch/commit/070622125f49c4cc9c35f5ba9c634f1fef6b26d2))
- **trustpay:** [Trustpay] Add error code mapping '800.100.100' ([#4224](https://github.com/juspay/hyperswitch/pull/4224)) ([`9798db4`](https://github.com/juspay/hyperswitch/commit/9798db4558d926a218a0ca6f7f7c4e24a187b3da))

### Refactors

- **config:** Allow wildcard origin for development and Docker Compose setups ([#4231](https://github.com/juspay/hyperswitch/pull/4231)) ([`6587472`](https://github.com/juspay/hyperswitch/commit/65874728094bb550d6c311965fbb5f1577091bbb))

**Full Changelog:** [`2024.03.27.0...2024.03.28.0`](https://github.com/juspay/hyperswitch/compare/2024.03.27.0...2024.03.28.0)

- - -

## 2024.03.27.0

### Bug Fixes

- **connector:**
  - [Trustpay] fix deserialization error for incoming webhook response for trustpay and add error code mapping '800.100.203' ([#4199](https://github.com/juspay/hyperswitch/pull/4199)) ([`84bef25`](https://github.com/juspay/hyperswitch/commit/84bef251480a77027b43c3dc91353a0cb40d5ff1))
  - [CRYPTOPAY] Skip metadata serialization if none ([#4205](https://github.com/juspay/hyperswitch/pull/4205)) ([`0429399`](https://github.com/juspay/hyperswitch/commit/0429399c29f76c97bf2096bbe9e9b429c025e56b))
- **core:** Make eci in AuthenticationData optional ([#4187](https://github.com/juspay/hyperswitch/pull/4187)) ([`4f0c788`](https://github.com/juspay/hyperswitch/commit/4f0c788cf26907e2be784978c412081a93386d04))

**Full Changelog:** [`2024.03.26.0...2024.03.27.0`](https://github.com/juspay/hyperswitch/compare/2024.03.26.0...2024.03.27.0)

- - -

## 2024.03.26.0

### Features

- **events:** Allow listing webhook events and webhook delivery attempts by business profile ([#4159](https://github.com/juspay/hyperswitch/pull/4159)) ([`4c8cdf1`](https://github.com/juspay/hyperswitch/commit/4c8cdf1475ac74fb2df5bea419dfa7657f26f298))
- **payouts:** Add user roles for payouts ([#4167](https://github.com/juspay/hyperswitch/pull/4167)) ([`13fe584`](https://github.com/juspay/hyperswitch/commit/13fe58450bad094fb2b4745ecf76bc2df8b96798))

### Miscellaneous Tasks

- Address Rust 1.77 clippy lints ([#4172](https://github.com/juspay/hyperswitch/pull/4172)) ([`f213c51`](https://github.com/juspay/hyperswitch/commit/f213c51b3e5c4f0b3546b35bac4dde9698818e01))

**Full Changelog:** [`2024.03.22.0...2024.03.26.0`](https://github.com/juspay/hyperswitch/compare/2024.03.22.0...2024.03.26.0)

- - -

## 2024.03.22.0

### Features

- **events:** Add APIs to list webhook events and webhook delivery attempts ([#4131](https://github.com/juspay/hyperswitch/pull/4131)) ([`14e1bba`](https://github.com/juspay/hyperswitch/commit/14e1bbaf071d1178f91124fe85580f178cb1cf96))
- **global-search-regex-escape:** Escape reserved characters which break global search query ([#4135](https://github.com/juspay/hyperswitch/pull/4135)) ([`4f8461b`](https://github.com/juspay/hyperswitch/commit/4f8461b2a949fd2a6d24b8b42f1bf8bab55cfeeb))

### Miscellaneous Tasks

- Update Slack workspace URL ([#4168](https://github.com/juspay/hyperswitch/pull/4168)) ([`75b4bac`](https://github.com/juspay/hyperswitch/commit/75b4bacc984d11cb755a8c36821ec41d3f1e2187))

**Full Changelog:** [`2024.03.21.1...2024.03.22.0`](https://github.com/juspay/hyperswitch/compare/2024.03.21.1...2024.03.22.0)

- - -

## 2024.03.21.1

### Features

- **payouts:**
  - Implement list and filter APIs ([#3651](https://github.com/juspay/hyperswitch/pull/3651)) ([`fb5f0e6`](https://github.com/juspay/hyperswitch/commit/fb5f0e6c7eb7255ac423ed4385613e9a78227c77))
  - Add payout types in euclid crate ([#3862](https://github.com/juspay/hyperswitch/pull/3862)) ([`a151485`](https://github.com/juspay/hyperswitch/commit/a1514853176e6cdac73e69d90165416613c97d70))

### Bug Fixes

- **router:** Handle redirection to return_url from nested iframe in separate 3ds flow ([#4164](https://github.com/juspay/hyperswitch/pull/4164)) ([`b8c9275`](https://github.com/juspay/hyperswitch/commit/b8c927593a85792588e582bf25f2daadfa5f7fb0))

**Full Changelog:** [`2024.03.21.0...2024.03.21.1`](https://github.com/juspay/hyperswitch/compare/2024.03.21.0...2024.03.21.1)

- - -

## 2024.03.21.0

### Features

- Store payment check codes and authentication data from processors ([#3958](https://github.com/juspay/hyperswitch/pull/3958)) ([`7afc44e`](https://github.com/juspay/hyperswitch/commit/7afc44e8357b09c900a1e9aa384619f93f3bc81d))

### Bug Fixes

- **payment_methods:**
  - Update payment method status only if existing status is not active ([#4149](https://github.com/juspay/hyperswitch/pull/4149)) ([`0e9b252`](https://github.com/juspay/hyperswitch/commit/0e9b2524cf22a220abeb604dd172aa00855a7ee6))
  - Make `ApplepayPaymentMethod` in payment_method_data column of `payment_attempt` table as json ([#4154](https://github.com/juspay/hyperswitch/pull/4154)) ([`7c0e4c7`](https://github.com/juspay/hyperswitch/commit/7c0e4c7229acacbeb93102bcdc25b74fd7a3314c))

### Refactors

- **connector:** [Stripe] update stripe-api-version in API-headers ([#4120](https://github.com/juspay/hyperswitch/pull/4120)) ([`3653c2c`](https://github.com/juspay/hyperswitch/commit/3653c2c108b80a20df6e8a2bf980d48c204376cd))
- **payment_method_data:** Add a trait to retrieve billing from payment method data ([#4095](https://github.com/juspay/hyperswitch/pull/4095)) ([`9b9bce8`](https://github.com/juspay/hyperswitch/commit/9b9bce80a6419abdd5318d993f1abd6598853dd3))

### Build System / Dependencies

- **router_env:** Obtain workspace member package names from `cargo_metadata` more deterministically ([#4139](https://github.com/juspay/hyperswitch/pull/4139)) ([`8f7d9fb`](https://github.com/juspay/hyperswitch/commit/8f7d9fbc3a002127e220d8a968a6a4e15796e2fd))

**Full Changelog:** [`2024.03.20.0...2024.03.21.0`](https://github.com/juspay/hyperswitch/compare/2024.03.20.0...2024.03.21.0)

- - -

## 2024.03.20.0

### Features

- **global-search:** Add dispute events index to global-search ([#4068](https://github.com/juspay/hyperswitch/pull/4068)) ([`9345379`](https://github.com/juspay/hyperswitch/commit/9345379f85a5da786c8f733542d796da567b6ffc))
- **payouts:** Implement KVRouterStore ([#3889](https://github.com/juspay/hyperswitch/pull/3889)) ([`944089d`](https://github.com/juspay/hyperswitch/commit/944089d6914cb6bece9056f78b9aabf90e485151))
- **router:**
  - Add offset in mandate list route ([#3923](https://github.com/juspay/hyperswitch/pull/3923)) ([`17a866a`](https://github.com/juspay/hyperswitch/commit/17a866a73541c2340547c67e47b60f813c53f744))
  - Handle redirection to return_url from iframe for separate 3ds flow ([#4119](https://github.com/juspay/hyperswitch/pull/4119)) ([`3eb4642`](https://github.com/juspay/hyperswitch/commit/3eb464250e5d604d90a99d61d1c9d6115252f0ef))

### Refactors

- **connector:** [Stripe] make name field of StripeShippingAddress mandatory ([#4111](https://github.com/juspay/hyperswitch/pull/4111)) ([`ab1ec2a`](https://github.com/juspay/hyperswitch/commit/ab1ec2ad4e3f1197d08c5ff947c31e7f0fcf5c65))
- **core:** Move authentication data fields to authentication table ([#4093](https://github.com/juspay/hyperswitch/pull/4093)) ([`a3dec0b`](https://github.com/juspay/hyperswitch/commit/a3dec0b6bc52f20246a65ed5255768fcf585147a))

**Full Changelog:** [`2024.03.19.0...2024.03.20.0`](https://github.com/juspay/hyperswitch/compare/2024.03.19.0...2024.03.20.0)

- - -

## 2024.03.19.0

### Features

- **events:** Add audit events scaffolding ([#3863](https://github.com/juspay/hyperswitch/pull/3863)) ([`6f67985`](https://github.com/juspay/hyperswitch/commit/6f679851dfaca8690fa3c5c2d1a2978bfe6d42b6))

### Bug Fixes

- **payments:** Populate merchant connector id and profile id in list ([#4104](https://github.com/juspay/hyperswitch/pull/4104)) ([`1dac028`](https://github.com/juspay/hyperswitch/commit/1dac0286bb402fec5e4ac3270112fcb7c3f35cd6))

### Refactors

- **connector:**
  - [Coinbase][Cryptopay] Mask PII data ([#3936](https://github.com/juspay/hyperswitch/pull/3936)) ([`8eb31f9`](https://github.com/juspay/hyperswitch/commit/8eb31f94f4aae5daa41799dc78eb3a116653aa0d))
  - [Prophetpay][Rapyd][Shift4][Square] Mask PII data ([#3930](https://github.com/juspay/hyperswitch/pull/3930)) ([`b1face6`](https://github.com/juspay/hyperswitch/commit/b1face64424cd68f0a21192b981698abaec05ec3))
  - [Worldline][Worldpay][Zen] Mask PII data ([#3935](https://github.com/juspay/hyperswitch/pull/3935)) ([`612d2b1`](https://github.com/juspay/hyperswitch/commit/612d2b17e233985b70e72fa5c12164f57dede0ee))
  - [Adyen] change error message from not supported to not implemented ([#2845](https://github.com/juspay/hyperswitch/pull/2845)) ([`c3ef599`](https://github.com/juspay/hyperswitch/commit/c3ef599ad736dee34286150ec6bf5143a526ae6c))
  - [Aci] remove default case handling ([#2513](https://github.com/juspay/hyperswitch/pull/2513)) ([`7398371`](https://github.com/juspay/hyperswitch/commit/73983710a068ff04da54152d1a1a84639f961622))
  - [Klarna] Mask PII data ([#3854](https://github.com/juspay/hyperswitch/pull/3854)) ([`384f32b`](https://github.com/juspay/hyperswitch/commit/384f32ba2d8fcd57eba6c06cf49c0fa08fb21c81))
- **payment_link:** Make performance optimisation for payment_link ([#4092](https://github.com/juspay/hyperswitch/pull/4092)) ([`fcfd567`](https://github.com/juspay/hyperswitch/commit/fcfd567bfe55747dcb05c88def96373a707f8c78))
- **router:** Add FE error logs to loki ([#4077](https://github.com/juspay/hyperswitch/pull/4077)) ([`6149d4f`](https://github.com/juspay/hyperswitch/commit/6149d4fb607304ccdf184c8c5f28269a45ef3974))
- **stripe:** Change NotSupported to NotImplemented error for Stripe ([#3690](https://github.com/juspay/hyperswitch/pull/3690)) ([`6ff8f75`](https://github.com/juspay/hyperswitch/commit/6ff8f75d6cd58e36882877e5194da6a160a88f9e))

### Miscellaneous Tasks

- **config:** Add wasm changes for checkout connector to support external authentication flow ([#4096](https://github.com/juspay/hyperswitch/pull/4096)) ([`ce5cbfb`](https://github.com/juspay/hyperswitch/commit/ce5cbfbda6d1d75790eba6ae68d401e8817aba55))

**Full Changelog:** [`2024.03.18.0...2024.03.19.0`](https://github.com/juspay/hyperswitch/compare/2024.03.18.0...2024.03.19.0)

- - -

## 2024.03.18.0

### Features

- **connector:**
  - [Paypal] Unify error code and error message in Paypal ([#2354](https://github.com/juspay/hyperswitch/pull/2354)) ([`fc81f90`](https://github.com/juspay/hyperswitch/commit/fc81f90f6168dc6e08cbfacdda0f59e99def07da))
  - [BOA/CYB] Add support for payment status ACCEPTED and CANCELLED ([#4107](https://github.com/juspay/hyperswitch/pull/4107)) ([`c52dbd6`](https://github.com/juspay/hyperswitch/commit/c52dbd6fc21c9c16ebc8f2abed1d2979bc5a606b))
- **pm_auth:** Support different pm types in PM auth ([#3114](https://github.com/juspay/hyperswitch/pull/3114)) ([`290c456`](https://github.com/juspay/hyperswitch/commit/290c456a235072ac5a5b900c11ca8a4fa1a3b9e4))

### Bug Fixes

- **api_response:** Ghost payment_method_billing being populated in the response ([#4085](https://github.com/juspay/hyperswitch/pull/4085)) ([`3d4baa2`](https://github.com/juspay/hyperswitch/commit/3d4baa230cdfa0e4e0f0ab36f3ca8c96e9b705ad))

### Refactors

- **connector:**
  - [Wise] Response Fields made Optional ([#4007](https://github.com/juspay/hyperswitch/pull/4007)) ([`8c103c0`](https://github.com/juspay/hyperswitch/commit/8c103c0f8e838a899a0c1207c88fa7617b37f138))
  - [Stripe] add stripe-api-version in API-headers ([#4109](https://github.com/juspay/hyperswitch/pull/4109)) ([`ed6fdad`](https://github.com/juspay/hyperswitch/commit/ed6fdad73757425d9419575ce6aba80fae8daf4d))
  - [Payu][Placetopay][PowerTranz] Mask PII data ([#3928](https://github.com/juspay/hyperswitch/pull/3928)) ([`4cbd00b`](https://github.com/juspay/hyperswitch/commit/4cbd00ba410c093878a6e7bc4b3cb76941a57351))
  - [NMI] Mask PII data ([#3876](https://github.com/juspay/hyperswitch/pull/3876)) ([`bbf20c5`](https://github.com/juspay/hyperswitch/commit/bbf20c5b155c003bcef91880653f87c9dedc928f))
- **core:** Remove pament_method_status from payment_data ([#4061](https://github.com/juspay/hyperswitch/pull/4061)) ([`0f6c97c`](https://github.com/juspay/hyperswitch/commit/0f6c97c47ddd0980ace13840faadc4b6eefaa48e))

**Full Changelog:** [`2024.03.15.0...2024.03.18.0`](https://github.com/juspay/hyperswitch/compare/2024.03.15.0...2024.03.18.0)

- - -

## 2024.03.15.0

### Features

- **connector:** [cybersource] add card holder name in dynamic fields ([#4082](https://github.com/juspay/hyperswitch/pull/4082)) ([`5185d65`](https://github.com/juspay/hyperswitch/commit/5185d65ef5d48c21b203250cbc310b94212511c9))
- **webhooks:** Store request and response payloads in `events` table ([#4029](https://github.com/juspay/hyperswitch/pull/4029)) ([`fd67a6c`](https://github.com/juspay/hyperswitch/commit/fd67a6c2255b866ca20823e25c4a2a6fa3304fa7))

### Bug Fixes

- **connector:** [Iatapay] remove unused fields from auth response ([#4091](https://github.com/juspay/hyperswitch/pull/4091)) ([`e5b7bc6`](https://github.com/juspay/hyperswitch/commit/e5b7bc62fbfee7c1e6631b4d38fef5859dd736c1))

### Refactors

- **payment_methods:** Enable country currency filter for cards ([#4056](https://github.com/juspay/hyperswitch/pull/4056)) ([`9ae10dc`](https://github.com/juspay/hyperswitch/commit/9ae10dc4d050f3aa705c72b27e676cdcb0e379c4))
- **router:** Add IO level application logs ([#4042](https://github.com/juspay/hyperswitch/pull/4042)) ([`ad17cc7`](https://github.com/juspay/hyperswitch/commit/ad17cc738372e7397d73d6f55cae56beafa4e849))

### Miscellaneous Tasks

- **config:** [AUTHORIZEDOTNET] Add apple pay manual flow to dashboard ([#4080](https://github.com/juspay/hyperswitch/pull/4080)) ([`59a2bc4`](https://github.com/juspay/hyperswitch/commit/59a2bc434dca0d9faeceaa42b965f4ba4e93b1a9))

**Full Changelog:** [`2024.03.13.3...2024.03.15.0`](https://github.com/juspay/hyperswitch/compare/2024.03.13.3...2024.03.15.0)

- - -

## 2024.03.13.3

### Bug Fixes

- **mandates:** Give higher precedence to connector mandate id over network txn id in mandates ([#4073](https://github.com/juspay/hyperswitch/pull/4073)) ([`d28e415`](https://github.com/juspay/hyperswitch/commit/d28e415dc289a58468e24c15bbaf7fc15b4a91ee))
- Get valid test cards list based on wasm feature config ([#4066](https://github.com/juspay/hyperswitch/pull/4066)) ([`fad23ad`](https://github.com/juspay/hyperswitch/commit/fad23ad032971497b07035c530397539413b7653))

**Full Changelog:** [`2024.03.13.2...2024.03.13.3`](https://github.com/juspay/hyperswitch/compare/2024.03.13.2...2024.03.13.3)

- - -

## 2024.03.13.2

### Bug Fixes

- **connector:** [cybersource] update mandate condition ([#4048](https://github.com/juspay/hyperswitch/pull/4048)) ([`d82960c`](https://github.com/juspay/hyperswitch/commit/d82960c1cca5ae43d1a51f8fff6f7b6b9e016c2b))
- **payment_methods:** Set requires-cvv to false for cards in customer payment methods list if making an off-session payment ([#4075](https://github.com/juspay/hyperswitch/pull/4075)) ([`db25dac`](https://github.com/juspay/hyperswitch/commit/db25dac5c0023ff49a839e7914a639403c733e8a))

**Full Changelog:** [`2024.03.13.1...2024.03.13.2`](https://github.com/juspay/hyperswitch/compare/2024.03.13.1...2024.03.13.2)

- - -

## 2024.03.13.1

### Bug Fixes

- **router:** Fix token fetch logic in complete authorize flow for three ds payments ([#4052](https://github.com/juspay/hyperswitch/pull/4052)) ([`ada0002`](https://github.com/juspay/hyperswitch/commit/ada000245522662c36032034a76c3e8b57152582))

**Full Changelog:** [`2024.03.13.0...2024.03.13.1`](https://github.com/juspay/hyperswitch/compare/2024.03.13.0...2024.03.13.1)

- - -

## 2024.03.13.0

### Features

- **connector:** [AUTHORIZEDOTNET] Audit Connector ([#4035](https://github.com/juspay/hyperswitch/pull/4035)) ([`7840bdb`](https://github.com/juspay/hyperswitch/commit/7840bdb95f90065f3f6d671b07c3044e77740ed2))
- **core:** Confirm flow and authorization api changes for external authentication ([#4015](https://github.com/juspay/hyperswitch/pull/4015)) ([`ce3625c`](https://github.com/juspay/hyperswitch/commit/ce3625cb0cdccc750a073c012f0e541b014c3190))
- **global-search:** Dashboard globalsearch apis ([#3831](https://github.com/juspay/hyperswitch/pull/3831)) ([`ac8ddd4`](https://github.com/juspay/hyperswitch/commit/ac8ddd40208f3da5f65ca97bf5033cea5ca3ebe3))

### Bug Fixes

- **connector:** [Adyen] update config and add required fields ([#4046](https://github.com/juspay/hyperswitch/pull/4046)) ([`16d73cb`](https://github.com/juspay/hyperswitch/commit/16d73cb5f9f469d791f8880f3a2fd79135c821cd))
- **core:** [REFUNDS] Fix Not Supported Connector Error ([#4045](https://github.com/juspay/hyperswitch/pull/4045)) ([`7513423`](https://github.com/juspay/hyperswitch/commit/7513423631ddf0fe86ef656ec6cad76d82c807bc))

### Refactors

- **address:** Pass payment method billing to the connector module ([#3828](https://github.com/juspay/hyperswitch/pull/3828)) ([`195c700`](https://github.com/juspay/hyperswitch/commit/195c700e6c88e457cecc0722a7e5990db1379f22))
- **connector:** [Checkout] remove Paypal from wasm ([#4044](https://github.com/juspay/hyperswitch/pull/4044)) ([`3eff4eb`](https://github.com/juspay/hyperswitch/commit/3eff4ebd3a60b5831cbec0158527475c8f7d7eb4))
- **openai:** Update open-api spec to have payment changes ([#4043](https://github.com/juspay/hyperswitch/pull/4043)) ([`708cce9`](https://github.com/juspay/hyperswitch/commit/708cce926125a29b406db48cf0ebd35b217927d4))
- **payment_methods:**
  - Filter wallet payment method from mca based on customer pm ([#4038](https://github.com/juspay/hyperswitch/pull/4038)) ([`abe9c2a`](https://github.com/juspay/hyperswitch/commit/abe9c2ac17a0783f3625dd7fde5d28e285012ec3))
  - Allow deletion of default payment method for a customer if only one pm exists ([#4027](https://github.com/juspay/hyperswitch/pull/4027)) ([`45ed56f`](https://github.com/juspay/hyperswitch/commit/45ed56f16516c44acbe75b75c0621b78ccdb9894))
- [Checkout] change payment and webhooks API contract ([#4023](https://github.com/juspay/hyperswitch/pull/4023)) ([`733a560`](https://github.com/juspay/hyperswitch/commit/733a560146bb06e51fa4ee7ed9b6d1d3d9eddf12))

**Full Changelog:** [`2024.03.12.0...2024.03.13.0`](https://github.com/juspay/hyperswitch/compare/2024.03.12.0...2024.03.13.0)

- - -

## 2024.03.12.0

### Refactors

- **core:** Status handling for payment_method_status ([#3965](https://github.com/juspay/hyperswitch/pull/3965)) ([`e87f2ea`](https://github.com/juspay/hyperswitch/commit/e87f2ea8c5669473940df8bc2f5c61fdf3f218ff))

### Miscellaneous Tasks

- Add threedsecureio base url in deployment config files ([#4039](https://github.com/juspay/hyperswitch/pull/4039)) ([`d9f8423`](https://github.com/juspay/hyperswitch/commit/d9f84232a4a29814a1f9a792ebc74923862a1da6))

**Full Changelog:** [`2024.03.11.1...2024.03.12.0`](https://github.com/juspay/hyperswitch/compare/2024.03.11.1...2024.03.12.0)

- - -

## 2024.03.11.1

### Features

- **router:** Add routing support for token-based mit payments ([#4012](https://github.com/juspay/hyperswitch/pull/4012)) ([`43ebfbc`](https://github.com/juspay/hyperswitch/commit/43ebfbc47f03eaaaf274847290861dcb00db26a5))
- **users:** Implemented Set-Cookie ([#3865](https://github.com/juspay/hyperswitch/pull/3865)) ([`44eef46`](https://github.com/juspay/hyperswitch/commit/44eef46e5d7f0a198be80602ceae1c843449319c))

### Refactors

- **connector:**
  - [Multisafepay] Mask PII data ([#3869](https://github.com/juspay/hyperswitch/pull/3869)) ([`c2b1561`](https://github.com/juspay/hyperswitch/commit/c2b15615e3c61e6f497180be8fa66d008ed150bb))
  - [Globalpay] Mask PII data ([#3840](https://github.com/juspay/hyperswitch/pull/3840)) ([`13f6d6c`](https://github.com/juspay/hyperswitch/commit/13f6d6c10ce421329a7eb8b494fbb3bd31aed91f))
  - [Iatapay] Mask PII data ([#3850](https://github.com/juspay/hyperswitch/pull/3850)) ([`bd7accb`](https://github.com/juspay/hyperswitch/commit/bd7accb2c250b5f330b6bbb87f6f6edf4a479a61))
  - [Payme][Payeezy] Mask PII data ([#3926](https://github.com/juspay/hyperswitch/pull/3926)) ([`ffcb2bc`](https://github.com/juspay/hyperswitch/commit/ffcb2bcf2b7a26d8fc7fc45f9878d41ba74d2fe0))
  - [Nexinets] Mask PII data ([#3874](https://github.com/juspay/hyperswitch/pull/3874)) ([`9ea5310`](https://github.com/juspay/hyperswitch/commit/9ea531068d87b76e8f41ee7d9e9d26fd755bced4))
  - [Noon] Mask PII data ([#3879](https://github.com/juspay/hyperswitch/pull/3879)) ([`96efc2a`](https://github.com/juspay/hyperswitch/commit/96efc2abf94e3e9174f625bee2270236bad50278))
  - [stripe] capture error_code and error_message for psync ([#3771](https://github.com/juspay/hyperswitch/pull/3771)) ([`614182a`](https://github.com/juspay/hyperswitch/commit/614182ae4cdc7a762e0ce90d1336b1ff16fc9fa3))
  - [Trustpay][Volt] Mask PII data ([#3932](https://github.com/juspay/hyperswitch/pull/3932)) ([`a179b9c`](https://github.com/juspay/hyperswitch/commit/a179b9c90c2b9a419f1ce394d06158f80c29ee45))
  - [Nuvie] Mask PII data ([#3924](https://github.com/juspay/hyperswitch/pull/3924)) ([`6b2f71c`](https://github.com/juspay/hyperswitch/commit/6b2f71c850ff2ea36365375a81a7026fd8c87ebc))
  - [adyen] add more fields in the payments request ([#4010](https://github.com/juspay/hyperswitch/pull/4010)) ([`5584f11`](https://github.com/juspay/hyperswitch/commit/5584f1131ae4180020be23d4c735b8356482c22d))
- **core:** Updated payments response with payment_method_id & payment_method_status ([#3883](https://github.com/juspay/hyperswitch/pull/3883)) ([`7391416`](https://github.com/juspay/hyperswitch/commit/7391416e2473eab0474bd01bb155a9ecc96da263))

**Full Changelog:** [`2024.03.11.0...2024.03.11.1`](https://github.com/juspay/hyperswitch/compare/2024.03.11.0...2024.03.11.1)

- - -

## 2024.03.11.0

### Features

- **connector:**
  - Add threedsecureio three_ds authentication connector ([#4004](https://github.com/juspay/hyperswitch/pull/4004)) ([`06c3096`](https://github.com/juspay/hyperswitch/commit/06c30967cf626e7406aa9be8643fb73288aae383))
  - [Checkout] add support for external authentication for checkout connector ([#4006](https://github.com/juspay/hyperswitch/pull/4006)) ([`142a22c`](https://github.com/juspay/hyperswitch/commit/142a22c752a7c623cee62a6d552e6ffda73df777))
- **router:** Add payments authentication api flow ([#3996](https://github.com/juspay/hyperswitch/pull/3996)) ([`41556ba`](https://github.com/juspay/hyperswitch/commit/41556baed98c59373e0a053c023c32f2f7346b51))

**Full Changelog:** [`2024.03.09.0...2024.03.11.0`](https://github.com/juspay/hyperswitch/compare/2024.03.09.0...2024.03.11.0)

- - -

## 2024.03.09.0

### Features

- **core:** Add core functions for external authentication ([#3969](https://github.com/juspay/hyperswitch/pull/3969)) ([`897e264`](https://github.com/juspay/hyperswitch/commit/897e264ad9e26df9877a18eef26a24e05de78528))
- **payment_link:** Add shimmer page before payment_link loads starts ([#4014](https://github.com/juspay/hyperswitch/pull/4014)) ([`ba9d465`](https://github.com/juspay/hyperswitch/commit/ba9d465483edcefeacc7ace0fc8efc86ca0f813c))

### Bug Fixes

- **deserialization:** Error message is different when invalid data is passed for payment method data ([#4022](https://github.com/juspay/hyperswitch/pull/4022)) ([`f1fe295`](https://github.com/juspay/hyperswitch/commit/f1fe295475adb0e827bd713be036687da662b361))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`a7d0487`](https://github.com/juspay/hyperswitch/commit/a7d04873d63c1f007d0081f02ba9a373e24ae882))

**Full Changelog:** [`2024.03.08.0...2024.03.09.0`](https://github.com/juspay/hyperswitch/compare/2024.03.08.0...2024.03.09.0)

- - -

## 2024.03.08.0

### Features

- **router:** Add domain types, admin core changes and other prerequisites for 3ds external authentication flow ([#3962](https://github.com/juspay/hyperswitch/pull/3962)) ([`4902c40`](https://github.com/juspay/hyperswitch/commit/4902c403452500847f0395babc5fb940f4e2b755))

### Bug Fixes

- **deserialization:** Deserialize reward payment method data ([#4011](https://github.com/juspay/hyperswitch/pull/4011)) ([`f6b44f3`](https://github.com/juspay/hyperswitch/commit/f6b44f3860147a2ddc7b37123bfe064e50b7182a))
- **postman:** Fix postman collections for saving cards with customer_acceptance ([#4008](https://github.com/juspay/hyperswitch/pull/4008)) ([`deac899`](https://github.com/juspay/hyperswitch/commit/deac8991f78bd29d081088b0cf75a254eb358a2e))
- **webhooks:** Abort outgoing webhook retry task if webhook URL is not available in business profile ([#3997](https://github.com/juspay/hyperswitch/pull/3997)) ([`ce0ac3d`](https://github.com/juspay/hyperswitch/commit/ce0ac3d0297da5372772efe19167f0d2f62e82eb))

### Refactors

- **core:** Add OnSession as default for setup_future_usage ([#3990](https://github.com/juspay/hyperswitch/pull/3990)) ([`f9b6f5d`](https://github.com/juspay/hyperswitch/commit/f9b6f5da36c3a57da4b89db3151996403e2f3dfd))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`d36702d`](https://github.com/juspay/hyperswitch/commit/d36702d270be2b7e3816954fbac4a320d8224f31))

**Full Changelog:** [`2024.03.07.1...2024.03.08.0`](https://github.com/juspay/hyperswitch/compare/2024.03.07.1...2024.03.08.0)

- - -

## 2024.03.07.1

### Features

- **users:** Add new API get the user and role details of specific user ([#3988](https://github.com/juspay/hyperswitch/pull/3988)) ([`ba42fba`](https://github.com/juspay/hyperswitch/commit/ba42fbaed0adb2a3e4d9f2d07a4f0d99ba227241))

### Bug Fixes

- **users:** Revert using mget in authorization ([#3999](https://github.com/juspay/hyperswitch/pull/3999)) ([`7375b86`](https://github.com/juspay/hyperswitch/commit/7375b866a2a2767df2f213bc9eb61268392fb60d))

### Refactors

- **router:** Store `ApplepayPaymentMethod` in `payment_method_data` column of `payment_attempt` table ([#3940](https://github.com/juspay/hyperswitch/pull/3940)) ([`6671bff`](https://github.com/juspay/hyperswitch/commit/6671bff3b11e9548a0085046d2594cad9f2571e2))

**Full Changelog:** [`2024.03.07.0...2024.03.07.1`](https://github.com/juspay/hyperswitch/compare/2024.03.07.0...2024.03.07.1)

- - -

## 2024.03.07.0

### Features

- **connector:** [AUTHORIZEDOTNET] Add billing address in payments request ([#3981](https://github.com/juspay/hyperswitch/pull/3981)) ([`3806cd3`](https://github.com/juspay/hyperswitch/commit/3806cd35c763cc4517b761b4e3b0e736c60fac9f))
- **core:** Store customer_acceptance in the payment_methods table ([#3885](https://github.com/juspay/hyperswitch/pull/3885)) ([`a1fd36a`](https://github.com/juspay/hyperswitch/commit/a1fd36a1abea4d400386a00ccf182dfe9da5bcda))
- **payment_method:** Set the initial payment method as default until its explicitly set ([#3970](https://github.com/juspay/hyperswitch/pull/3970)) ([`34c1b90`](https://github.com/juspay/hyperswitch/commit/34c1b905b178973d2611bab14c7d85582ed225f0))
- **payment_methods:** Store connector_mandate_details in PaymentMethods table ([#3907](https://github.com/juspay/hyperswitch/pull/3907)) ([`d220e81`](https://github.com/juspay/hyperswitch/commit/d220e815dc81925b205fb57d5d4f05883c1a7cde))

### Bug Fixes

- **connector:**
  - [Trustpay] Add mapping to error code `100.390.105` ([#3968](https://github.com/juspay/hyperswitch/pull/3968)) ([`bf67587`](https://github.com/juspay/hyperswitch/commit/bf675878a2e36f7005468e91eefadc111ccba6b2))
  - [adyen] handle Webhook reference and object ([#3976](https://github.com/juspay/hyperswitch/pull/3976)) ([`0aa40cb`](https://github.com/juspay/hyperswitch/commit/0aa40cbae75fd4cf5b13cfc518ff761b2b673246))
- **tests/postman/adyen:** Remove enabled payment methods for payouts processor ([#3913](https://github.com/juspay/hyperswitch/pull/3913)) ([`289b20a`](https://github.com/juspay/hyperswitch/commit/289b20a82e5ee32aae6eb4e5766f9c757d26345d))
- **user:**
  - Use mget to check in blocklist ([#3945](https://github.com/juspay/hyperswitch/pull/3945)) ([`8154a61`](https://github.com/juspay/hyperswitch/commit/8154a611efcfa4bef3d5674db0574b065b55e9cd))
  - Improve role validation to prevent duplicate groups ([#3949](https://github.com/juspay/hyperswitch/pull/3949)) ([`05a4752`](https://github.com/juspay/hyperswitch/commit/05a475271a2c37ba6ced90b85e53015c47d573bc))

### Refactors

- **connector:** [Checkout] handle default cases for dispute status mapping ([#3966](https://github.com/juspay/hyperswitch/pull/3966)) ([`2cda3dd`](https://github.com/juspay/hyperswitch/commit/2cda3dd794e51f84537a89e1015ee975322a2081))
- **payment_methods:**
  - Filter applepay payment method from mca based on customer pm ([#3953](https://github.com/juspay/hyperswitch/pull/3953)) ([`2db39e8`](https://github.com/juspay/hyperswitch/commit/2db39e8bb9af3d55e3d075d77ff8616ee2e15f0a))
  - Prevent deletion of default payment method for a customer ([#3964](https://github.com/juspay/hyperswitch/pull/3964)) ([`db39bb0`](https://github.com/juspay/hyperswitch/commit/db39bb0a3cf350e8399a7f17842d9af9b2de440e))
  - Insert payment_method_id in redis for wallet tokens ([#3989](https://github.com/juspay/hyperswitch/pull/3989)) ([`d997e29`](https://github.com/juspay/hyperswitch/commit/d997e298f2614079a72a773493cd98ba4507b35a))
- Kms decrypt analytics config ([#3984](https://github.com/juspay/hyperswitch/pull/3984)) ([`cfade55`](https://github.com/juspay/hyperswitch/commit/cfade55e693594a772c18eee2c35d7b3dc03f84d))

### Miscellaneous Tasks

- **doc:** Add API ref for KV toggle ([#3784](https://github.com/juspay/hyperswitch/pull/3784)) ([`5e8fcda`](https://github.com/juspay/hyperswitch/commit/5e8fcda7d12f482e47be9ed672093cb45fac9e29))
- **postman:** Update Postman collection files ([`2db4a59`](https://github.com/juspay/hyperswitch/commit/2db4a599eae2560bfef327231f2381af74145e39))

**Full Changelog:** [`2024.03.06.0...2024.03.07.0`](https://github.com/juspay/hyperswitch/compare/2024.03.06.0...2024.03.07.0)

- - -

## 2024.03.06.0

### Features

- **api_models:** Add api_models for external 3ds authentication flow ([#3858](https://github.com/juspay/hyperswitch/pull/3858)) ([`0a43ceb`](https://github.com/juspay/hyperswitch/commit/0a43ceb14e27d998794941ecb7605b9e7175c757))
- **connector:** [Checkout] accept connector_transaction_id in 2xx and 4xx error_response of connector flows ([#3959](https://github.com/juspay/hyperswitch/pull/3959)) ([`f6f6a0c`](https://github.com/juspay/hyperswitch/commit/f6f6a0c0f727a6f367c6bafb4db9a89cb46f667a))
- **core:** External authentication related schema changes for existing tables ([#3904](https://github.com/juspay/hyperswitch/pull/3904)) ([`c09b2b3`](https://github.com/juspay/hyperswitch/commit/c09b2b3a2ae9a71d4a73063faf4796e0c8732bb4))
- **payouts:** Implement Single Connector Retry for Payouts ([#3908](https://github.com/juspay/hyperswitch/pull/3908)) ([`0cb95a4`](https://github.com/juspay/hyperswitch/commit/0cb95a4911054e089e6ed3c528645ee1b881ebc6))
- **roles:** Add caching for custom roles ([#3946](https://github.com/juspay/hyperswitch/pull/3946)) ([`19c5023`](https://github.com/juspay/hyperswitch/commit/19c502398f980d20b9e0a4fe98c33a2239c90c5b))
- **router:** Add incoming header request logs ([#3939](https://github.com/juspay/hyperswitch/pull/3939)) ([`050df50`](https://github.com/juspay/hyperswitch/commit/050df5022cd3d44db23ca75f81158fb7c2429f86))

### Bug Fixes

- **core:** Fix metadata validation for update payment connector ([#3834](https://github.com/juspay/hyperswitch/pull/3834)) ([`54938ad`](https://github.com/juspay/hyperswitch/commit/54938ad345a2b899360b608d8845fd7f885f82ba))
- **router:** [nuvei] Nuvei error handling for payment declined status and included tests ([#3832](https://github.com/juspay/hyperswitch/pull/3832)) ([`087932f`](https://github.com/juspay/hyperswitch/commit/087932f06044454570c971def0e82dc3d838598c))

### Refactors

- **connector:**
  - [Fiserv] Mask PII data ([#3821](https://github.com/juspay/hyperswitch/pull/3821)) ([`03cfb73`](https://github.com/juspay/hyperswitch/commit/03cfb735af29f00bccf729013e7e06684611b30d))
  - Remove default cases for Authorizedotnet, Braintree and Fiserv Connector ([#2796](https://github.com/juspay/hyperswitch/pull/2796)) ([`dbac556`](https://github.com/juspay/hyperswitch/commit/dbac55683a8f95e0efdbac43f8c2ae793063a032))

### Miscellaneous Tasks

- **configs:** [BOA] Add USD Currency Filter Configuration ([#3961](https://github.com/juspay/hyperswitch/pull/3961)) ([`8a0e468`](https://github.com/juspay/hyperswitch/commit/8a0e468e6a574717b29ccbdd143908727c251dfb))
- **postman:** Update Postman collection files ([`6305bb5`](https://github.com/juspay/hyperswitch/commit/6305bb57269fb5f6803edbad58d6e574ad4f6509))
- **tests:** Add unit tests for backwards compatibility ([#3822](https://github.com/juspay/hyperswitch/pull/3822)) ([`c65729a`](https://github.com/juspay/hyperswitch/commit/c65729adc9009f046398312a16841532fdc177da))

**Full Changelog:** [`2024.03.05.0...2024.03.06.0`](https://github.com/juspay/hyperswitch/compare/2024.03.05.0...2024.03.06.0)

- - -

## 2024.03.05.0

### Features

- **connector:** [PLACETOPAY] Fix refund request and status mapping ([#3894](https://github.com/juspay/hyperswitch/pull/3894)) ([`5eff9d4`](https://github.com/juspay/hyperswitch/commit/5eff9d47d3e53d380ef792a8fbdf06ecf78d3d16))
- **webhooks:** Implement automatic retries for failed webhook deliveries using scheduler ([#3842](https://github.com/juspay/hyperswitch/pull/3842)) ([`5bb67c7`](https://github.com/juspay/hyperswitch/commit/5bb67c7dcc22f9cee51adf501bdd8455b41548db))

### Bug Fixes

- **connector:** [Volt] Fix status mapping for Volt ([#3915](https://github.com/juspay/hyperswitch/pull/3915)) ([`f132527`](https://github.com/juspay/hyperswitch/commit/f132527490a7d8cd8469573d8e6856f33974959f))
- **router:** [nuvei] Nuvei recurring MIT fix and mandatory details fix ([#3602](https://github.com/juspay/hyperswitch/pull/3602)) ([`aa001b4`](https://github.com/juspay/hyperswitch/commit/aa001b4579a6be022b46eb0cc3e65c52ec9d10bb))

### Refactors

- **api_keys:** Provide identifier for api key in the expiry reminder email ([#3888](https://github.com/juspay/hyperswitch/pull/3888)) ([`901d61b`](https://github.com/juspay/hyperswitch/commit/901d61bc0ddb4b2ad742de927126f468629a79af))
- **connectors:** [Checkout] PII data masking ([#3775](https://github.com/juspay/hyperswitch/pull/3775)) ([`6076eb0`](https://github.com/juspay/hyperswitch/commit/6076eb01ca80ae2d06218a09d2a69f01d78cdec4))
- **test_utils:** Use json to run collection and add run time edit ([#3807](https://github.com/juspay/hyperswitch/pull/3807)) ([`a1d63d4`](https://github.com/juspay/hyperswitch/commit/a1d63d4b8be273c525aac76f22cf3bda25719f28))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`cd7040f`](https://github.com/juspay/hyperswitch/commit/cd7040fa8cad2e69a53e3ed609c9eb8a8a17495a))
- Upgrade msrv to 1.70 ([#3938](https://github.com/juspay/hyperswitch/pull/3938)) ([`0e60083`](https://github.com/juspay/hyperswitch/commit/0e600837f77af0443335deb0c73d6f3b2bda5ac2))

**Full Changelog:** [`2024.03.04.0...2024.03.05.0`](https://github.com/juspay/hyperswitch/compare/2024.03.04.0...2024.03.05.0)

- - -

## 2024.03.04.0

### Features

- **address:** Add payment method billing details ([#3812](https://github.com/juspay/hyperswitch/pull/3812)) ([`33f0741`](https://github.com/juspay/hyperswitch/commit/33f07419abb7adc9198c67604f4d0bebab9faeb4))
- **core:** Diesel models and db interface changes for authentication table ([#3859](https://github.com/juspay/hyperswitch/pull/3859)) ([`8162668`](https://github.com/juspay/hyperswitch/commit/816266819928477738f70b782eab0e26b600b171))

### Bug Fixes

- **connector:** [BOA/CYB] Pass ucaf for apple pay mastercard users ([#3899](https://github.com/juspay/hyperswitch/pull/3899)) ([`f95beaa`](https://github.com/juspay/hyperswitch/commit/f95beaa189f17a6e117971a749e2b4595e1e2fc3))
- **mandates:** Remove validation for `mandate_data` object in payments create request ([#3860](https://github.com/juspay/hyperswitch/pull/3860)) ([`49d2298`](https://github.com/juspay/hyperswitch/commit/49d22981026e0bc5105aca842a3be6533bbbd477))
- **payment_methods:** Insert `locker_id` as null in case of payment method not getting stored in locker ([#3919](https://github.com/juspay/hyperswitch/pull/3919)) ([`9917dd0`](https://github.com/juspay/hyperswitch/commit/9917dd065444d66628039b19df7cd8e7d5c107db))
- **wasm:** [Adyen] update connector account configs and integration bugs ([#3910](https://github.com/juspay/hyperswitch/pull/3910)) ([`34f7705`](https://github.com/juspay/hyperswitch/commit/34f7705c44f5551ccc34a54b70867177909079a7))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`cb5761b`](https://github.com/juspay/hyperswitch/commit/cb5761be47fa5a9f6a1e0abb135369de96a116fa))
- Adding addition fields from psql to kafka event for analytics usecase ([#3815](https://github.com/juspay/hyperswitch/pull/3815)) ([`cc0d006`](https://github.com/juspay/hyperswitch/commit/cc0d00633058277e6f49f352e8d158554c864038))

**Full Changelog:** [`2024.03.01.0...2024.03.04.0`](https://github.com/juspay/hyperswitch/compare/2024.03.01.0...2024.03.04.0)

- - -

## 2024.03.01.0

### Features

- **roles:** Add groups for `get_from_token` api ([#3872](https://github.com/juspay/hyperswitch/pull/3872)) ([`b0b9bfa`](https://github.com/juspay/hyperswitch/commit/b0b9bfa731695b530cdcdeaeba29dc0f88bd8887))
- Add unresponsive timeout for fred ([#3369](https://github.com/juspay/hyperswitch/pull/3369)) ([`26fb96e`](https://github.com/juspay/hyperswitch/commit/26fb96eeaaaffb4e4f87a644a3f7cc920e4b2057))

### Bug Fixes

- **connector:** [adyen] production endpoints and mappings ([#3900](https://github.com/juspay/hyperswitch/pull/3900)) ([`8933ddf`](https://github.com/juspay/hyperswitch/commit/8933ddff66901027b22bb01424a528d20b54adad))

### Refactors

- **connector:** CANCEL button after redirection is enabled for card 3ds ([#3829](https://github.com/juspay/hyperswitch/pull/3829)) ([`e003958`](https://github.com/juspay/hyperswitch/commit/e003958ff31ea0f1e0cddb3d2369945e8d2a2353))
- **core:** Status mapping for Capture for 429 http code ([#3897](https://github.com/juspay/hyperswitch/pull/3897)) ([`9b5f26a`](https://github.com/juspay/hyperswitch/commit/9b5f26a5d29fe8d297cb8651b53be5cfba275276))
- **roles:** Add more checks in create, update role APIs and change the response type ([#3896](https://github.com/juspay/hyperswitch/pull/3896)) ([`0136523`](https://github.com/juspay/hyperswitch/commit/0136523f38b7ceda0022843779ba922d612423a6))
- **router:** Add parent caller function for DB ([#3838](https://github.com/juspay/hyperswitch/pull/3838)) ([`0936b02`](https://github.com/juspay/hyperswitch/commit/0936b02ade7f57eaa0213c4f4422bff7c9bb4de2))

### Miscellaneous Tasks

- **configs:** [Cashtocode] wasm changes for AUD, INR, JPY, NZD, ZAR currency ([#3892](https://github.com/juspay/hyperswitch/pull/3892)) ([`de7f400`](https://github.com/juspay/hyperswitch/commit/de7f400c07d85b97340255556b39383648a0fd9f))
- **dispute:** Adding disputeamount as int type ([#3886](https://github.com/juspay/hyperswitch/pull/3886)) ([`7db499d`](https://github.com/juspay/hyperswitch/commit/7db499d8a9388b9a3674f7fa130bc389151840ec))

**Full Changelog:** [`2024.02.29.0...2024.03.01.0`](https://github.com/juspay/hyperswitch/compare/2024.02.29.0...2024.03.01.0)

- - -

## 2024.02.29.0

### Features

- **analytics:**
  - Adding metric api for dispute analytics ([#3810](https://github.com/juspay/hyperswitch/pull/3810)) ([`de6b16b`](https://github.com/juspay/hyperswitch/commit/de6b16bed98280a4ed8fc8cdad920a759662aa19))
  - Add force retrieve call for force retrieve calls ([#3565](https://github.com/juspay/hyperswitch/pull/3565)) ([`032d58c`](https://github.com/juspay/hyperswitch/commit/032d58cdbbf388cf25cbf2e43b0117b83f7d076d))
- **payment_methods:** Add default payment method column in customers table and last used column in payment_methods table ([#3790](https://github.com/juspay/hyperswitch/pull/3790)) ([`f3931cf`](https://github.com/juspay/hyperswitch/commit/f3931cf484f61a4d9c107c362d0f3f6ee872e0e7))
- **payouts:** Implement Smart Retries for Payout ([#3580](https://github.com/juspay/hyperswitch/pull/3580)) ([`8b32dff`](https://github.com/juspay/hyperswitch/commit/8b32dffe324a4cdbfde173cffe3fad2e839a52aa))

### Bug Fixes

- **tests/postman/adyen:** Enable sepa payment method type for payout flows ([#3861](https://github.com/juspay/hyperswitch/pull/3861)) ([`53559c2`](https://github.com/juspay/hyperswitch/commit/53559c22527dde9536aa493ad7cd3bf353335c1a))

### Refactors

- **connector:**
  - [Gocardless] Mask PII data ([#3844](https://github.com/juspay/hyperswitch/pull/3844)) ([`2f3ec7f`](https://github.com/juspay/hyperswitch/commit/2f3ec7f951967359d3995f743a486f3b380dd1f8))
  - [Mollie] Mask PII data ([#3856](https://github.com/juspay/hyperswitch/pull/3856)) ([`ffbe042`](https://github.com/juspay/hyperswitch/commit/ffbe042fdccde4a721d329d6b85c95203234368e))
- **payment_link:** Add Miscellaneous charges in cart ([#3645](https://github.com/juspay/hyperswitch/pull/3645)) ([`15b367e`](https://github.com/juspay/hyperswitch/commit/15b367eb792448fb3f3312484ab13dd8241d4a14))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`5c91a94`](https://github.com/juspay/hyperswitch/commit/5c91a9440e098490cc00a54ead34989da81babc0))

**Full Changelog:** [`2024.02.28.0...2024.02.29.0`](https://github.com/juspay/hyperswitch/compare/2024.02.28.0...2024.02.29.0)

- - -

## 2024.02.28.0

### Features

- **connector:** Mask pii information in connector request and response for stripe, aci, adyen, airwallex and authorizedotnet ([#3678](https://github.com/juspay/hyperswitch/pull/3678)) ([`1c6913b`](https://github.com/juspay/hyperswitch/commit/1c6913be747bd3da53fa2b48e339810bb30226e7))
- **roles:** Change list roles, get role and authorization info api to respond with groups ([#3837](https://github.com/juspay/hyperswitch/pull/3837)) ([`fbe9d2f`](https://github.com/juspay/hyperswitch/commit/fbe9d2f19e9c0ca3af45a60e3d82b3ea774e11ce))
- **router:** Add connector mit related columns to the payment methods table ([#3764](https://github.com/juspay/hyperswitch/pull/3764)) ([`5b8c261`](https://github.com/juspay/hyperswitch/commit/5b8c261d1ec34fab850c33f5d59d46255c7ebe4f))

### Bug Fixes

- **connector:** [AUTHORIZEDOTNET] Fix status mapping ([#3845](https://github.com/juspay/hyperswitch/pull/3845)) ([`f4d0e2b`](https://github.com/juspay/hyperswitch/commit/f4d0e2b441a25048186be4b9d0871e2473a6f357))
- **core:** Validate amount_to_capture in payment update ([#3830](https://github.com/juspay/hyperswitch/pull/3830)) ([`04e9734`](https://github.com/juspay/hyperswitch/commit/04e9734800a9011d9ae7bd43f75c90a75a9a9334))

### Refactors

- **compatibility:** Added compatibility layer request logs ([#3774](https://github.com/juspay/hyperswitch/pull/3774)) ([`cd1a17b`](https://github.com/juspay/hyperswitch/commit/cd1a17bcd260629aad7548ff274f5512c37bfab7))
- **connector:**
  - [Forte] Mask PII data ([#3824](https://github.com/juspay/hyperswitch/pull/3824)) ([`bd890b0`](https://github.com/juspay/hyperswitch/commit/bd890b0715ff5b77b8d1769083fa3e6c965e6dc3))
  - [Braintree] Mask PII data ([#3759](https://github.com/juspay/hyperswitch/pull/3759)) ([`3e87d44`](https://github.com/juspay/hyperswitch/commit/3e87d4468193cb4f60cd7b9fe93f2eba9250eeb5))
  - [Square] change error message from NotSupported to NotImplemented ([#2875](https://github.com/juspay/hyperswitch/pull/2875)) ([`0626ca9`](https://github.com/juspay/hyperswitch/commit/0626ca968576709a3559243f5a64e742201dbf91))
- **payment_methods:** Introduce `locker_id` column in `payment_methods` table ([#3760](https://github.com/juspay/hyperswitch/pull/3760)) ([`3856226`](https://github.com/juspay/hyperswitch/commit/385622678f764b2bdb67423be0e5c8f055dd0b7c))
- **router:** Added logs health and deep health ([#3780](https://github.com/juspay/hyperswitch/pull/3780)) ([`cd82228`](https://github.com/juspay/hyperswitch/commit/cd8222820a19c53dbc0a0abe6f8ab3408cb7b13f))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`8862746`](https://github.com/juspay/hyperswitch/commit/88627463eacd86bf3c8726ea4a08aedb6236ca32))

**Full Changelog:** [`2024.02.27.0...2024.02.28.0`](https://github.com/juspay/hyperswitch/compare/2024.02.27.0...2024.02.28.0)

- - -

## 2024.02.27.0

### Features

- **connector:** [Payme] Add Void flow to Payme ([#3817](https://github.com/juspay/hyperswitch/pull/3817)) ([`9aabb14`](https://github.com/juspay/hyperswitch/commit/9aabb14a60f821769ccc61013368fb9683711d94))
- **payouts:** Extend routing capabilities to payout operation ([#3531](https://github.com/juspay/hyperswitch/pull/3531)) ([`75c633f`](https://github.com/juspay/hyperswitch/commit/75c633fc7c37341177597041ccbcdfc3cf9e236f))
- Add unique constraint restriction for KV ([#3723](https://github.com/juspay/hyperswitch/pull/3723)) ([`c117f8e`](https://github.com/juspay/hyperswitch/commit/c117f8ec638536d7ca92603ddadba59793b232de))

### Bug Fixes

- **core:** Do not construct request if it is already available ([#3826](https://github.com/juspay/hyperswitch/pull/3826)) ([`84d91a7`](https://github.com/juspay/hyperswitch/commit/84d91a7b344df47899ff31a87b86b8410c204f95))

### Refactors

- **connector:** [Cybersource] Mask PII data ([#3786](https://github.com/juspay/hyperswitch/pull/3786)) ([`a5cb6bb`](https://github.com/juspay/hyperswitch/commit/a5cb6bb5a456e8c435bbe0b561aa4d8a6f29ad87))
- Incorporate `hyperswitch_interface` into router ([#3669](https://github.com/juspay/hyperswitch/pull/3669)) ([`2185cd3`](https://github.com/juspay/hyperswitch/commit/2185cd38c1ddf08d9dbb7a320b627fc03f0e7026))

**Full Changelog:** [`2024.02.26.0...2024.02.27.0`](https://github.com/juspay/hyperswitch/compare/2024.02.26.0...2024.02.27.0)

- - -

## 2024.02.26.0

### Features

- **connector:** [BOA/Cybersource] Pass commerce indicator using card network for apple pay ([#3795](https://github.com/juspay/hyperswitch/pull/3795)) ([`54fa309`](https://github.com/juspay/hyperswitch/commit/54fa309b7da5d153855fb684f9655f505b2ba309))
- **roles:** Add blacklist for roles ([#3794](https://github.com/juspay/hyperswitch/pull/3794)) ([`734327a`](https://github.com/juspay/hyperswitch/commit/734327a957c216511b182151a2f0b27819e7e3bb))

### Bug Fixes

- **cards:** Return a 200 response indicating that a customer is none ([#3773](https://github.com/juspay/hyperswitch/pull/3773)) ([`2c95dcd`](https://github.com/juspay/hyperswitch/commit/2c95dcd19778726f476b219271dc42da182088af))

**Full Changelog:** [`2024.02.23.0...2024.02.26.0`](https://github.com/juspay/hyperswitch/compare/2024.02.23.0...2024.02.26.0)

- - -

## 2024.02.23.0

### Features

- **address:** Add email field to address ([#3682](https://github.com/juspay/hyperswitch/pull/3682)) ([`863e380`](https://github.com/juspay/hyperswitch/commit/863e380cf2eb8ace17cad0f1bcbc2a9f4a460983))
- **router:** Added api for the deleting config key ([#3554](https://github.com/juspay/hyperswitch/pull/3554)) ([`bbb3d3d`](https://github.com/juspay/hyperswitch/commit/bbb3d3d1e26f303964a495606dece7958f6c40fc))
- **user:** Create apis for custom role ([#3763](https://github.com/juspay/hyperswitch/pull/3763)) ([`58809ab`](https://github.com/juspay/hyperswitch/commit/58809ab1f9c00d802b9a2a3d30b17dff1614431d))

### Bug Fixes

- **api_keys:** Fix internal server error being thrown when trying to update or delete non-existent API key ([#3762](https://github.com/juspay/hyperswitch/pull/3762)) ([`5c24a76`](https://github.com/juspay/hyperswitch/commit/5c24a76fbd0de314f370a4e3d3ca897d2b7eaa3d))

### Refactors

- **connector:**
  - [NMI] add hyperswitch loader to card 3ds ([#3755](https://github.com/juspay/hyperswitch/pull/3755)) ([`5aae179`](https://github.com/juspay/hyperswitch/commit/5aae1798257b5ee0c5a62104e4711748cdb5f935))
  - [NMI] Include customer_vault_id for card 3ds transaction request ([#3777](https://github.com/juspay/hyperswitch/pull/3777)) ([`2e7d30a`](https://github.com/juspay/hyperswitch/commit/2e7d30a4ad8d1731b5e91020488658d9d65866f6))
- **connectors:** [Bluesnap] PII data masking ([#3714](https://github.com/juspay/hyperswitch/pull/3714)) ([`d000847`](https://github.com/juspay/hyperswitch/commit/d000847b938952de6ff9c2e01bdd06b4ede60e69))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`1d739ee`](https://github.com/juspay/hyperswitch/commit/1d739eee5eca8051b4a1d6a91a656df646219964))

**Full Changelog:** [`2024.02.22.0...2024.02.23.0`](https://github.com/juspay/hyperswitch/compare/2024.02.22.0...2024.02.23.0)

- - -

## 2024.02.22.0

### Features

- **authz:** Add custom role checks in authorization ([#3719](https://github.com/juspay/hyperswitch/pull/3719)) ([`ada6a32`](https://github.com/juspay/hyperswitch/commit/ada6a3227616b556a0fb710302434027ff2276f4))
- **connector:**
  - [adyen] Use connector_response_reference_id as reference to merchant ([#3688](https://github.com/juspay/hyperswitch/pull/3688)) ([`f3b90ee`](https://github.com/juspay/hyperswitch/commit/f3b90ee17f35253dd39d3e2723f8b56d416fd6e3))
  - [Adyen] populate connector_transaction_id for Adyen Payment Response ([#3727](https://github.com/juspay/hyperswitch/pull/3727)) ([`deec8b4`](https://github.com/juspay/hyperswitch/commit/deec8b4eb5493b072eaef0352a735748979cd95d))
- **invite_multiple:** Set status of user as `InvitationSent` if `email` feature flag is enabled ([#3757](https://github.com/juspay/hyperswitch/pull/3757)) ([`ef5e886`](https://github.com/juspay/hyperswitch/commit/ef5e886ab1abdf50254343be8c6c48100ec2ec2d))
- **users:** Send email to user if the user already exists ([#3705](https://github.com/juspay/hyperswitch/pull/3705)) ([`9725223`](https://github.com/juspay/hyperswitch/commit/97252237a9c7aa1cb5e7fa15f7ccb5c365b70b85))

### Bug Fixes

- **core:** Validate capture method before update trackers ([#3715](https://github.com/juspay/hyperswitch/pull/3715)) ([`5952017`](https://github.com/juspay/hyperswitch/commit/5952017260180f0b52f989b60ff678868267a634))
- **users:** Fix wrong email content in invite users ([#3625](https://github.com/juspay/hyperswitch/pull/3625)) ([`e139731`](https://github.com/juspay/hyperswitch/commit/e139731761387e9f00546815e260287ed600cc6e))

### Refactors

- **core:** Inclusion of locker to store fingerprints ([#3630](https://github.com/juspay/hyperswitch/pull/3630)) ([`7b0bce5`](https://github.com/juspay/hyperswitch/commit/7b0bce555845c6d1187c97a921342fe129b6acba))
- **permissions:** Remove permissions for utility APIs ([#3730](https://github.com/juspay/hyperswitch/pull/3730)) ([`4ae28e4`](https://github.com/juspay/hyperswitch/commit/4ae28e48cd73a9f96b6ae24babf167824fd182a0))
- **scheduler:** Improve code reusability and consumer logs ([#3712](https://github.com/juspay/hyperswitch/pull/3712)) ([`7c63c76`](https://github.com/juspay/hyperswitch/commit/7c63c76011cec5fb398cff90b6237578c132b87d))

**Full Changelog:** [`2024.02.21.0...2024.02.22.0`](https://github.com/juspay/hyperswitch/compare/2024.02.21.0...2024.02.22.0)

- - -

## 2024.02.21.0

### Features

- **analytics:** Added filter api for dispute analytics ([#3724](https://github.com/juspay/hyperswitch/pull/3724)) ([`6aeb440`](https://github.com/juspay/hyperswitch/commit/6aeb44091b34f202b60868028979b3720e3507ce))
- **connector:** Accept connector_transaction_id in 2xx and 4xx error_response of connector flows for Adyen ([#3703](https://github.com/juspay/hyperswitch/pull/3703)) ([`236c5ba`](https://github.com/juspay/hyperswitch/commit/236c5baeda69721513c91682edca54facb947536))

### Bug Fixes

- **config:** Add update mandate config in docker_compose ([#3732](https://github.com/juspay/hyperswitch/pull/3732)) ([`d541953`](https://github.com/juspay/hyperswitch/commit/d541953693ef7292fce2f4b2c39fe2cd5cddccbf))
- Remove status_code being printed in EndRequest log ([#3722](https://github.com/juspay/hyperswitch/pull/3722)) ([`cf3c666`](https://github.com/juspay/hyperswitch/commit/cf3c66636ffee30cdd4353b276a89a8f9fc2d9d0))

### Refactors

- **connector:** [ADYEN] Capture error reason in case of 2xx and 4xx failure ([#3708](https://github.com/juspay/hyperswitch/pull/3708)) ([`1c933a0`](https://github.com/juspay/hyperswitch/commit/1c933a08a9cad0980a4d14dd9b641995d0f4e659))
- **connectors:**
  - [Bitpay] PII data masking ([#3704](https://github.com/juspay/hyperswitch/pull/3704)) ([`1c92328`](https://github.com/juspay/hyperswitch/commit/1c9232820e4821652112b21863e1910b3dd6be3b))
  - [Bambora] data masking ([#3695](https://github.com/juspay/hyperswitch/pull/3695)) ([`e5e4485`](https://github.com/juspay/hyperswitch/commit/e5e44857d21af9db8dee580e276028de76c7d278))
  - [BOA] PII data masking ([#3702](https://github.com/juspay/hyperswitch/pull/3702)) ([`49c71d0`](https://github.com/juspay/hyperswitch/commit/49c71d093e7e01b241ab29e3bb7b0c724b399aaf))
- **merchant_connector_account:** Change unique constraint to connector label ([#3091](https://github.com/juspay/hyperswitch/pull/3091)) ([`073310c`](https://github.com/juspay/hyperswitch/commit/073310c1f671ccbb71cc5c8725eca9771e511589))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`421b9e8`](https://github.com/juspay/hyperswitch/commit/421b9e8f463949aca82e74e4259c88950f2bf15a))

**Full Changelog:** [`2024.02.20.0...2024.02.21.0`](https://github.com/juspay/hyperswitch/compare/2024.02.20.0...2024.02.21.0)

- - -

## 2024.02.20.0

### Features

- **analytics:** Added dispute as uri param to analytics info api ([#3693](https://github.com/juspay/hyperswitch/pull/3693)) ([`76ac1a7`](https://github.com/juspay/hyperswitch/commit/76ac1a753a08f3ecc8ee264e4bccc47e8b219d1d))
- **connector-config:** [Volt] Add config changes for open_banking_uk ([#3700](https://github.com/juspay/hyperswitch/pull/3700)) ([`1e45bb5`](https://github.com/juspay/hyperswitch/commit/1e45bb5d0f58047987cd98a063b5ffa770750423))
- **user:** Setup roles table with queries ([#3691](https://github.com/juspay/hyperswitch/pull/3691)) ([`e0d8bb2`](https://github.com/juspay/hyperswitch/commit/e0d8bb207e8db2a6ba47307090dea7b8a6b7759f))

### Bug Fixes

- **connector:**
  - [noon] Fail the payment for specific error_response ([#3674](https://github.com/juspay/hyperswitch/pull/3674)) ([`df739a3`](https://github.com/juspay/hyperswitch/commit/df739a302b062277647afe5c3888015272fdc2cf))
  - [Payme] `payme_transaction_id` converted to optional ([#3707](https://github.com/juspay/hyperswitch/pull/3707)) ([`3370c00`](https://github.com/juspay/hyperswitch/commit/3370c00589f7c04c2320370c672a9a569ab3907f))

### Refactors

- **ext_traits:** Simplify the signatures of some methods in `Encode` extension trait ([#3687](https://github.com/juspay/hyperswitch/pull/3687)) ([`11fc9b3`](https://github.com/juspay/hyperswitch/commit/11fc9b39867dec50ff37cd090c686560ba2d1a9d))
- **router:**
  - Remove fallback feature for `/add` and `/get` for locker ([#3648](https://github.com/juspay/hyperswitch/pull/3648)) ([`d0f529f`](https://github.com/juspay/hyperswitch/commit/d0f529fa4b2d14bbd0ae0986bc2bf037794d51e9))
  - Added status_code to golden_log_line ([#3681](https://github.com/juspay/hyperswitch/pull/3681)) ([`8038b48`](https://github.com/juspay/hyperswitch/commit/8038b48a54c937b3fe72b36cec5f20ee87309be4))
- Include api key expiry workflow into process tracker ([#3661](https://github.com/juspay/hyperswitch/pull/3661)) ([`0a7625f`](https://github.com/juspay/hyperswitch/commit/0a7625ff8c85f55a95a10415b31598fe9f16704a))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`2b8f1ba`](https://github.com/juspay/hyperswitch/commit/2b8f1ba1e6a60e03b94a7aab5466266852f69aa2))

**Full Changelog:** [`2024.02.19.0...2024.02.20.0`](https://github.com/juspay/hyperswitch/compare/2024.02.19.0...2024.02.20.0)

- - -

## 2024.02.19.0

### Features

- **analytics:** Adding kafka dispute analytic events ([#3549](https://github.com/juspay/hyperswitch/pull/3549)) ([`39e2233`](https://github.com/juspay/hyperswitch/commit/39e2233982c48977df8d501c898585bccd795c38))

### Bug Fixes

- **logging:** Fix missing fields in consolidated log line ([#3684](https://github.com/juspay/hyperswitch/pull/3684)) ([`783fa0b`](https://github.com/juspay/hyperswitch/commit/783fa0b0dff1e157920d683a75fc579942cd9c06))

### Refactors

- **connector:** [NMI] Add billing details for preprocessing ([#3672](https://github.com/juspay/hyperswitch/pull/3672)) ([`09d337b`](https://github.com/juspay/hyperswitch/commit/09d337b8a8d93884bff25d794b3a2feb314202ba))
- **openapi:** Enable other features in api_models when running openapi ([#3649](https://github.com/juspay/hyperswitch/pull/3649)) ([`fb254b8`](https://github.com/juspay/hyperswitch/commit/fb254b8924808e6a2b2a9a31dbed78749836e8d3))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`a49a34a`](https://github.com/juspay/hyperswitch/commit/a49a34af6c048c2649e4e8b0278ae83c4eb544a6))

**Full Changelog:** [`2024.02.16.0...2024.02.19.0`](https://github.com/juspay/hyperswitch/compare/2024.02.16.0...2024.02.19.0)

- - -

## 2024.02.16.0

### Features

- **users:** Email JWT blacklist ([#3659](https://github.com/juspay/hyperswitch/pull/3659)) ([`a9e3d74`](https://github.com/juspay/hyperswitch/commit/a9e3d74cc160d35b75278e39faac5df3aebd16bb))

### Bug Fixes

- **env:** Add dashboard origin in toml file ([#3662](https://github.com/juspay/hyperswitch/pull/3662)) ([`cbd4039`](https://github.com/juspay/hyperswitch/commit/cbd40390b874dd91c53516d9370466fa1bdd5d15))
- **user:** Add migration for force password change ([#3668](https://github.com/juspay/hyperswitch/pull/3668)) ([`2f473dd`](https://github.com/juspay/hyperswitch/commit/2f473dd4e73b53ee2a2ee462e9f4a51874d85a84))

### Refactors

- **connector:** [NMI] Add Zip code as mandatory field for 3DS ([#3666](https://github.com/juspay/hyperswitch/pull/3666)) ([`1ddaee4`](https://github.com/juspay/hyperswitch/commit/1ddaee44df6eca0f1068f41f82c57f80511b436b))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`e94930c`](https://github.com/juspay/hyperswitch/commit/e94930cf5cf4219b967a0b447d3c7503c6a7363d))

**Full Changelog:** [`2024.02.15.1...2024.02.16.0`](https://github.com/juspay/hyperswitch/compare/2024.02.15.1...2024.02.16.0)

- - -

## 2024.02.15.1

### Features

- **api_models:** Add client_secret type to payments ([#3557](https://github.com/juspay/hyperswitch/pull/3557)) ([`610a5a3`](https://github.com/juspay/hyperswitch/commit/610a5a3969789f1e1bcb074a262070247a030eb1))

### Bug Fixes

- Allow all headers on cors ([#3653](https://github.com/juspay/hyperswitch/pull/3653)) ([`64bf815`](https://github.com/juspay/hyperswitch/commit/64bf815294244b1f4d42ea6cefcf2177d0febf9e))

### Refactors

- **webhooks:** Check event type not supported before checking for profile_id ([#3543](https://github.com/juspay/hyperswitch/pull/3543)) ([`2d4f6b3`](https://github.com/juspay/hyperswitch/commit/2d4f6b3fa004a3f03beaa604e2dbfe95fcbe22a6))

**Full Changelog:** [`2024.02.15.0...2024.02.15.1`](https://github.com/juspay/hyperswitch/compare/2024.02.15.0...2024.02.15.1)

- - -

## 2024.02.15.0

### Features

- **connector:** [Adyen] add PMD validation in validate_capture_method method for all the implemented PM’s ([#3584](https://github.com/juspay/hyperswitch/pull/3584)) ([`0c46f39`](https://github.com/juspay/hyperswitch/commit/0c46f39b9e1a397cecde1de9438c65cc7b93766b))
- **events:** Connector response masking in clickhouse ([#3566](https://github.com/juspay/hyperswitch/pull/3566)) ([`5fb3c00`](https://github.com/juspay/hyperswitch/commit/5fb3c001b5dc371f81fe1708fd9a6c6978fb726e))
- Add cors rules to actix ([#3646](https://github.com/juspay/hyperswitch/pull/3646)) ([`e702341`](https://github.com/juspay/hyperswitch/commit/e702341c64f5a6f542de9d413a6aa2b2e731eea6))
- Noon payme cryptopay error mapping ([#3258](https://github.com/juspay/hyperswitch/pull/3258)) ([`702e945`](https://github.com/juspay/hyperswitch/commit/702e945be93645f9260663dd456e08c510c2f1fc))

### Bug Fixes

- **router:** Store connector_mandate_id in complete auth ([#3576](https://github.com/juspay/hyperswitch/pull/3576)) ([`91cd70a`](https://github.com/juspay/hyperswitch/commit/91cd70a60b89b1c4e868e359a75f4088854562ef))

### Refactors

- **router:** Added payment_method to golden log line ([#3620](https://github.com/juspay/hyperswitch/pull/3620)) ([`c5343df`](https://github.com/juspay/hyperswitch/commit/c5343dfcc20f1000e319c62fa0341c46701595ff))
- Incorporate `hyperswitch_interface` into drainer ([#3629](https://github.com/juspay/hyperswitch/pull/3629)) ([`7b1c65b`](https://github.com/juspay/hyperswitch/commit/7b1c65b60d3874262867f77c8c28ebfa410b89a3))
- Adding connector_name into logs ( Logging Changes ) ([#3581](https://github.com/juspay/hyperswitch/pull/3581)) ([`de12ba7`](https://github.com/juspay/hyperswitch/commit/de12ba779a229966c292caa05976883dafb4996e))

### Documentation

- **connector:** Add wasm docs in connector integration docs ([#3641](https://github.com/juspay/hyperswitch/pull/3641)) ([`1236741`](https://github.com/juspay/hyperswitch/commit/1236741a14befd7472b0db0060315bb6efe720e0))

**Full Changelog:** [`2024.02.14.0...2024.02.15.0`](https://github.com/juspay/hyperswitch/compare/2024.02.14.0...2024.02.15.0)

- - -

## 2024.02.14.0

### Features

- **pm_list:** Add required field for Boleto Payment Method ([#3619](https://github.com/juspay/hyperswitch/pull/3619)) ([`4d805f6`](https://github.com/juspay/hyperswitch/commit/4d805f61641175fc3566a5f6122d16745c484bf1))
- **users:** Add some checks for prod-intent send to biz email ([#3631](https://github.com/juspay/hyperswitch/pull/3631)) ([`774a032`](https://github.com/juspay/hyperswitch/commit/774a0322aa4b36d87b122e47cd893383e262de12))

### Bug Fixes

- **healthcheck:** Do not return true as response if the check if not applicable ([#3551](https://github.com/juspay/hyperswitch/pull/3551)) ([`6e103ce`](https://github.com/juspay/hyperswitch/commit/6e103cef50fea31d2508880985f80f0fd65cd536))

### Documentation

- **postman:** Update rustman and collection generation docs ([#3615](https://github.com/juspay/hyperswitch/pull/3615)) ([`02652a2`](https://github.com/juspay/hyperswitch/commit/02652a2519d6372e8ef7dcfe99a86222dfeca5d6))

### Miscellaneous Tasks

- **env:** Update Iatapay env to use Sandbox URL instead of Prod ([#3644](https://github.com/juspay/hyperswitch/pull/3644)) ([`8853a60`](https://github.com/juspay/hyperswitch/commit/8853a60bf4e2ed2490c60df9eaac2a8e46552b96))

**Full Changelog:** [`2024.02.13.0...2024.02.14.0`](https://github.com/juspay/hyperswitch/compare/2024.02.13.0...2024.02.14.0)

- - -

## 2024.02.13.0

### Features

- **pm_list:** Add required fields for giropay ([#3194](https://github.com/juspay/hyperswitch/pull/3194)) ([`33df352`](https://github.com/juspay/hyperswitch/commit/33df3520d1daa3e399b567b85f6a75d1b10bca13))
- **router:** Add `delete_evidence` api for disputes ([#3608](https://github.com/juspay/hyperswitch/pull/3608)) ([`1dc660f`](https://github.com/juspay/hyperswitch/commit/1dc660f80453306e86a3ea77d09829118100b59b))
- **stripe:** Send billing address to stripe for card payment ([#3611](https://github.com/juspay/hyperswitch/pull/3611)) ([`67df984`](https://github.com/juspay/hyperswitch/commit/67df984c27841ee303eae6ba55577d8bf1ef68fa))

### Bug Fixes

- **payment_link:** Changed media screen queries size for web to mobile view ([#3574](https://github.com/juspay/hyperswitch/pull/3574)) ([`cc6759b`](https://github.com/juspay/hyperswitch/commit/cc6759bd2d4207ad874a69546cb0a48db70b8629))
- **payment_methods:**
  - Unmask last4 digits of card when listing payment methods for customer ([#3617](https://github.com/juspay/hyperswitch/pull/3617)) ([`834142e`](https://github.com/juspay/hyperswitch/commit/834142e690871e5cc8e48c2fed08621e325d5d8f))
  - Unmask last4 when metadata changed during /payments ([#3633](https://github.com/juspay/hyperswitch/pull/3633)) ([`8b1206d`](https://github.com/juspay/hyperswitch/commit/8b1206d31c6c3490c96212158252f2858e5d3f7c))

### Refactors

- Introducing `hyperswitch_interface` crates ([#3536](https://github.com/juspay/hyperswitch/pull/3536)) ([`b6754a7`](https://github.com/juspay/hyperswitch/commit/b6754a7de87a417ca3f95822e970cb92b741cb95))

### Miscellaneous Tasks

- **configs:** [Volt] Add configs for wasm for production ([#3406](https://github.com/juspay/hyperswitch/pull/3406)) ([`a9749c9`](https://github.com/juspay/hyperswitch/commit/a9749c93a579aa063a96e367e92232354f977fa6))
- Address Rust 1.76 clippy lints ([#3605](https://github.com/juspay/hyperswitch/pull/3605)) ([`c55eb0a`](https://github.com/juspay/hyperswitch/commit/c55eb0afca9d43866378e8e0891ba8118a3dca39))
- Chore(deps): bump the cargo group across 1 directories with 1 update ([#3624](https://github.com/juspay/hyperswitch/pull/3624)) ([`97e9e30`](https://github.com/juspay/hyperswitch/commit/97e9e30dbed74864ecb140dccd3c61c4b28931f8))

**Full Changelog:** [`2024.02.12.0...2024.02.13.0`](https://github.com/juspay/hyperswitch/compare/2024.02.12.0...2024.02.13.0)

- - -

## 2024.02.12.0

### Features

- **user:** Implement force password reset ([#3572](https://github.com/juspay/hyperswitch/pull/3572)) ([`cfa10aa`](https://github.com/juspay/hyperswitch/commit/cfa10aa60ef16d2302787f7ecf7c129228fc0549))
- **users:** Add transfer org ownership API ([#3603](https://github.com/juspay/hyperswitch/pull/3603)) ([`b9c29e7`](https://github.com/juspay/hyperswitch/commit/b9c29e7fd3bdc5e582a2dddbb98f3d2dbda72dd6))

### Refactors

- **webhooks:** Remove unnecessary clones and lazy evaluations ([#3596](https://github.com/juspay/hyperswitch/pull/3596)) ([`bebaf41`](https://github.com/juspay/hyperswitch/commit/bebaf413f2ab1925f93f386ca4b49604542d871b))

**Full Changelog:** [`2024.02.09.1...2024.02.12.0`](https://github.com/juspay/hyperswitch/compare/2024.02.09.1...2024.02.12.0)

- - -

## 2024.02.09.1

### Bug Fixes

- **core:** Add column mandate_data for storing the details of a mandate in PaymentAttempt ([#3606](https://github.com/juspay/hyperswitch/pull/3606)) ([`74f3721`](https://github.com/juspay/hyperswitch/commit/74f3721ccd0cceac6ae8e751cb83784d2f00a283))
- **postman:** Fix failing postman tests and send a proper error message ([#3601](https://github.com/juspay/hyperswitch/pull/3601)) ([`3cef73b`](https://github.com/juspay/hyperswitch/commit/3cef73b9d8b35cb337757e29e78d22bcbe72faac))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`155aa9d`](https://github.com/juspay/hyperswitch/commit/155aa9d1192c3632c5678a958c4bb89f7861c636))

**Full Changelog:** [`2024.02.09.0...2024.02.09.1`](https://github.com/juspay/hyperswitch/compare/2024.02.09.0...2024.02.09.1)

- - -

## 2024.02.09.0

### Features

- **permissions:** Permsision Info Ordering Change ([#3594](https://github.com/juspay/hyperswitch/pull/3594)) ([`96f82cb`](https://github.com/juspay/hyperswitch/commit/96f82cb21233677968aade844db91f91e3918843))
- Adding refunds type to api_event_logs api to fetch refunds audit trail ([#3503](https://github.com/juspay/hyperswitch/pull/3503)) ([`c2b2b65`](https://github.com/juspay/hyperswitch/commit/c2b2b65b9cfc43b5999888635c7b03b1d2de78b3))

### Refactors

- **payment_methods:** Handle card duplication ([#3146](https://github.com/juspay/hyperswitch/pull/3146)) ([`dd5630f`](https://github.com/juspay/hyperswitch/commit/dd5630f070db28051a3dd59a66f0a4ee6777e38f))
- **user_role:** Change update user role request to take `email` instead of `user_id` ([#3530](https://github.com/juspay/hyperswitch/pull/3530)) ([`edd6806`](https://github.com/juspay/hyperswitch/commit/edd6806f97b8d400f1215d845023bb0d7c06aaca))

### Documentation

- Add list mandates for customer ([#3592](https://github.com/juspay/hyperswitch/pull/3592)) ([`3a869a2`](https://github.com/juspay/hyperswitch/commit/3a869a2d5731a2393a687ed7773eda5344bd8e3f))

**Full Changelog:** [`2024.02.08.0...2024.02.09.0`](https://github.com/juspay/hyperswitch/compare/2024.02.08.0...2024.02.09.0)

- - -

## 2024.02.08.0

### Features

- **core:**
  - Routes to toggle blocklist ([#3568](https://github.com/juspay/hyperswitch/pull/3568)) ([`fbe84b2`](https://github.com/juspay/hyperswitch/commit/fbe84b2a334cfb744ae4f27b1eadc892c7f9b164))
  - Decide flow based on setup_future_usage ([#3569](https://github.com/juspay/hyperswitch/pull/3569)) ([`ef302dd`](https://github.com/juspay/hyperswitch/commit/ef302dd3983674c9df47812d3c398a7e7b423257))
  - Add config for update_mandate_flow ([#3542](https://github.com/juspay/hyperswitch/pull/3542)) ([`14c0a2b`](https://github.com/juspay/hyperswitch/commit/14c0a2b03f34ae4359ee6a3918b76466eda25320))
- **payouts:** Add Wallet to Payouts ([#3502](https://github.com/juspay/hyperswitch/pull/3502)) ([`3af6aaf`](https://github.com/juspay/hyperswitch/commit/3af6aaf28e92780679eb0314eb3e95803b9c3113))

### Bug Fixes

- **payouts:** Saved payment methods list for bank details ([#3507](https://github.com/juspay/hyperswitch/pull/3507)) ([`a15e7ae`](https://github.com/juspay/hyperswitch/commit/a15e7ae9b156659e61de752ca94b6f43932d9de5))
- **router:** Added validation check to number of workers in config ([#3533](https://github.com/juspay/hyperswitch/pull/3533)) ([`c0e31ed`](https://github.com/juspay/hyperswitch/commit/c0e31ed1df6cd1f17727c9ebf9d308ede02f2228))

### Refactors

- **connector:** [Adyen] Status mapping based on Payment method Type ([#3567](https://github.com/juspay/hyperswitch/pull/3567)) ([`ab6b5ab`](https://github.com/juspay/hyperswitch/commit/ab6b5ab7b4cc95ec4f691eda865ed64472cb1f4a))
- **users:** Change list roles api to also send inactive merchants ([#3583](https://github.com/juspay/hyperswitch/pull/3583)) ([`cef1643`](https://github.com/juspay/hyperswitch/commit/cef1643af54f128e68abbf4cdc9654df3b9a69e5))
- [Noon] add new field max_amount to mandate request ([#3481](https://github.com/juspay/hyperswitch/pull/3481)) ([`926d084`](https://github.com/juspay/hyperswitch/commit/926d084e44ed6f7c83e94e60ea9da35167e499b0))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`f10b65e`](https://github.com/juspay/hyperswitch/commit/f10b65e88ee5b0fc929a717eacdbbf2fc1f0848b))

**Full Changelog:** [`2024.02.07.0...2024.02.08.0`](https://github.com/juspay/hyperswitch/compare/2024.02.07.0...2024.02.08.0)

- - -

## 2024.02.07.0

### Features

- **connect:** [NMI] Use connector_response_reference_id as reference to merchant ([#2702](https://github.com/juspay/hyperswitch/pull/2702)) ([`683c1b8`](https://github.com/juspay/hyperswitch/commit/683c1b81c5a30ac0df93664805b78a8e44d49acc))
- **connector:** Send metadata in payment authorize request for noon nmi cryptopay ([#3325](https://github.com/juspay/hyperswitch/pull/3325)) ([`ebe4ac3`](https://github.com/juspay/hyperswitch/commit/ebe4ac30a8f8f8dda7f052cb4a3788d70417aa17))
- **router:** Block list spm customer for payment link flow ([#3500](https://github.com/juspay/hyperswitch/pull/3500)) ([`6304bda`](https://github.com/juspay/hyperswitch/commit/6304bda442be68226097fd8dcc28426b74264ab0))

### Bug Fixes

- **connector:** [Stripe] capture error message and error code for failed payment, capture, void and refunds ([#3237](https://github.com/juspay/hyperswitch/pull/3237)) ([`2c52b37`](https://github.com/juspay/hyperswitch/commit/2c52b377e05b6e6296958078dd0464a49c4981a9))
- **merchant_connector_account:** Change error to DuplicateMerchantAccount ([#3496](https://github.com/juspay/hyperswitch/pull/3496)) ([`c0d910f`](https://github.com/juspay/hyperswitch/commit/c0d910f50ebe9cf387b08ecbdb86f2f60346c0cb))
- Auto retry once for connection closed ([#3426](https://github.com/juspay/hyperswitch/pull/3426)) ([`94e9b26`](https://github.com/juspay/hyperswitch/commit/94e9b26854948fe3ff7b0d96b754b5f0c9cac31a))

### Refactors

- **blocklist:** Separate utility function & kill switch for validating data in blocklist ([#3360](https://github.com/juspay/hyperswitch/pull/3360)) ([`0a97a1e`](https://github.com/juspay/hyperswitch/commit/0a97a1eb6382a1aa465ac5a1dc792ea4e763511a))
- **configs:** [Payme] Development config for 3DS ([#3555](https://github.com/juspay/hyperswitch/pull/3555)) ([`3705f77`](https://github.com/juspay/hyperswitch/commit/3705f77ee445acd5ce555a370b375b19d20ce3d4))

**Full Changelog:** [`2024.02.06.0...2024.02.07.0`](https://github.com/juspay/hyperswitch/compare/2024.02.06.0...2024.02.07.0)

- - -

## 2024.02.06.0

### Features

- **connector:** [Adyen] Use connector_request_reference_id as reference to Payments ([#3547](https://github.com/juspay/hyperswitch/pull/3547)) ([`c2eecce`](https://github.com/juspay/hyperswitch/commit/c2eecce1e803de308dcfcf774aa8aa2323cc96ec))

### Bug Fixes

- **connector:** [NMI] Handle empty response in psync and error response in complete authorize ([#3548](https://github.com/juspay/hyperswitch/pull/3548)) ([`a0fcef3`](https://github.com/juspay/hyperswitch/commit/a0fcef3f04cab75cf05154ef16fd26ab5a3783b9))
- **router:** Handle empty body parse failures in bad request logger middleware ([#3541](https://github.com/juspay/hyperswitch/pull/3541)) ([`be22d60`](https://github.com/juspay/hyperswitch/commit/be22d60ddac18d9fb3032f72247634799e8f4ceb))
- Add `profile_id` in dispute ([#3486](https://github.com/juspay/hyperswitch/pull/3486)) ([`0d5cd71`](https://github.com/juspay/hyperswitch/commit/0d5cd711b245fb69d0f35830aa1ba2f0b8a297cc))
- Return currency in payment methods list response ([#3516](https://github.com/juspay/hyperswitch/pull/3516)) ([`a9c0d0c`](https://github.com/juspay/hyperswitch/commit/a9c0d0c55492c14a4a10283ffd8deae04c8ea853))

**Full Changelog:** [`2024.02.05.0...2024.02.06.0`](https://github.com/juspay/hyperswitch/compare/2024.02.05.0...2024.02.06.0)

- - -

## 2024.02.05.0

### Features

- **connector-config:** [Volt] Add config changes for open_banking_uk ([#3529](https://github.com/juspay/hyperswitch/pull/3529)) ([`11bc891`](https://github.com/juspay/hyperswitch/commit/11bc891fd41809b3cefb9004b161d1f9c30ce68c))
- **user:** Add support for resend invite ([#3523](https://github.com/juspay/hyperswitch/pull/3523)) ([`cf0e0b3`](https://github.com/juspay/hyperswitch/commit/cf0e0b330e4c62860f645bcb61d96b07c9f4fb7b))
- Add deep health check for drainer ([#3396](https://github.com/juspay/hyperswitch/pull/3396)) ([`63c383f`](https://github.com/juspay/hyperswitch/commit/63c383f5a2b8da36d82e5563bddc5878d4b5bef5))

### Bug Fixes

- Add outgoing checks for scheduler ([#3526](https://github.com/juspay/hyperswitch/pull/3526)) ([`d283053`](https://github.com/juspay/hyperswitch/commit/d283053e5eb2dab6cfdaacc3012d50199fb03175))

### Refactors

- **connector:** [Noon] change error message from not supported to not implemented ([#2849](https://github.com/juspay/hyperswitch/pull/2849)) ([`892b04f`](https://github.com/juspay/hyperswitch/commit/892b04f805c219e2cf7cbe5736aef19909e986f7))
- Rename `kms` feature flag to `aws_kms` ([#3249](https://github.com/juspay/hyperswitch/pull/3249)) ([`91519d8`](https://github.com/juspay/hyperswitch/commit/91519d846219a878c3c87ced466337ace02e99c6))

**Full Changelog:** [`2024.02.02.0...2024.02.05.0`](https://github.com/juspay/hyperswitch/compare/2024.02.02.0...2024.02.05.0)

- - -

## 2024.02.02.0

### Features

- **configs:** [Noon] Add applepay mandate configs ([#3508](https://github.com/juspay/hyperswitch/pull/3508)) ([`7cf6c8c`](https://github.com/juspay/hyperswitch/commit/7cf6c8c0b9c4042f2e6b9277b7c75c85546821f7))
- Add deep health check for scheduler ([#3304](https://github.com/juspay/hyperswitch/pull/3304)) ([`170e10c`](https://github.com/juspay/hyperswitch/commit/170e10cb8e0880737585284dd43437f549c019d3))
- Add healthcheck for outgoing request ([#3519](https://github.com/juspay/hyperswitch/pull/3519)) ([`54fb61e`](https://github.com/juspay/hyperswitch/commit/54fb61eeebec503f599774fe9e97f6b6ce3f1458))

### Bug Fixes

- **core:** Fix mandate_details to store some value only if mandate_data struct is present ([#3525](https://github.com/juspay/hyperswitch/pull/3525)) ([`78fdad2`](https://github.com/juspay/hyperswitch/commit/78fdad218ca3ae3c7410dfb8a7a8a5e542adff1c))
- **logging:** Add an end log line for `LogSpanInitializer` ([#3528](https://github.com/juspay/hyperswitch/pull/3528)) ([`13be7e6`](https://github.com/juspay/hyperswitch/commit/13be7e6f8771a1128e3c0c5b189c91d9a0dd1416))

### Refactors

- **connector:** [CYBERSOURCE] Remove default case for Cybersource ([#2705](https://github.com/juspay/hyperswitch/pull/2705)) ([`1828ea6`](https://github.com/juspay/hyperswitch/commit/1828ea6187c46d9c18dc8a0b5224387403b998e2))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`1deb37e`](https://github.com/juspay/hyperswitch/commit/1deb37ebd1128041ded64a4966a2d47a61e8c499))
- Add file storage config in env_specific toml ([#3512](https://github.com/juspay/hyperswitch/pull/3512)) ([`20efc30`](https://github.com/juspay/hyperswitch/commit/20efc3020ac389199eed13154f070685417ef82a))

**Full Changelog:** [`2024.02.01.0...2024.02.02.0`](https://github.com/juspay/hyperswitch/compare/2024.02.01.0...2024.02.02.0)

- - -

## 2024.02.01.0

### Features

- **dashboard_metadata:** Add email alert for Prod Intent ([#3482](https://github.com/juspay/hyperswitch/pull/3482)) ([`94cd7b6`](https://github.com/juspay/hyperswitch/commit/94cd7b689758a71e13a3eaa655335e658d13afc8))
- **pm_list:** Add required fields for google pay ([#3196](https://github.com/juspay/hyperswitch/pull/3196)) ([`7f2c434`](https://github.com/juspay/hyperswitch/commit/7f2c434bd29d337dadde8b71a9137797f1c03ec0))

### Bug Fixes

- **configs:** Add configs for Payme 3DS ([#3415](https://github.com/juspay/hyperswitch/pull/3415)) ([`58771b8`](https://github.com/juspay/hyperswitch/commit/58771b8985a53c83185805f770fee26c5836c645))

### Refactors

- **connector:**
  - [NMI] change error message from not supported to not implemented ([#2848](https://github.com/juspay/hyperswitch/pull/2848)) ([`7575341`](https://github.com/juspay/hyperswitch/commit/757534104ee0411a887c993e45cc1fb883e82992))
  - [Paypal] Change error message from NotSupported to NotImplemented ([#2877](https://github.com/juspay/hyperswitch/pull/2877)) ([`7251f64`](https://github.com/juspay/hyperswitch/commit/7251f6474fdac3575202971e55638c435ca5c4c8))
  - [Adyen] change expiresAt time from string to unixtimestamp ([#3506](https://github.com/juspay/hyperswitch/pull/3506)) ([`b7c0f9a`](https://github.com/juspay/hyperswitch/commit/b7c0f9aa098c880314a529bc10015256ce2139f7))

### Miscellaneous Tasks

- **connector_events_fields:** Added refund_id, dispute_id to connector events ([#3424](https://github.com/juspay/hyperswitch/pull/3424)) ([`90a2462`](https://github.com/juspay/hyperswitch/commit/90a24625ce312e4e7681cf4cc470e6365a052f8a))

**Full Changelog:** [`2024.01.31.1...2024.02.01.0`](https://github.com/juspay/hyperswitch/compare/2024.01.31.1...2024.02.01.0)

- - -

## 2024.01.31.1

### Features

- **users:**
  - Added blacklist for users ([#3469](https://github.com/juspay/hyperswitch/pull/3469)) ([`e331d2d`](https://github.com/juspay/hyperswitch/commit/e331d2d5569405b89052c6bb59f7e755523f6f15))
  - Add `merchant_id` in `EmailToken` and change user status in reset password ([#3473](https://github.com/juspay/hyperswitch/pull/3473)) ([`db3d53f`](https://github.com/juspay/hyperswitch/commit/db3d53ff1d8b42d107fafe7a6efe7ec9f155d5a0))
- Add deep health check for analytics ([#3438](https://github.com/juspay/hyperswitch/pull/3438)) ([`7597f3b`](https://github.com/juspay/hyperswitch/commit/7597f3b692124a762c3b212b604938be2d64175a))

### Bug Fixes

- **connector:** [Trustpay] add merchant_id in gpay session response for trustpay ([#3471](https://github.com/juspay/hyperswitch/pull/3471)) ([`20568dc`](https://github.com/juspay/hyperswitch/commit/20568dc976687b8b2bfba12ab2db8926cf1c14ed))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`a4b9782`](https://github.com/juspay/hyperswitch/commit/a4b97828be103d601a5007f8e4274837faa6886f))

**Full Changelog:** [`2024.01.31.0...2024.01.31.1`](https://github.com/juspay/hyperswitch/compare/2024.01.31.0...2024.01.31.1)

- - -

## 2024.01.31.0

### Features

- **connector:** [noon] add revoke mandate ([#3487](https://github.com/juspay/hyperswitch/pull/3487)) ([`b5bc8c4`](https://github.com/juspay/hyperswitch/commit/b5bc8c4e7cfdde8251ed0e2e3835ed5e3f1435c4))

### Bug Fixes

- **connector:** [BOA/Cybersource] Handle Invalid Api Secret ([#3485](https://github.com/juspay/hyperswitch/pull/3485)) ([`224c1cf`](https://github.com/juspay/hyperswitch/commit/224c1cf2a421441433097618cc1dd3db224d5915))
- **user:** Change permission for sample data ([#3462](https://github.com/juspay/hyperswitch/pull/3462)) ([`610c1c5`](https://github.com/juspay/hyperswitch/commit/610c1c575253ddf7a1a31ef941efaae2dd676b48))

### Refactors

- **core:** Restrict requires_customer_action in confirm ([#3235](https://github.com/juspay/hyperswitch/pull/3235)) ([`d2accde`](https://github.com/juspay/hyperswitch/commit/d2accdef410319733d6174057bdca468bde1ae83))

### Miscellaneous Tasks

- **config:** [ADYEN] Add configs for PIX in WASM ([#3498](https://github.com/juspay/hyperswitch/pull/3498)) ([`9821935`](https://github.com/juspay/hyperswitch/commit/9821935933e178765b3b0d0bcbfdf4ab041c3bc2))

**Full Changelog:** [`2024.01.30.1...2024.01.31.0`](https://github.com/juspay/hyperswitch/compare/2024.01.30.1...2024.01.31.0)

- - -

## 2024.01.30.1

### Features

- **config:** Add iDEAL and Sofort Env Configs ([#3492](https://github.com/juspay/hyperswitch/pull/3492)) ([`46c1822`](https://github.com/juspay/hyperswitch/commit/46c1822d0e367e59420c9d087428bc3b12794445))
- **connector:**
  - [Bluesnap] Metadata to connector metadata mapping ([#3331](https://github.com/juspay/hyperswitch/pull/3331)) ([`b2afdc3`](https://github.com/juspay/hyperswitch/commit/b2afdc35465426bd11428d8d4ac743617a443128))
  - [Stripe] Metadata to connector metadata mapping ([#3295](https://github.com/juspay/hyperswitch/pull/3295)) ([`864a8d7`](https://github.com/juspay/hyperswitch/commit/864a8d7b02acda5ea593cae83594962ea249c16d))
- **core:** Update card_details for an existing mandate ([#3452](https://github.com/juspay/hyperswitch/pull/3452)) ([`02074df`](https://github.com/juspay/hyperswitch/commit/02074dfc23f1a126e76935ba5311c6aed6590ca5))
- **pm_list:** Add required fields for sofort ([#3192](https://github.com/juspay/hyperswitch/pull/3192)) ([`3d55e3b`](https://github.com/juspay/hyperswitch/commit/3d55e3ba45619978e8ca9e5012c156dc017d2879))
- **users:** Signin and Verify Email changes for User Invitation changes ([#3420](https://github.com/juspay/hyperswitch/pull/3420)) ([`d91da89`](https://github.com/juspay/hyperswitch/commit/d91da89065a6870f05e1ff9db007d16a58454c84))

### Bug Fixes

- **logging:** Add flow to persistent logs fields ([#3472](https://github.com/juspay/hyperswitch/pull/3472)) ([`ac49103`](https://github.com/juspay/hyperswitch/commit/ac491038b16c77fc7f2249042b35dfb1d58e653d))
- Empty payment attempts on payment retrieve ([#3447](https://github.com/juspay/hyperswitch/pull/3447)) ([`bec4f2a`](https://github.com/juspay/hyperswitch/commit/bec4f2a24e2236f7814119a6ebf0363cbf598540))

### Refactors

- **payment_link:** Segregated payment link in html css js files, sdk over flow issue, surcharge bug, block SPM customer call for payment link ([#3410](https://github.com/juspay/hyperswitch/pull/3410)) ([`a7bc8c6`](https://github.com/juspay/hyperswitch/commit/a7bc8c655f5b745dccd4d818ac3ceb08c3b80c0e))
- **settings:** Make the function to deserialize hashsets more generic ([#3104](https://github.com/juspay/hyperswitch/pull/3104)) ([`87191d6`](https://github.com/juspay/hyperswitch/commit/87191d687cd66bf096bfb98ffe51a805b4b76a03))
- Add support for extending file storage to other schemes and provide a runtime flag for the same ([#3348](https://github.com/juspay/hyperswitch/pull/3348)) ([`a9638d1`](https://github.com/juspay/hyperswitch/commit/a9638d118e0b68653fef3bec2ce8aa3c47feedd3))

### Miscellaneous Tasks

- **analytics:**
  - Adding status code to connector Kafka events ([#3393](https://github.com/juspay/hyperswitch/pull/3393)) ([`d6807ab`](https://github.com/juspay/hyperswitch/commit/d6807abba46136eabadcbfbc51bce421144dca2c))
  - Adding dispute id to api log events ([#3450](https://github.com/juspay/hyperswitch/pull/3450)) ([`937aea9`](https://github.com/juspay/hyperswitch/commit/937aea906e759e6e8a76a424db99ed052d46b7d2))
- **kv:** Add metrics while pushing to stream ([#3364](https://github.com/juspay/hyperswitch/pull/3364)) ([`8c0c49c`](https://github.com/juspay/hyperswitch/commit/8c0c49c6bb02d4ec58242bc90eadfb267c24481e))

**Full Changelog:** [`2024.01.30.0...2024.01.30.1`](https://github.com/juspay/hyperswitch/compare/2024.01.30.0...2024.01.30.1)

- - -

## 2024.01.30.0

### Features

- **router:** Add request_details logger middleware for 400 bad requests ([#3414](https://github.com/juspay/hyperswitch/pull/3414)) ([`dd0d2dc`](https://github.com/juspay/hyperswitch/commit/dd0d2dc2dd9a6263bbb8a99d1f0b2077f38dd621))

### Refactors

- **openapi:** Move openapi to separate crate to decrease compile times ([#3110](https://github.com/juspay/hyperswitch/pull/3110)) ([`7d8d68f`](https://github.com/juspay/hyperswitch/commit/7d8d68faba55dfcb2886c63ae7969ebd4b9ec98c))

### Miscellaneous Tasks

- **configs:** [NMI] add wasm changes for prod dashboard ([#3470](https://github.com/juspay/hyperswitch/pull/3470)) ([`3fbffdc`](https://github.com/juspay/hyperswitch/commit/3fbffdc242dafe7983c542573b7c6362f99331e6))

**Full Changelog:** [`2024.01.29.0...2024.01.30.0`](https://github.com/juspay/hyperswitch/compare/2024.01.29.0...2024.01.30.0)

- - -

## 2024.01.29.0

### Features

- **connector:** [Adyen] Add support for PIX Payment Method ([#3236](https://github.com/juspay/hyperswitch/pull/3236)) ([`fc6e68f`](https://github.com/juspay/hyperswitch/commit/fc6e68f7f07bf2d48466fa493596c0db02d7550a))
- **core:**
  - [CYBERSOURCE] Add original authorized amount in router data ([#3417](https://github.com/juspay/hyperswitch/pull/3417)) ([`47fbe48`](https://github.com/juspay/hyperswitch/commit/47fbe486cec252b8befca38f1b7ea77cc0823ee5))
  - Add outgoing webhook for manual `partial_capture` events ([#3388](https://github.com/juspay/hyperswitch/pull/3388)) ([`d5e9866`](https://github.com/juspay/hyperswitch/commit/d5e9866b522bad3e62f6f6c0d7993f5dcc2939af))
- **logging:** Add a logging middleware to log all api requests ([#3437](https://github.com/juspay/hyperswitch/pull/3437)) ([`c2946cf`](https://github.com/juspay/hyperswitch/commit/c2946cfe05ffa81a66643e04eff5e89b545d2d43))
- **user:**
  - Add support to delete user ([#3374](https://github.com/juspay/hyperswitch/pull/3374)) ([`7777710`](https://github.com/juspay/hyperswitch/commit/777771048a8144aac9e2f837c85531e139ecc125))
  - Support multiple invites ([#3422](https://github.com/juspay/hyperswitch/pull/3422)) ([`a59ac7d`](https://github.com/juspay/hyperswitch/commit/a59ac7d5b98f27f5fb34206c20ef9c37a07259a3))

### Bug Fixes

- **connector:**
  - Use `ConnectorError::InvalidConnectorConfig` for an invalid `CoinbaseConnectorMeta` ([#3168](https://github.com/juspay/hyperswitch/pull/3168)) ([`d827c9a`](https://github.com/juspay/hyperswitch/commit/d827c9af29b8516f379e648e00f4ab307ae1a34d))
  - Fix connector template script ([#3453](https://github.com/juspay/hyperswitch/pull/3453)) ([`9a54838`](https://github.com/juspay/hyperswitch/commit/9a54838b0529013ab8f449ec6b347a104b55f8f7))
  - [HELCIM] Handle 4XX Errors ([#3458](https://github.com/juspay/hyperswitch/pull/3458)) ([`ec859ea`](https://github.com/juspay/hyperswitch/commit/ec859eabbfb8a511f0fffd30a47a144fb07f2886))
- **core:** Return surcharge in payment method list response if passed in create request ([#3363](https://github.com/juspay/hyperswitch/pull/3363)) ([`3507ad6`](https://github.com/juspay/hyperswitch/commit/3507ad60b2f1fd84d32eb4d97fe0a847db6f2045))
- **euclid_wasm:** Include `payouts` feature in `default` features ([#3392](https://github.com/juspay/hyperswitch/pull/3392)) ([`b45e4ca`](https://github.com/juspay/hyperswitch/commit/b45e4ca2a3788823701bdeac2e2a8c1147bb071a))

### Refactors

- **connector:**
  - [Iatapay] refactor authorize flow and fix payment status mapping ([#2409](https://github.com/juspay/hyperswitch/pull/2409)) ([`f0c7bb9`](https://github.com/juspay/hyperswitch/commit/f0c7bb9a5228f2ee31858fea07abe4ecee9b78a2))
  - Use utility function to raise payment method not implemented errors ([#1871](https://github.com/juspay/hyperswitch/pull/1871)) ([`66cd5b2`](https://github.com/juspay/hyperswitch/commit/66cd5b2fc9a32085608ed34e0af477dcafe4b957))
- **payouts:** Propagate `Not Implemented` error ([#3429](https://github.com/juspay/hyperswitch/pull/3429)) ([`5ab4437`](https://github.com/juspay/hyperswitch/commit/5ab44377b84941b8b59f9e73b1d1f0c3889eb02b))

### Miscellaneous Tasks

- **configs:** [Cashtocode] wasm changes for CAD, CHF currency ([#3461](https://github.com/juspay/hyperswitch/pull/3461)) ([`10055c1`](https://github.com/juspay/hyperswitch/commit/10055c1a7354faae8d0f504e0851d2046df5734a))

**Full Changelog:** [`2024.01.25.0...2024.01.29.0`](https://github.com/juspay/hyperswitch/compare/2024.01.25.0...2024.01.29.0)

- - -

## 2024.01.25.0

### Refactors

- **configs:** Add configs for deployments to environments ([#3265](https://github.com/juspay/hyperswitch/pull/3265)) ([`77c1bbb`](https://github.com/juspay/hyperswitch/commit/77c1bbb5a3fe3244cd988ac1260a4a31ae7fcd20))

**Full Changelog:** [`2024.01.24.1...2024.01.25.0`](https://github.com/juspay/hyperswitch/compare/2024.01.24.1...2024.01.25.0)

- - -

## 2024.01.24.1

### Features

- **hashicorp:** Implement hashicorp secrets manager solution ([#3297](https://github.com/juspay/hyperswitch/pull/3297)) ([`629d546`](https://github.com/juspay/hyperswitch/commit/629d546aa7c774e86d609abec3b3ab5cf0d100a7))

### Refactors

- **Router:** [Noon] revert adding new field max_amount to mandate request ([#3435](https://github.com/juspay/hyperswitch/pull/3435)) ([`4cd65a2`](https://github.com/juspay/hyperswitch/commit/4cd65a24f70fdef160eb2d87654f1e30538c3339))
- **compatibility:** Revert add multiuse mandates support in stripe compatibility ([#3436](https://github.com/juspay/hyperswitch/pull/3436)) ([`8a019f0`](https://github.com/juspay/hyperswitch/commit/8a019f08acf74e04c3ae9c8790dd481301bdcfee))

### Miscellaneous Tasks

- **ckh-source:** Updated ckh analytics source tables ([#3397](https://github.com/juspay/hyperswitch/pull/3397)) ([`3f343d3`](https://github.com/juspay/hyperswitch/commit/3f343d36bff7ce8f73602a2391d205367d5581c7))

**Full Changelog:** [`2024.01.24.0...2024.01.24.1`](https://github.com/juspay/hyperswitch/compare/2024.01.24.0...2024.01.24.1)

- - -

## 2024.01.24.0

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`7885b2a`](https://github.com/juspay/hyperswitch/commit/7885b2a213f474da3e018ddeb56bc6e407c48471))

**Full Changelog:** [`2024.01.23.0...2024.01.24.0`](https://github.com/juspay/hyperswitch/compare/2024.01.23.0...2024.01.24.0)

- - -

## 2024.01.23.0

### Features

- **compatibility:** Add multiuse mandates support in stripe compatibility ([#3425](https://github.com/juspay/hyperswitch/pull/3425)) ([`4a8104e`](https://github.com/juspay/hyperswitch/commit/4a8104e5f8dd2cfd03de4055baf1256cb7533895))

**Full Changelog:** [`2024.01.22.1...2024.01.23.0`](https://github.com/juspay/hyperswitch/compare/2024.01.22.1...2024.01.23.0)

- - -

## 2024.01.22.1

### Features

- **core:** Send `customer_name` to connectors when creating customer ([#3380](https://github.com/juspay/hyperswitch/pull/3380)) ([`7813cee`](https://github.com/juspay/hyperswitch/commit/7813ceece2081b73f1374e2ee5a9a673f0b72127))

### Miscellaneous Tasks

- Chore(deps): bump the cargo group across 1 directories with 3 updates ([#3409](https://github.com/juspay/hyperswitch/pull/3409)) ([`6c46e9c`](https://github.com/juspay/hyperswitch/commit/6c46e9c19b304bb11f304e60c46e8abf67accf6d))

**Full Changelog:** [`2024.01.22.0...2024.01.22.1`](https://github.com/juspay/hyperswitch/compare/2024.01.22.0...2024.01.22.1)

- - -

## 2024.01.22.0

### Features

- **user_roles:** Add accept invitation API and `UserJWTAuth` ([#3365](https://github.com/juspay/hyperswitch/pull/3365)) ([`a47372a`](https://github.com/juspay/hyperswitch/commit/a47372a451b60defda35fa212565b889ed5b2d2b))

### Documentation

- Add link to api docs ([#3405](https://github.com/juspay/hyperswitch/pull/3405)) ([`4e1e78e`](https://github.com/juspay/hyperswitch/commit/4e1e78ecd962f4b34fa04f611f03e8e6f6e1bd7c))

**Full Changelog:** [`2024.01.19.1...2024.01.22.0`](https://github.com/juspay/hyperswitch/compare/2024.01.19.1...2024.01.22.0)

- - -

## 2024.01.19.1

### Bug Fixes

- **connector:** [CRYPTOPAY] Fix header generation for PSYNC ([#3402](https://github.com/juspay/hyperswitch/pull/3402)) ([`ec16ed0`](https://github.com/juspay/hyperswitch/commit/ec16ed0f82f258c5699d54a386f67aff06c0d144))
- **frm:** Update FRM manual review flow ([#3176](https://github.com/juspay/hyperswitch/pull/3176)) ([`5255ba9`](https://github.com/juspay/hyperswitch/commit/5255ba9170c633899cd4c3bbe24a44b429546f15))

### Refactors

- Rename `s3` feature flag to `aws_s3` ([#3341](https://github.com/juspay/hyperswitch/pull/3341)) ([`1c04ac7`](https://github.com/juspay/hyperswitch/commit/1c04ac751240f5c931df0f282af1e0ad745e9509))

**Full Changelog:** [`2024.01.19.0...2024.01.19.1`](https://github.com/juspay/hyperswitch/compare/2024.01.19.0...2024.01.19.1)

- - -

## 2024.01.19.0

### Features

- **users:**
  - Add `preferred_merchant_id` column and update user details API ([#3373](https://github.com/juspay/hyperswitch/pull/3373)) ([`862a1b5`](https://github.com/juspay/hyperswitch/commit/862a1b5303ff304cca41d3553f652fd1091aab9b))
  - Added get role from jwt api ([#3385](https://github.com/juspay/hyperswitch/pull/3385)) ([`7516a16`](https://github.com/juspay/hyperswitch/commit/7516a16763877c03ecc35fda19388bbd021c5cc7))

### Refactors

- **recon:** Update recipient email and mail body for ProFeatureRequest ([#3381](https://github.com/juspay/hyperswitch/pull/3381)) ([`5a791aa`](https://github.com/juspay/hyperswitch/commit/5a791aaf4dc05e8ffdb60464a03b6fc41f860581))

**Full Changelog:** [`2024.01.18.1...2024.01.19.0`](https://github.com/juspay/hyperswitch/compare/2024.01.18.1...2024.01.19.0)

- - -

## 2024.01.18.1

### Bug Fixes

- **connector:**
  - Trustpay zen error mapping ([#3255](https://github.com/juspay/hyperswitch/pull/3255)) ([`e816ccf`](https://github.com/juspay/hyperswitch/commit/e816ccfbdd7b0e24464aa93421e399d63f23b17c))
  - [Cashtocode] update amount from i64 to f64 in webhook payload ([#3382](https://github.com/juspay/hyperswitch/pull/3382)) ([`059e866`](https://github.com/juspay/hyperswitch/commit/059e86607dc271c25bb3d23f5adfc7d5f21f62fb))
- **metrics:** Add TASKS_ADDED_COUNT and TASKS_RESET_COUNT metrics in router scheduler flow ([#3189](https://github.com/juspay/hyperswitch/pull/3189)) ([`b4df40d`](https://github.com/juspay/hyperswitch/commit/b4df40db25f6ea743c7a25db47e8f1d8e0d544e3))
- **user:** Fetch profile_id for sample data ([#3358](https://github.com/juspay/hyperswitch/pull/3358)) ([`2f693ad`](https://github.com/juspay/hyperswitch/commit/2f693ad1fd857280ef30c6cc0297fb926f0e79e8))

### Refactors

- **connector:** [Volt] Refactor Payments and Refunds Webhooks ([#3377](https://github.com/juspay/hyperswitch/pull/3377)) ([`acb3296`](https://github.com/juspay/hyperswitch/commit/acb329672297cd7337d0b0239e4c662257812e8a))
- **core:** Add locker config to enable or disable locker ([#3352](https://github.com/juspay/hyperswitch/pull/3352)) ([`bd5356e`](https://github.com/juspay/hyperswitch/commit/bd5356e7e7cf61f9d07fe9b67c9c5bb38fddf9c7))

**Full Changelog:** [`2024.01.18.0...2024.01.18.1`](https://github.com/juspay/hyperswitch/compare/2024.01.18.0...2024.01.18.1)

- - -

## 2024.01.18.0

### Features

- **connector_events:** Added api to fetch connector event logs ([#3319](https://github.com/juspay/hyperswitch/pull/3319)) ([`68a3a28`](https://github.com/juspay/hyperswitch/commit/68a3a280676c8309f9becffae545b134b5e1f2ea))
- **payment_method:** Add capability to store bank details using /payment_methods endpoint ([#3113](https://github.com/juspay/hyperswitch/pull/3113)) ([`01c2de2`](https://github.com/juspay/hyperswitch/commit/01c2de223f60595d77c06a59a40dfe041e02cfee))

### Bug Fixes

- **core:** Add validation for authtype and metadata in update payment connector ([#3305](https://github.com/juspay/hyperswitch/pull/3305)) ([`52f38d3`](https://github.com/juspay/hyperswitch/commit/52f38d3d5a7d035e8211e1f51c8f982232e2d7ab))
- **events:** Fix event generation for paymentmethods list ([#3337](https://github.com/juspay/hyperswitch/pull/3337)) ([`ac8d81b`](https://github.com/juspay/hyperswitch/commit/ac8d81b32b3d91b875113d32782a8c62e39ba2a8))

### Refactors

- **connector:** [cybersource] recurring mandate flow ([#3354](https://github.com/juspay/hyperswitch/pull/3354)) ([`387c1c4`](https://github.com/juspay/hyperswitch/commit/387c1c491bdc413ae361d04f0be25eaa58e72fa9))
- [Noon] adding new field max_amount to mandate request ([#3209](https://github.com/juspay/hyperswitch/pull/3209)) ([`eb2a61d`](https://github.com/juspay/hyperswitch/commit/eb2a61d8597995838f21b8233653c691118b2191))

### Miscellaneous Tasks

- **router:** Remove recon from default features ([#3370](https://github.com/juspay/hyperswitch/pull/3370)) ([`928beec`](https://github.com/juspay/hyperswitch/commit/928beecdd7fe9e09b38ffe750627ca4af94ffc93))

**Full Changelog:** [`2024.01.17.0...2024.01.18.0`](https://github.com/juspay/hyperswitch/compare/2024.01.17.0...2024.01.18.0)

- - -

## 2024.01.17.0

### Features

- **connector:** [BANKOFAMERICA] Implement 3DS flow for cards ([#3343](https://github.com/juspay/hyperswitch/pull/3343)) ([`d533c98`](https://github.com/juspay/hyperswitch/commit/d533c98b5107fb6876c11b183eb9bc382a77a2f1))
- **recon:** Add recon APIs ([#3345](https://github.com/juspay/hyperswitch/pull/3345)) ([`8678f8d`](https://github.com/juspay/hyperswitch/commit/8678f8d1448b5ce430931bfbbc269ef979d9eea7))

### Bug Fixes

- **connector_onboarding:** Check if connector exists for the merchant account and add reset tracking id API ([#3229](https://github.com/juspay/hyperswitch/pull/3229)) ([`58cc8d6`](https://github.com/juspay/hyperswitch/commit/58cc8d6109ce49d385b06c762ab3f6670f5094eb))
- **payment_link:** Added expires_on in payment response ([#3332](https://github.com/juspay/hyperswitch/pull/3332)) ([`5ad3f89`](https://github.com/juspay/hyperswitch/commit/5ad3f8939afafce3eec39704dcaa92270b384dcd))

**Full Changelog:** [`2024.01.12.1...2024.01.17.0`](https://github.com/juspay/hyperswitch/compare/2024.01.12.1...2024.01.17.0)

- - -

## 2024.01.12.1

### Miscellaneous Tasks

- **config:** Add merchant_secret config for webhooks for cashtocode and volt in wasm dashboard ([#3333](https://github.com/juspay/hyperswitch/pull/3333)) ([`57f2cff`](https://github.com/juspay/hyperswitch/commit/57f2cff75e58b0a7811492a1fdb636f59dcefbd0))
- Add api reference for blocklist ([#3336](https://github.com/juspay/hyperswitch/pull/3336)) ([`f381d86`](https://github.com/juspay/hyperswitch/commit/f381d86b7c9fa79d632991c74cab53d0181231c6))

**Full Changelog:** [`2024.01.12.0...2024.01.12.1`](https://github.com/juspay/hyperswitch/compare/2024.01.12.0...2024.01.12.1)

- - -

## 2024.01.12.0

### Features

- **connector:**
  - [BOA/Cyb] Include merchant metadata in capture and void requests ([#3308](https://github.com/juspay/hyperswitch/pull/3308)) ([`5a5400c`](https://github.com/juspay/hyperswitch/commit/5a5400cf5b539996b2f327c51d4a07b4a86fd1be))
  - [Volt] Add support for refund webhooks ([#3326](https://github.com/juspay/hyperswitch/pull/3326)) ([`e376f68`](https://github.com/juspay/hyperswitch/commit/e376f68c167a289957a4372df108797088ab1f6e))
  - [BOA/CYB] Store AVS response in connector_metadata ([#3271](https://github.com/juspay/hyperswitch/pull/3271)) ([`e75b11e`](https://github.com/juspay/hyperswitch/commit/e75b11e98ac4c8d37c842c8ee0ccf361dcb52793))
- **euclid_wasm:** Config changes for NMI ([#3329](https://github.com/juspay/hyperswitch/pull/3329)) ([`ed07c5b`](https://github.com/juspay/hyperswitch/commit/ed07c5ba90868a3132ca90d72219db3ba8978232))
- **outgoingwebhookevent:** Adding api for query to fetch outgoing webhook events log ([#3310](https://github.com/juspay/hyperswitch/pull/3310)) ([`54d44be`](https://github.com/juspay/hyperswitch/commit/54d44bef730c0679f3535f66e89e88139d70ba2e))
- **payment_link:** Added sdk layout option payment link ([#3207](https://github.com/juspay/hyperswitch/pull/3207)) ([`6117652`](https://github.com/juspay/hyperswitch/commit/61176524ca0c11c605538a1da9a267837193e1ec))
- **router:** Payment_method block ([#3056](https://github.com/juspay/hyperswitch/pull/3056)) ([`bb09613`](https://github.com/juspay/hyperswitch/commit/bb096138b5937092badd02741fb869ee35e2e3cc))
- **users:** Invite user without email ([#3328](https://github.com/juspay/hyperswitch/pull/3328)) ([`6a47063`](https://github.com/juspay/hyperswitch/commit/6a4706323c61f3722dc543993c55084dc9ff9850))
- Feat(connector): [cybersource] Implement 3DS flow for cards ([#3290](https://github.com/juspay/hyperswitch/pull/3290)) ([`6fb3b00`](https://github.com/juspay/hyperswitch/commit/6fb3b00e82d1e3c03dc1c816ffa6353cc7991a53))
- Add support for card extended bin in payment attempt ([#3312](https://github.com/juspay/hyperswitch/pull/3312)) ([`cc3eefd`](https://github.com/juspay/hyperswitch/commit/cc3eefd317117d761cdcc76804f3510952d4cec2))

### Bug Fixes

- **core:** Surcharge with saved card failure ([#3318](https://github.com/juspay/hyperswitch/pull/3318)) ([`5a1a3da`](https://github.com/juspay/hyperswitch/commit/5a1a3da7502ce9e13546b896477d82719162d5b6))
- **refund:** Add merchant_connector_id in refund ([#3303](https://github.com/juspay/hyperswitch/pull/3303)) ([`af43b07`](https://github.com/juspay/hyperswitch/commit/af43b07e4394458db478bc16e5fb8d3b0d636a31))
- **router:** Add config to avoid connector tokenization for `apple pay` `simplified flow` ([#3234](https://github.com/juspay/hyperswitch/pull/3234)) ([`4f9c04b`](https://github.com/juspay/hyperswitch/commit/4f9c04b856761b9c0486abad4c36de191da2c460))
- Update amount_capturable based on intent_status and payment flow ([#3278](https://github.com/juspay/hyperswitch/pull/3278)) ([`469ea20`](https://github.com/juspay/hyperswitch/commit/469ea20214aa7c1a3b4b86520724c2509ae37b0b))

### Refactors

- **router:**
  - Flagged order_details validation to skip validation ([#3116](https://github.com/juspay/hyperswitch/pull/3116)) ([`8626bda`](https://github.com/juspay/hyperswitch/commit/8626bda6d5aa9e7531edc7ea50ed4f30c3b7227a))
  - Restricted list payment method Customer to api-key based ([#3100](https://github.com/juspay/hyperswitch/pull/3100)) ([`9eaebe8`](https://github.com/juspay/hyperswitch/commit/9eaebe8db3d83105ef1e8fc784241e1fb795dd22))

### Miscellaneous Tasks

- Remove connector auth TOML files from `.gitignore` and `.dockerignore` ([#3330](https://github.com/juspay/hyperswitch/pull/3330)) ([`9f6ef3f`](https://github.com/juspay/hyperswitch/commit/9f6ef3f2240052053b5b7df0a13a5503d8141d56))

**Full Changelog:** [`2024.01.11.0...2024.01.12.0`](https://github.com/juspay/hyperswitch/compare/2024.01.11.0...2024.01.12.0)

- - -

## 2024.01.11.0

### Features

- **core:** Add new payments webhook events ([#3212](https://github.com/juspay/hyperswitch/pull/3212)) ([`e0e28b8`](https://github.com/juspay/hyperswitch/commit/e0e28b87c0647252918ef110cd7614c46b5cf943))
- **payment_link:** Add status page for payment link ([#3213](https://github.com/juspay/hyperswitch/pull/3213)) ([`50e4d79`](https://github.com/juspay/hyperswitch/commit/50e4d797da31b570b5920b33d77c24a21d9871e2))

### Bug Fixes

- **euclid_wasm:** Update braintree config prod ([#3288](https://github.com/juspay/hyperswitch/pull/3288)) ([`8830563`](https://github.com/juspay/hyperswitch/commit/8830563748ed20c40b7a21a66e9ad9fd02ddcf0e))

### Refactors

- **connector:** [bluesnap] add connector_txn_id fallback for webhook ([#3315](https://github.com/juspay/hyperswitch/pull/3315)) ([`a69e876`](https://github.com/juspay/hyperswitch/commit/a69e876f8212cb94202686e073005c23b1b2fc35))
- Removed basilisk feature ([#3281](https://github.com/juspay/hyperswitch/pull/3281)) ([`612f8d9`](https://github.com/juspay/hyperswitch/commit/612f8d9d5f5bcba78aa64c3128cc72be0f2860ea))

### Miscellaneous Tasks

- Nits and small code improvements found during investigation of PR#3168 ([#3259](https://github.com/juspay/hyperswitch/pull/3259)) ([`fe3cf54`](https://github.com/juspay/hyperswitch/commit/fe3cf54781302c733c1682ded2c1735544407a5f))

**Full Changelog:** [`2024.01.10.0...2024.01.11.0`](https://github.com/juspay/hyperswitch/compare/2024.01.10.0...2024.01.11.0)

- - -

## 2024.01.10.0

### Features

- **Connector:** [VOLT] Add support for Payments Webhooks ([#3155](https://github.com/juspay/hyperswitch/pull/3155)) ([`eba7896`](https://github.com/juspay/hyperswitch/commit/eba789640b72cdfbc17d0994d16ce111a1788fe5))
- **pm_list:** Add required fields for Ideal ([#3183](https://github.com/juspay/hyperswitch/pull/3183)) ([`1c3c5f6`](https://github.com/juspay/hyperswitch/commit/1c3c5f6b0cff9a0037175ba92c002cdf4249108d))

### Bug Fixes

- **connector:**
  - [BOA/CYB] Fix Metadata Error ([#3283](https://github.com/juspay/hyperswitch/pull/3283)) ([`71044a1`](https://github.com/juspay/hyperswitch/commit/71044a14ed87ac0cd7d2bb2009f0e59c79bd344c))
  - [BOA, Cybersource] capture error_code ([#3239](https://github.com/juspay/hyperswitch/pull/3239)) ([`ecf51b5`](https://github.com/juspay/hyperswitch/commit/ecf51b5e3a30f055634edfafcd36f64cef535a53))
- **outgoingwebhookevents:** Throw an error when outgoing webhook events env var not found ([#3291](https://github.com/juspay/hyperswitch/pull/3291)) ([`ee044a0`](https://github.com/juspay/hyperswitch/commit/ee044a0be811a53842c69f64c27d9995d84b7040))
- **users:** Added merchant name is list merchants ([#3289](https://github.com/juspay/hyperswitch/pull/3289)) ([`8a354f4`](https://github.com/juspay/hyperswitch/commit/8a354f42295a3167d0e846c9522bc091ebdca3f4))
- **wasm:** Fix failing `wasm-pack build` for `euclid_wasm` ([#3284](https://github.com/juspay/hyperswitch/pull/3284)) ([`5eb6711`](https://github.com/juspay/hyperswitch/commit/5eb67114646674fe227f073e417f26beb97e9a43))

### Refactors

- Pass customer object to `make_pm_data` ([#3246](https://github.com/juspay/hyperswitch/pull/3246)) ([`36c32c3`](https://github.com/juspay/hyperswitch/commit/36c32c377ae788c96b578303eae5d029e3044b7c))

### Miscellaneous Tasks

- **postman:** Update Postman collection files ([`8fc68ad`](https://github.com/juspay/hyperswitch/commit/8fc68adc7fb6a23d4a2970a05f5739db6010a53d))

**Full Changelog:** [`2024.01.08.0...2024.01.10.0`](https://github.com/juspay/hyperswitch/compare/2024.01.08.0...2024.01.10.0)

- - -

## 2024.01.08.0

### Features

- **analytics:** Adding outgoing webhooks kafka event ([#3140](https://github.com/juspay/hyperswitch/pull/3140)) ([`1d26df2`](https://github.com/juspay/hyperswitch/commit/1d26df28bc5e1db359272b40adae70bfba9b7360))
- **connector:** Add Revoke mandate flow ([#3261](https://github.com/juspay/hyperswitch/pull/3261)) ([`90ac26a`](https://github.com/juspay/hyperswitch/commit/90ac26a92f837568be5181108fdb1272171bbf23))
- **merchant_account:** Add list multiple merchants in `MerchantAccountInterface` ([#3220](https://github.com/juspay/hyperswitch/pull/3220)) ([`c3172ef`](https://github.com/juspay/hyperswitch/commit/c3172ef60603325a1d9e5cab45e72d23a383e218))
- **payments:** Add payment id in all the payment logs ([#3142](https://github.com/juspay/hyperswitch/pull/3142)) ([`7766245`](https://github.com/juspay/hyperswitch/commit/7766245478f72b0bc942922b1138c87a239be153))
- **pm_list:** Add required fields for eps ([#3169](https://github.com/juspay/hyperswitch/pull/3169)) ([`bfd8a5a`](https://github.com/juspay/hyperswitch/commit/bfd8a5a31abb3c95cc9ca21689d5c30a6dc4ce8d))
- Add deep health check ([#3210](https://github.com/juspay/hyperswitch/pull/3210)) ([`f30ba89`](https://github.com/juspay/hyperswitch/commit/f30ba89884d3abf2356cf1870d833a97d2411f69))
- Include version number in response headers and on application startup ([#3045](https://github.com/juspay/hyperswitch/pull/3045)) ([`252443a`](https://github.com/juspay/hyperswitch/commit/252443a50dc48939eb08b3bcd67273bb71bbe349))

### Bug Fixes

- **analytics:**
  - Fixed response code to 501 ([#3119](https://github.com/juspay/hyperswitch/pull/3119)) ([`00008c1`](https://github.com/juspay/hyperswitch/commit/00008c16c1c20f1f34381d0fc7e55ef05183e776))
  - Added response to the connector outgoing event ([#3129](https://github.com/juspay/hyperswitch/pull/3129)) ([`d152c3a`](https://github.com/juspay/hyperswitch/commit/d152c3a1ca70c39f5c64edf63b5995f6cf02c88a))
- **connector:**
  - [NMI] Populating `ErrorResponse` with required fields and Mapping `connector_response_reference_id` ([#3214](https://github.com/juspay/hyperswitch/pull/3214)) ([`64babd3`](https://github.com/juspay/hyperswitch/commit/64babd34786ba8e6f63aa1dba1cbd1bc6264f2ac))
  - [Stripe] Deserialization Error while parsing Dispute Webhook Body ([#3256](https://github.com/juspay/hyperswitch/pull/3256)) ([`01b4ac3`](https://github.com/juspay/hyperswitch/commit/01b4ac30e40a55b05fe3585d0544b21125762bc7))
- **router:**
  - Multiple incremental_authorizations with kv enabled ([#3185](https://github.com/juspay/hyperswitch/pull/3185)) ([`f78d02d`](https://github.com/juspay/hyperswitch/commit/f78d02d981dd7b35f2150f204b327847b811badd))
  - Payment link api contract change ([#2975](https://github.com/juspay/hyperswitch/pull/2975)) ([`3cd7496`](https://github.com/juspay/hyperswitch/commit/3cd74966b279dc1c43935dc1bceb1c69b9eb0643))
- **user:** Add integration_completed enum in metadata type ([#3245](https://github.com/juspay/hyperswitch/pull/3245)) ([`3ab71fb`](https://github.com/juspay/hyperswitch/commit/3ab71fbd5ac86f12cf19d17561e428d33c51a4cf))
- **users:** Fix wrong redirection url in magic link ([#3217](https://github.com/juspay/hyperswitch/pull/3217)) ([`000e644`](https://github.com/juspay/hyperswitch/commit/000e64438838461ea930545405fb2ee0d3c4356c))
- Introduce net_amount field in payment response ([#3115](https://github.com/juspay/hyperswitch/pull/3115)) ([`23e0c63`](https://github.com/juspay/hyperswitch/commit/23e0c6354185d666771c07b8534e42380cc50812))

### Refactors

- **api_lock:** Allow api lock on psync only when force sync is true ([#3242](https://github.com/juspay/hyperswitch/pull/3242)) ([`ac5349c`](https://github.com/juspay/hyperswitch/commit/ac5349cd7160f67f7a56f48f54981cf3dc1e5b52))
- **drainer:** Change logic for trimming the stream and refactor for modularity ([#3128](https://github.com/juspay/hyperswitch/pull/3128)) ([`de7a607`](https://github.com/juspay/hyperswitch/commit/de7a607e66847ff4bbddcbbafa50d54a56f02f62))
- **euclid_wasm:** Update wasm config ([#3222](https://github.com/juspay/hyperswitch/pull/3222)) ([`7ea50c3`](https://github.com/juspay/hyperswitch/commit/7ea50c3a78bc1a091077c23999a69dda1cf0f463))
- Address panics due to indexing and slicing ([#3233](https://github.com/juspay/hyperswitch/pull/3233)) ([`34318bc`](https://github.com/juspay/hyperswitch/commit/34318bc1f12a1298e8993021a2d516cf86049980))

### Miscellaneous Tasks

- Address Rust 1.75 clippy lints ([#3231](https://github.com/juspay/hyperswitch/pull/3231)) ([`c8279b1`](https://github.com/juspay/hyperswitch/commit/c8279b110e6c55784f042aebb956931e1870b0ca))

**Full Changelog:** [`v1.106.1...2024.01.08.0`](https://github.com/juspay/hyperswitch/compare/v1.106.1...2024.01.08.0)

- - -

## 1.106.1 (2024-01-05)

### Bug Fixes

- **connector:** [iatapay] change refund amount ([#3244](https://github.com/juspay/hyperswitch/pull/3244)) ([`e79604b`](https://github.com/juspay/hyperswitch/commit/e79604bd4681a69802f3c3169dd94424e3688e42))

**Full Changelog:** [`v1.106.0...v1.106.1`](https://github.com/juspay/hyperswitch/compare/v1.106.0...v1.106.1)

- - -


## 1.106.0 (2024-01-04)

### Features

- **connector:**
  - [BOA] Populate merchant_defined_information with metadata ([#3208](https://github.com/juspay/hyperswitch/pull/3208)) ([`18eca7e`](https://github.com/juspay/hyperswitch/commit/18eca7e9fbe6cdc101bd135c4618882b7a5455bf))
  - [CYBERSOURCE] Refactor cybersource ([#3215](https://github.com/juspay/hyperswitch/pull/3215)) ([`e06ba14`](https://github.com/juspay/hyperswitch/commit/e06ba148b666772fe79d7050d0c505dd2f04f87c))
- **customers:** Add JWT Authentication for `/customers` APIs ([#3179](https://github.com/juspay/hyperswitch/pull/3179)) ([`aefe618`](https://github.com/juspay/hyperswitch/commit/aefe6184ec3e3156877c72988ca0f92454a47e7d))

### Bug Fixes

- **connector:** [Volt] Error handling for auth response ([#3187](https://github.com/juspay/hyperswitch/pull/3187)) ([`a51c54d`](https://github.com/juspay/hyperswitch/commit/a51c54d39d3687c6a06176895435ac66fa194d7b))
- **core:** Fix recurring mandates flow for cyber source ([#3224](https://github.com/juspay/hyperswitch/pull/3224)) ([`6a1743e`](https://github.com/juspay/hyperswitch/commit/6a1743ebe993d5abb53f2ce1b8b383aa4a9553fb))
- **middleware:** Add support for logging request-id sent in request ([#3225](https://github.com/juspay/hyperswitch/pull/3225)) ([`0f72b55`](https://github.com/juspay/hyperswitch/commit/0f72b5527aab221b8e69e737e5d19abdd0696150))

### Refactors

- **connector:** [NMI] Include mandatory fields for card 3DS ([#3203](https://github.com/juspay/hyperswitch/pull/3203)) ([`a46b8a7`](https://github.com/juspay/hyperswitch/commit/a46b8a7b05367fbbdbf4fca89d8a6b29110a4e1c))

### Testing

- **postman:** Update postman collection files ([`0248d35`](https://github.com/juspay/hyperswitch/commit/0248d35dd49d2dc7e5e4da6b60a3ee3577c8eac9))

### Miscellaneous Tasks

- Fix channel handling for consumer workflow loop ([#3223](https://github.com/juspay/hyperswitch/pull/3223)) ([`51e1fac`](https://github.com/juspay/hyperswitch/commit/51e1fac556fdd8775e0bbc858b0b3cc50a7e88ec))

**Full Changelog:** [`v1.105.0...v1.106.0`](https://github.com/juspay/hyperswitch/compare/v1.105.0...v1.106.0)

- - -


## 1.105.0 (2023-12-23)

### Features

- **connector:** [BOA/CYBERSOURCE] Populate connector_transaction_id ([#3202](https://github.com/juspay/hyperswitch/pull/3202)) ([`110d3d2`](https://github.com/juspay/hyperswitch/commit/110d3d211be2edf47533cc5297ae159cad0e5034))

**Full Changelog:** [`v1.104.0...v1.105.0`](https://github.com/juspay/hyperswitch/compare/v1.104.0...v1.105.0)

- - -


## 1.104.0 (2023-12-22)

### Features

- **connector:** [BOA] Implement apple pay manual flow ([#3191](https://github.com/juspay/hyperswitch/pull/3191)) ([`25fd3d5`](https://github.com/juspay/hyperswitch/commit/25fd3d502e48f10dd3acbdc88caea4007310d4ee))
- **router:** Make the billing country for apple pay as optional field ([#3188](https://github.com/juspay/hyperswitch/pull/3188)) ([`15987cc`](https://github.com/juspay/hyperswitch/commit/15987cc81ecba3c1d0de4fa0a12424066a8842eb))

### Bug Fixes

- **connector:**
  - [Trustpay] Use `connector_request_reference_id` for merchant reference instead of `payment_id` ([#2885](https://github.com/juspay/hyperswitch/pull/2885)) ([`c51c761`](https://github.com/juspay/hyperswitch/commit/c51c761677e8c5ff80de40f8796f340cf1331f96))
  - [BOA/Cyb] Truncate state length to <20 ([#3198](https://github.com/juspay/hyperswitch/pull/3198)) ([`79a18e2`](https://github.com/juspay/hyperswitch/commit/79a18e2bf7bb1f338cf982fb1a152add2ed4e087))
  - [Iatapay] fix error response handling when payment is failed ([#3197](https://github.com/juspay/hyperswitch/pull/3197)) ([`716a74c`](https://github.com/juspay/hyperswitch/commit/716a74cf8449583541c426a5c427c9e32f5b2528))
  - [BOA] Display 2XX Failure Errors ([#3200](https://github.com/juspay/hyperswitch/pull/3200)) ([`07fd9be`](https://github.com/juspay/hyperswitch/commit/07fd9bedf02a1d70fc248fbbab480a5e24a7f077))
  - [CYBERSOURCE] Display 2XX Failure Errors ([#3201](https://github.com/juspay/hyperswitch/pull/3201)) ([`86c2622`](https://github.com/juspay/hyperswitch/commit/86c26221357e14b585f44c6ebe46962c085f6552))
- **users:** Wrong `user_role` insertion in `invite_user` for new users ([#3193](https://github.com/juspay/hyperswitch/pull/3193)) ([`b06a8d6`](https://github.com/juspay/hyperswitch/commit/b06a8d6e0d7fc4fb1bec30f702d64f0bd5e1068e))

**Full Changelog:** [`v1.103.1...v1.104.0`](https://github.com/juspay/hyperswitch/compare/v1.103.1...v1.104.0)

- - -


## 1.103.1 (2023-12-21)

### Bug Fixes

- **connector:**
  - Remove set_body method for connectors implementing default get_request_body ([#3182](https://github.com/juspay/hyperswitch/pull/3182)) ([`a5e141b`](https://github.com/juspay/hyperswitch/commit/a5e141b542622e7065f0e0070a3cddacde78fd8a))
  - [Paypal] remove shipping address as mandatory field for paypal wallet ([#3181](https://github.com/juspay/hyperswitch/pull/3181)) ([`680ed60`](https://github.com/juspay/hyperswitch/commit/680ed603c5113ec29fbd13c4c633e18ad4ad10ee))

**Full Changelog:** [`v1.103.0...v1.103.1`](https://github.com/juspay/hyperswitch/compare/v1.103.0...v1.103.1)

- - -


## 1.103.0 (2023-12-20)

### Features

- **connector:**
  - [NMI] Implement webhook for Payments and Refunds ([#3164](https://github.com/juspay/hyperswitch/pull/3164)) ([`30c1401`](https://github.com/juspay/hyperswitch/commit/30c14019d067ad5f105563f205eb1941010233e8))
  - [BOA] Handle BOA 5XX errors ([#3178](https://github.com/juspay/hyperswitch/pull/3178)) ([`1d80949`](https://github.com/juspay/hyperswitch/commit/1d80949bef1228bf432dc445eaba15afccb030bd))
- **connector-config:** Add wasm support for dashboard connector configuration ([#3138](https://github.com/juspay/hyperswitch/pull/3138)) ([`b0ffbe9`](https://github.com/juspay/hyperswitch/commit/b0ffbe9355b7e38226994c1ccbbe80cdbc77adde))
- **db:** Implement `AuthorizationInterface` for `MockDb` ([#3151](https://github.com/juspay/hyperswitch/pull/3151)) ([`396a64f`](https://github.com/juspay/hyperswitch/commit/396a64f3bbad6e75d4b263286a7ef6a2f09b180e))
- **postman:** [Prophetpay] Add test cases ([#2946](https://github.com/juspay/hyperswitch/pull/2946)) ([`583d7b8`](https://github.com/juspay/hyperswitch/commit/583d7b87a711102e4e62417f3191ac837886eca9))

### Bug Fixes

- **connector:**
  - [NMI] Fix response deserialization for vault id creation ([#3166](https://github.com/juspay/hyperswitch/pull/3166)) ([`d44daaf`](https://github.com/juspay/hyperswitch/commit/d44daaf539021a9cbc33c9391172c38825d74dcd))
  - Connector wise validation for zero auth flow ([#3159](https://github.com/juspay/hyperswitch/pull/3159)) ([`45ba128`](https://github.com/juspay/hyperswitch/commit/45ba128b6ab39f513dd114567d9915acf0eaea20))
- **events:** Add logger for incoming webhook payload ([#3171](https://github.com/juspay/hyperswitch/pull/3171)) ([`cf47a65`](https://github.com/juspay/hyperswitch/commit/cf47a65916fd4fb5c996946ffd579fd6755d02f7))
- **users:** Send correct `user_role` values in `switch_merchant` response ([#3167](https://github.com/juspay/hyperswitch/pull/3167)) ([`dc589d5`](https://github.com/juspay/hyperswitch/commit/dc589d580f1382874bc755d3719bd3244fdedc67))

### Refactors

- **core:** Fix payment status for 4xx ([#3177](https://github.com/juspay/hyperswitch/pull/3177)) ([`e7949c2`](https://github.com/juspay/hyperswitch/commit/e7949c23b9be56a4cd763d4990c1a95c0fefae95))
- **payment_methods:** Make the card_holder_name as an empty string if not sent ([#3173](https://github.com/juspay/hyperswitch/pull/3173)) ([`b98e53d`](https://github.com/juspay/hyperswitch/commit/b98e53d5cba5a5af04ada9bd83fa7bd2e27462d9))

### Testing

- **postman:** Update postman collection files ([`6890e90`](https://github.com/juspay/hyperswitch/commit/6890e9029d90bfd518ba23979a0bd507853dc983))

### Documentation

- **connector:** Update connector integration documentation  ([#3041](https://github.com/juspay/hyperswitch/pull/3041)) ([`ce5514e`](https://github.com/juspay/hyperswitch/commit/ce5514eadfce240bc4cefb472405f37432a8507b))

**Full Changelog:** [`v1.102.1...v1.103.0`](https://github.com/juspay/hyperswitch/compare/v1.102.1...v1.103.0)

- - -


## 1.102.1 (2023-12-18)

### Bug Fixes

- **connector:** [BOA/CYBERSOURCE] Update error handling ([#3156](https://github.com/juspay/hyperswitch/pull/3156)) ([`8e484dd`](https://github.com/juspay/hyperswitch/commit/8e484ddab8d3f4463299c7f7e8ce75b8dd628599))
- **euclid_wasm:** Add function to retrieve keys for 3ds and surcharge decision manager ([#3160](https://github.com/juspay/hyperswitch/pull/3160)) ([`30fe9d1`](https://github.com/juspay/hyperswitch/commit/30fe9d19e4955035a370f8f9ce37963cdb76c68a))
- **payment_link:** Added amount conversion to base unit based on currency ([#3162](https://github.com/juspay/hyperswitch/pull/3162)) ([`0fa61a9`](https://github.com/juspay/hyperswitch/commit/0fa61a9dd194c5b3688f8f68b056c263d92327d0))
- Change prodintent name in dashboard metadata ([#3161](https://github.com/juspay/hyperswitch/pull/3161)) ([`8db3361`](https://github.com/juspay/hyperswitch/commit/8db3361d80f674a28a3916830a4b0c1c2b89776a))

### Refactors

- **connector:**
  - [Helcim] change error message from not supported to not implemented ([#2850](https://github.com/juspay/hyperswitch/pull/2850)) ([`41b5a82`](https://github.com/juspay/hyperswitch/commit/41b5a82bafa9b0392bb43ed268fefc5187b48636))
  - [Forte] change error message from not supported to not implemented ([#2847](https://github.com/juspay/hyperswitch/pull/2847)) ([`3fc0e2d`](https://github.com/juspay/hyperswitch/commit/3fc0e2d8195948d50f735df5192ae0f8431b432b))
  - [Cryptopay] change error message from not supported to not implemented ([#2846](https://github.com/juspay/hyperswitch/pull/2846)) ([`2d895be`](https://github.com/juspay/hyperswitch/commit/2d895be9856d17cd923665568aa9b6e54fc1a305))
- **router:** [ACI] change payment error message from not supported to not implemented error ([#2837](https://github.com/juspay/hyperswitch/pull/2837)) ([`cc12e8a`](https://github.com/juspay/hyperswitch/commit/cc12e8a2435e5e47eeec77c620c747b156a3e16b))
- **users:** Rename `user_roles` and `dashboard_metadata` columns ([#3135](https://github.com/juspay/hyperswitch/pull/3135)) ([`e3589e6`](https://github.com/juspay/hyperswitch/commit/e3589e641c8a0b3b690b82f09a61d512db2d9932))

**Full Changelog:** [`v1.102.0+hotfix.1...v1.102.1`](https://github.com/juspay/hyperswitch/compare/v1.102.0+hotfix.1...v1.102.1)

- - -


## 1.102.0 (2023-12-17)

### Features

- **connector:**
  - [CYBERSOURCE] Implement Google Pay ([#3139](https://github.com/juspay/hyperswitch/pull/3139)) ([`4ae6af4`](https://github.com/juspay/hyperswitch/commit/4ae6af4632bbef5d21c3cb28538dcc4a94a10789))
  - [PlaceToPay] Implement Cards for PlaceToPay ([#3117](https://github.com/juspay/hyperswitch/pull/3117)) ([`107c66f`](https://github.com/juspay/hyperswitch/commit/107c66fec331376aa8c9f1e710e1503793fde119))
  - [CYBERSOURCE] Implement Apple Pay ([#3149](https://github.com/juspay/hyperswitch/pull/3149)) ([`5f53d84`](https://github.com/juspay/hyperswitch/commit/5f53d84a8b92f8aab67d09666b45362b287809ff))
  - [NMI] Implement 3DS for Cards ([#3143](https://github.com/juspay/hyperswitch/pull/3143)) ([`7df4523`](https://github.com/juspay/hyperswitch/commit/7df45235b1b55c3e4f1205169fb512d2aadc98ac))

### Bug Fixes

- **connector:**
  - [Checkout] Fix status mapping for checkout ([#3073](https://github.com/juspay/hyperswitch/pull/3073)) ([`5b2c329`](https://github.com/juspay/hyperswitch/commit/5b2c3291d4fbe3c4154c187b4e915dc3365e761a))
  - [Cybersource] signature authentication in incremental_authorization flow ([#3141](https://github.com/juspay/hyperswitch/pull/3141)) ([`d47a7cc`](https://github.com/juspay/hyperswitch/commit/d47a7cc418b0f4bb609d99f4a463a14c39df46e4))
- [CYBERSOURCE] Fix Status Mapping ([#3144](https://github.com/juspay/hyperswitch/pull/3144)) ([`62c0c47`](https://github.com/juspay/hyperswitch/commit/62c0c47e99f154399687a32caf9999b365da60ae))

### Testing

- **postman:** Update postman collection files ([`d40de4c`](https://github.com/juspay/hyperswitch/commit/d40de4c8b51010a9e6a3164196702a20c2ab3563))

### Miscellaneous Tasks

- **deps:** Bump zerocopy from 0.7.26 to 0.7.31 ([#3136](https://github.com/juspay/hyperswitch/pull/3136)) ([`d8de3c2`](https://github.com/juspay/hyperswitch/commit/d8de3c285c90103da93f0f3fd0241924dabd256f))
- **events:** Remove duplicate logs ([#3148](https://github.com/juspay/hyperswitch/pull/3148)) ([`a78fed7`](https://github.com/juspay/hyperswitch/commit/a78fed73babace05b4f668ef219909277045ba85))

**Full Changelog:** [`v1.101.0...v1.102.0`](https://github.com/juspay/hyperswitch/compare/v1.101.0...v1.102.0)

- - -


## 1.101.0 (2023-12-14)

### Features

- **payments:** Add outgoing payments webhooks ([#3133](https://github.com/juspay/hyperswitch/pull/3133)) ([`f457846`](https://github.com/juspay/hyperswitch/commit/f4578463d5e1a0f442aacebdfa7af0460489ba8c))

### Bug Fixes

- **connector:** [CashToCode]Fix cashtocode redirection for evoucher pm type ([#3131](https://github.com/juspay/hyperswitch/pull/3131)) ([`71a86a8`](https://github.com/juspay/hyperswitch/commit/71a86a804e15e4d053f92cfddb36a15cf7b77f7a))
- **locker:** Fix double serialization for json request ([#3134](https://github.com/juspay/hyperswitch/pull/3134)) ([`70b86b7`](https://github.com/juspay/hyperswitch/commit/70b86b71e4809d2a47c6bc1214f72c37d3325c37))
- **router:** Add routing cache invalidation on payment connector update ([#3132](https://github.com/juspay/hyperswitch/pull/3132)) ([`1f84865`](https://github.com/juspay/hyperswitch/commit/1f848659f135542fdfa967b3b48ad6cdf69fda2c))

**Full Changelog:** [`v1.100.0...v1.101.0`](https://github.com/juspay/hyperswitch/compare/v1.100.0...v1.101.0)

- - -


## 1.100.0 (2023-12-14)

### Features

- **connector:**
  - [RISKIFIED] Add support for riskified frm connector ([#2533](https://github.com/juspay/hyperswitch/pull/2533)) ([`151a30f`](https://github.com/juspay/hyperswitch/commit/151a30f4eed10924cd93bf7f4f66976af0ab8314))
  - [HELCIM] Add connector_request_reference_id in invoice_number  ([#3087](https://github.com/juspay/hyperswitch/pull/3087)) ([`3cc9642`](https://github.com/juspay/hyperswitch/commit/3cc9642f3ac4c07fb675e9ff4032832819d877a1))
- **core:** Enable surcharge support for all connectors ([#3109](https://github.com/juspay/hyperswitch/pull/3109)) ([`57e1ae9`](https://github.com/juspay/hyperswitch/commit/57e1ae9dea6ff70fb1bca47c479c35026c167bad))
- **events:** Add type info to outgoing requests & maintain structural & PII type info ([#2956](https://github.com/juspay/hyperswitch/pull/2956)) ([`6e82b0b`](https://github.com/juspay/hyperswitch/commit/6e82b0bd746b405281f79b86a3cd92b550a33f68))
- **external_services:** Adds encrypt function for KMS ([#3111](https://github.com/juspay/hyperswitch/pull/3111)) ([`bca7cdb`](https://github.com/juspay/hyperswitch/commit/bca7cdb4c14b5fbb40d8cbf59fd1756ad27ac674))

### Bug Fixes

- **api_locking:** Fix the unit interpretation for `LockSettings` expiry ([#3121](https://github.com/juspay/hyperswitch/pull/3121)) ([`3f4167d`](https://github.com/juspay/hyperswitch/commit/3f4167dbd477c793e1a4cc572da0c12d66f2b649))
- **connector:** [trustpay] make paymentId optional field ([#3101](https://github.com/juspay/hyperswitch/pull/3101)) ([`62a7c30`](https://github.com/juspay/hyperswitch/commit/62a7c3053c5e276091f5bd54a5679caef58a4ace))
- **docker-compose:** Remove label list from docker compose yml ([#3118](https://github.com/juspay/hyperswitch/pull/3118)) ([`e1e23fd`](https://github.com/juspay/hyperswitch/commit/e1e23fd987cae96e56311d1cfdcb225d9327860c))
- Validate refund amount with amount_captured instead of amount ([#3120](https://github.com/juspay/hyperswitch/pull/3120)) ([`be13d15`](https://github.com/juspay/hyperswitch/commit/be13d15d3c0214c863e131cf1dbe184d5baec5d7))

### Refactors

- **connector:** [Wise] Error Message For Connector Implementation  ([#2952](https://github.com/juspay/hyperswitch/pull/2952)) ([`1add2c0`](https://github.com/juspay/hyperswitch/commit/1add2c059f4fb5653f33e2f3ce454793caf2d595))
- **payments:** Add support for receiving card_holder_name field as an empty string ([#3127](https://github.com/juspay/hyperswitch/pull/3127)) ([`4d19d8b`](https://github.com/juspay/hyperswitch/commit/4d19d8b1d18f49f02e951c5025d35cf5d62cec1b))

### Testing

- **postman:** Update postman collection files ([`a5618cd`](https://github.com/juspay/hyperswitch/commit/a5618cd5d6eb5b007f7927f05e777e875195a678))

**Full Changelog:** [`v1.99.0...v1.100.0`](https://github.com/juspay/hyperswitch/compare/v1.99.0...v1.100.0)

- - -


## 1.99.0 (2023-12-12)

### Features

- **connector:** [Placetopay] Add Connector Template Code  ([#3084](https://github.com/juspay/hyperswitch/pull/3084)) ([`a7b688a`](https://github.com/juspay/hyperswitch/commit/a7b688aac72e15f782046b9d108aca12f43a9994))
- Add utility to convert TOML configuration file to list of environment variables ([#3096](https://github.com/juspay/hyperswitch/pull/3096)) ([`2c4599a`](https://github.com/juspay/hyperswitch/commit/2c4599a1cd7e244b6fb11948c88c55c5b8faad76))

### Bug Fixes

- **router:** Make `request_incremental_authorization` optional in payment_intent ([#3086](https://github.com/juspay/hyperswitch/pull/3086)) ([`f7da59d`](https://github.com/juspay/hyperswitch/commit/f7da59d06af11707e210b58a875c013d31c3ee17))

### Refactors

- **email:** Create client every time of sending email ([#3105](https://github.com/juspay/hyperswitch/pull/3105)) ([`fc2f163`](https://github.com/juspay/hyperswitch/commit/fc2f16392148cd66b3c3e67e3e0c782910e37e1f))

### Testing

- **postman:** Update postman collection files ([`aa97821`](https://github.com/juspay/hyperswitch/commit/aa9782164fb7846fe533c5057a17756dc82ede54))

### Miscellaneous Tasks

- **deps:** Update fred and moka ([#3088](https://github.com/juspay/hyperswitch/pull/3088)) ([`129b1e5`](https://github.com/juspay/hyperswitch/commit/129b1e55bd1cbad0243030fd25379f1400eb170c))

**Full Changelog:** [`v1.98.0...v1.99.0`](https://github.com/juspay/hyperswitch/compare/v1.98.0...v1.99.0)

- - -


## 1.98.0 (2023-12-11)

### Features

- **connector:** Accept connector_transaction_id in error_response of connector flows for Trustpay ([#3060](https://github.com/juspay/hyperswitch/pull/3060)) ([`f53b090`](https://github.com/juspay/hyperswitch/commit/f53b090db87e094f9694481f13af62240c4c422a))
- **pm_auth:** Pm_auth service migration ([#3047](https://github.com/juspay/hyperswitch/pull/3047)) ([`9c1c44a`](https://github.com/juspay/hyperswitch/commit/9c1c44a706750b14857e9180f5161b61ed89a2ad))
- **user:** Add `verify_email` API ([#3076](https://github.com/juspay/hyperswitch/pull/3076)) ([`585e009`](https://github.com/juspay/hyperswitch/commit/585e00980c43797f326efb809df9ffd497d1dd26))
- **users:** Add resend verification email API ([#3093](https://github.com/juspay/hyperswitch/pull/3093)) ([`6d5c25e`](https://github.com/juspay/hyperswitch/commit/6d5c25e3369117acaf5865965769649d524226af))

### Bug Fixes

- **analytics:** Adding api_path to api logs event and to auditlogs api response ([#3079](https://github.com/juspay/hyperswitch/pull/3079)) ([`bf67438`](https://github.com/juspay/hyperswitch/commit/bf674380d5c7e856d0bae75554326aa9017c0201))
- **config:** Add missing config fields in `docker_compose.toml` ([#3080](https://github.com/juspay/hyperswitch/pull/3080)) ([`1f8116d`](https://github.com/juspay/hyperswitch/commit/1f8116db368aec344d08603045c4cb46c2c25b41))
- **connector:** [CYBERSOURCE] Remove Phone Number Field From Address ([#3095](https://github.com/juspay/hyperswitch/pull/3095)) ([`72955ec`](https://github.com/juspay/hyperswitch/commit/72955ecc68280773b9c77b4db3d46de95a62f9ed))
- **drainer:** Properly log deserialization errors ([#3075](https://github.com/juspay/hyperswitch/pull/3075)) ([`42b5bd4`](https://github.com/juspay/hyperswitch/commit/42b5bd4f3d142c9fa12475f36a8b144753ac06e2))
- **router:** Allow zero amount for payment intent in list payment methods ([#3090](https://github.com/juspay/hyperswitch/pull/3090)) ([`b283b6b`](https://github.com/juspay/hyperswitch/commit/b283b6b662c9f2eabe90473434369d8f7c2369a6))
- **user:** Add checks for change password ([#3078](https://github.com/juspay/hyperswitch/pull/3078)) ([`26a2611`](https://github.com/juspay/hyperswitch/commit/26a261131b4dbb8570e139127a2c0d356e2820be))

### Refactors

- **payment_methods:** Make the card_holder_name optional for card details in the payment APIs ([#3074](https://github.com/juspay/hyperswitch/pull/3074)) ([`b279591`](https://github.com/juspay/hyperswitch/commit/b279591057cdba6004c99efc82bb856f0bacd1e0))
- **user:** Add account verification check in signin ([#3082](https://github.com/juspay/hyperswitch/pull/3082)) ([`f7d6e3c`](https://github.com/juspay/hyperswitch/commit/f7d6e3c0149869175a59996e67d3e2d3b6f3b8c2))

### Documentation

- **openapi:** Fix `payment_methods_enabled` OpenAPI spec in merchant connector account APIs ([#3068](https://github.com/juspay/hyperswitch/pull/3068)) ([`b6838c4`](https://github.com/juspay/hyperswitch/commit/b6838c4d1a3a456e28a5f438fcd74a60bedb2539))

### Miscellaneous Tasks

- **configs:** [CYBERSOURCE] Add mandate configs ([#3085](https://github.com/juspay/hyperswitch/pull/3085)) ([`777cd5c`](https://github.com/juspay/hyperswitch/commit/777cd5cdc2342fb7195a06505647fa331725e1dd))

**Full Changelog:** [`v1.97.0...v1.98.0`](https://github.com/juspay/hyperswitch/compare/v1.97.0...v1.98.0)

- - -


## 1.97.0 (2023-12-06)

### Features

- **Braintree:** Sync with Hyperswitch Reference ([#3037](https://github.com/juspay/hyperswitch/pull/3037)) ([`8a995ce`](https://github.com/juspay/hyperswitch/commit/8a995cefdf6806645383710c6f39d963da232e94))
- **connector:** [BANKOFAMERICA] Implement Apple Pay ([#3061](https://github.com/juspay/hyperswitch/pull/3061)) ([`47c0383`](https://github.com/juspay/hyperswitch/commit/47c038300adad1c02e4c77d529c7cc2457cf3b91))
- **metrics:** Add drainer delay metric ([#3034](https://github.com/juspay/hyperswitch/pull/3034)) ([`c6e2ee2`](https://github.com/juspay/hyperswitch/commit/c6e2ee29d9ee4fe54e6fa6f87c2fa065a290d258))

### Bug Fixes

- **config:** Parse kafka brokers from env variable as sequence ([#3066](https://github.com/juspay/hyperswitch/pull/3066)) ([`84decd8`](https://github.com/juspay/hyperswitch/commit/84decd8126d306a5e1cf22b36e1378a73dc963f5))
- Throw bad request while pushing duplicate data to redis ([#3016](https://github.com/juspay/hyperswitch/pull/3016)) ([`a2405e5`](https://github.com/juspay/hyperswitch/commit/a2405e56fbd84936a1afa6aa9f8f7e815267fbec))
- Return url none on complete authorize ([#3067](https://github.com/juspay/hyperswitch/pull/3067)) ([`6eec06b`](https://github.com/juspay/hyperswitch/commit/6eec06b1d6ee9a00b374905e0ab9e425d0e41095))

### Miscellaneous Tasks

- **codeowners:** Add codeowners for hyperswitch dashboard ([#3057](https://github.com/juspay/hyperswitch/pull/3057)) ([`cfafd5c`](https://github.com/juspay/hyperswitch/commit/cfafd5cd29857283d57731dda7c5a332a493f531))

**Full Changelog:** [`v1.96.0...v1.97.0`](https://github.com/juspay/hyperswitch/compare/v1.96.0...v1.97.0)

- - -


## 1.96.0 (2023-12-05)

### Features

- **connector_onboarding:** Add Connector onboarding APIs ([#3050](https://github.com/juspay/hyperswitch/pull/3050)) ([`7bd6e05`](https://github.com/juspay/hyperswitch/commit/7bd6e05c0c05ebae9b82a6f410e61ca4409d088b))
- **pm_list:** Add required fields for bancontact_card for Mollie, Adyen and Stripe ([#3035](https://github.com/juspay/hyperswitch/pull/3035)) ([`792e642`](https://github.com/juspay/hyperswitch/commit/792e642ad58f90bae3ddcea5e6cbc70e948d8e28))
- **user:** Add email apis and new enums for metadata ([#3053](https://github.com/juspay/hyperswitch/pull/3053)) ([`1c3d260`](https://github.com/juspay/hyperswitch/commit/1c3d260dc3e18fbf6cbd5122122a6c73dceb39a3))
- Implement FRM flows ([#2968](https://github.com/juspay/hyperswitch/pull/2968)) ([`055d838`](https://github.com/juspay/hyperswitch/commit/055d8383671f6b466297c177bcc770618c7da96a))

### Bug Fixes

- Remove redundant call to populate_payment_data function ([#3054](https://github.com/juspay/hyperswitch/pull/3054)) ([`53df543`](https://github.com/juspay/hyperswitch/commit/53df543b7f1407a758232025b7de0fb527be8e86))

### Documentation

- **test_utils:** Update postman docs ([#3055](https://github.com/juspay/hyperswitch/pull/3055)) ([`8b7a7aa`](https://github.com/juspay/hyperswitch/commit/8b7a7aa6494ff669e1f8bcc92a5160e422d6b26e))

**Full Changelog:** [`v1.95.0...v1.96.0`](https://github.com/juspay/hyperswitch/compare/v1.95.0...v1.96.0)

- - -


## 1.95.0 (2023-12-05)

### Features

- **connector:** [BOA/CYBERSOURCE] Fix Status Mapping for Terminal St… ([#3031](https://github.com/juspay/hyperswitch/pull/3031)) ([`95876b0`](https://github.com/juspay/hyperswitch/commit/95876b0ce03e024edf77909502c53eb4e63a9855))
- **pm_list:** Add required field for open_banking_uk for Adyen and Volt Connector  ([#3032](https://github.com/juspay/hyperswitch/pull/3032)) ([`9d93533`](https://github.com/juspay/hyperswitch/commit/9d935332193dcc9f191a0a5a9e7405316794a418))
- **router:**
  - Add key_value to locker metrics ([#2995](https://github.com/juspay/hyperswitch/pull/2995)) ([`83fcd1a`](https://github.com/juspay/hyperswitch/commit/83fcd1a9deb106a44c8262923c7f1660b0c46bf2))
  - Add payments incremental authorization api ([#3038](https://github.com/juspay/hyperswitch/pull/3038)) ([`a0cfdd3`](https://github.com/juspay/hyperswitch/commit/a0cfdd3fb12f04b603f65551eac985c31e08da85))
- **types:** Add email types for sending emails ([#3020](https://github.com/juspay/hyperswitch/pull/3020)) ([`c4bd47e`](https://github.com/juspay/hyperswitch/commit/c4bd47eca93a158c9daeeeb18afb1e735eea8c94))
- **user:**
  - Generate and delete sample data ([#2987](https://github.com/juspay/hyperswitch/pull/2987)) ([`092ec73`](https://github.com/juspay/hyperswitch/commit/092ec73b3c65ce6048d379383b078d643f0f35fc))
  - Add user_list and switch_list apis ([#3033](https://github.com/juspay/hyperswitch/pull/3033)) ([`ec15ddd`](https://github.com/juspay/hyperswitch/commit/ec15ddd0d0ed942fedec525406df3005d494b8d4))
- Calculate surcharge for customer saved card list ([#3039](https://github.com/juspay/hyperswitch/pull/3039)) ([`daf0f09`](https://github.com/juspay/hyperswitch/commit/daf0f09f8e3293ee6a3599a25362d9171fc5b2e7))

### Bug Fixes

- **connector:** [Paypal] Parse response for Cards with no 3DS check ([#3021](https://github.com/juspay/hyperswitch/pull/3021)) ([`d883cd1`](https://github.com/juspay/hyperswitch/commit/d883cd18972c5f9e8350e9a3f4e5cd56ec2c0787))
- **pm_list:** [Trustpay]Update dynamic fields for trustpay blik ([#3042](https://github.com/juspay/hyperswitch/pull/3042)) ([`9274cef`](https://github.com/juspay/hyperswitch/commit/9274cefbdd29d2ac64baeea2fe504dff2472cb47))
- **wasm:** Fix wasm function to return the categories for keys with their description respectively ([#3023](https://github.com/juspay/hyperswitch/pull/3023)) ([`2ac5b2c`](https://github.com/juspay/hyperswitch/commit/2ac5b2cd764c0aad53ac7c672dfcc9132fa5668f))
- Use card bin to get additional card details ([#3036](https://github.com/juspay/hyperswitch/pull/3036)) ([`6c7d3a2`](https://github.com/juspay/hyperswitch/commit/6c7d3a2e8a047ff23b52b76792fe8f28d3b952a4))
- Transform connector name to lowercase in connector integration script ([#3048](https://github.com/juspay/hyperswitch/pull/3048)) ([`298e362`](https://github.com/juspay/hyperswitch/commit/298e3627c379de5acfcafb074036754661801f1e))
- Add fallback to reverselookup error ([#3025](https://github.com/juspay/hyperswitch/pull/3025)) ([`ba392f5`](https://github.com/juspay/hyperswitch/commit/ba392f58b2956d67e93a08853bcf2270a869be27))

### Refactors

- **payment_methods:** Add support for passing card_cvc in payment_method_data object along with token ([#3024](https://github.com/juspay/hyperswitch/pull/3024)) ([`3ce04ab`](https://github.com/juspay/hyperswitch/commit/3ce04abae4eddfa27025368f5ef28987cccea43d))
- **users:** Separate signup and signin ([#2921](https://github.com/juspay/hyperswitch/pull/2921)) ([`80efeb7`](https://github.com/juspay/hyperswitch/commit/80efeb76b1801529766978af1c06e2d2c7de66c0))
- Create separate struct for surcharge details response ([#3027](https://github.com/juspay/hyperswitch/pull/3027)) ([`57591f8`](https://github.com/juspay/hyperswitch/commit/57591f819c7994099e76cff1affc7bcf3e45a031))

### Testing

- **postman:** Update postman collection files ([`6e09bc9`](https://github.com/juspay/hyperswitch/commit/6e09bc9e2c4bbe14dcb70da4a438850b03b3254c))

**Full Changelog:** [`v1.94.0...v1.95.0`](https://github.com/juspay/hyperswitch/compare/v1.94.0...v1.95.0)

- - -


## 1.94.0 (2023-12-01)

### Features

- **user_role:** Add APIs for user roles ([#3013](https://github.com/juspay/hyperswitch/pull/3013)) ([`3fa0bdf`](https://github.com/juspay/hyperswitch/commit/3fa0bdf76558ec91df8d3beef3c36658cd138b37))

### Bug Fixes

- **config:** Add kms decryption support for sqlx password ([#3029](https://github.com/juspay/hyperswitch/pull/3029)) ([`b593467`](https://github.com/juspay/hyperswitch/commit/b5934674e518f991a8a575ad01b971dd086eeb40))

### Refactors

- **connector:**
  - [Multisafe Pay] change error message from not supported to not implemented ([#2851](https://github.com/juspay/hyperswitch/pull/2851)) ([`668b943`](https://github.com/juspay/hyperswitch/commit/668b943403df2b3bb354dd093b8ec073a2618bda))
  - [Shift4] change error message from NotSupported to NotImplemented ([#2880](https://github.com/juspay/hyperswitch/pull/2880)) ([`bc79d52`](https://github.com/juspay/hyperswitch/commit/bc79d522c30aa036378cf1e01354c422585cc226))

**Full Changelog:** [`v1.93.0...v1.94.0`](https://github.com/juspay/hyperswitch/compare/v1.93.0...v1.94.0)

- - -


## 1.93.0 (2023-11-30)

### Features

- **connector:** [BANKOFAMERICA] Add Required Fields for GPAY ([#3014](https://github.com/juspay/hyperswitch/pull/3014)) ([`d30b58a`](https://github.com/juspay/hyperswitch/commit/d30b58abb5e716b70c2dadec9e6f13c9e3403b6f))
- **core:** Add ability to verify connector credentials before integrating the connector ([#2986](https://github.com/juspay/hyperswitch/pull/2986)) ([`39f255b`](https://github.com/juspay/hyperswitch/commit/39f255b4b209588dec35d780078c2ab7ceb37b10))
- **router:** Make core changes in payments flow to support incremental authorization ([#3009](https://github.com/juspay/hyperswitch/pull/3009)) ([`1ca2ba4`](https://github.com/juspay/hyperswitch/commit/1ca2ba459495ff9340954c87a6ae3e4dce0e7b71))
- **user:** Add support for dashboard metadata ([#3000](https://github.com/juspay/hyperswitch/pull/3000)) ([`6a2e4ab`](https://github.com/juspay/hyperswitch/commit/6a2e4ab4169820f35e953a949bd2e82e7f098ed2))

### Bug Fixes

- **connector:**
  - Move authorised status to charged in setup mandate ([#3017](https://github.com/juspay/hyperswitch/pull/3017)) ([`663754d`](https://github.com/juspay/hyperswitch/commit/663754d629d59a17ba9d4985fe04f9404ceb16b7))
  - [Trustpay] Add mapping to error code `800.100.165` and `900.100.100` ([#2925](https://github.com/juspay/hyperswitch/pull/2925)) ([`8c37a8d`](https://github.com/juspay/hyperswitch/commit/8c37a8d857c5a58872fa2b2e194b85e755129677))
- **core:** Error message on Refund update for `Not Implemented` Case ([#3011](https://github.com/juspay/hyperswitch/pull/3011)) ([`6b7ada1`](https://github.com/juspay/hyperswitch/commit/6b7ada1a34450ea3a7fc019375ba462a14ddd6ab))
- **pm_list:** [Trustpay] Update Cards, Bank_redirect - blik pm type required field info for Trustpay ([#2999](https://github.com/juspay/hyperswitch/pull/2999)) ([`c05432c`](https://github.com/juspay/hyperswitch/commit/c05432c0bd70f222c2f898ce2cbb47a46364a490))
- **router:**
  - [Dlocal] connector transaction id fix ([#2872](https://github.com/juspay/hyperswitch/pull/2872)) ([`44b1f49`](https://github.com/juspay/hyperswitch/commit/44b1f4949ea06d59480670ccfa02446fa7713d13))
  - Use default value for the routing algorithm column during business profile creation ([#2791](https://github.com/juspay/hyperswitch/pull/2791)) ([`b1fe76a`](https://github.com/juspay/hyperswitch/commit/b1fe76a82b4026d6eaa3baf4356378040880a458))
- **routing:** Fix kgraph to exclude PM auth during construction ([#3019](https://github.com/juspay/hyperswitch/pull/3019)) ([`c6cb527`](https://github.com/juspay/hyperswitch/commit/c6cb527f07e23796c342f3562fbf3b61f1ef6801))

### Refactors

- **connector:**
  - [Stax] change error message from NotSupported to NotImplemented ([#2879](https://github.com/juspay/hyperswitch/pull/2879)) ([`8a4dabc`](https://github.com/juspay/hyperswitch/commit/8a4dabc61df3e6012e50f785d93808ca3349be65))
  - [Volt] change error message from NotSupported to NotImplemented ([#2878](https://github.com/juspay/hyperswitch/pull/2878)) ([`de8e31b`](https://github.com/juspay/hyperswitch/commit/de8e31b70d9b3c11e268cd1deffa71918dc4270d))
  - [Adyen] Change country and issuer type to Optional for OpenBankingUk ([#2993](https://github.com/juspay/hyperswitch/pull/2993)) ([`ab3dac7`](https://github.com/juspay/hyperswitch/commit/ab3dac79b4f138cd1f60a9afc0635dcc137a4a05))
- **postman:** Fix payme postman collection for handling `order_details` ([#2996](https://github.com/juspay/hyperswitch/pull/2996)) ([`1e60c71`](https://github.com/juspay/hyperswitch/commit/1e60c710985b341a118bb32962bd74b406d78f69))

**Full Changelog:** [`v1.92.0...v1.93.0`](https://github.com/juspay/hyperswitch/compare/v1.92.0...v1.93.0)

- - -


## 1.92.0 (2023-11-29)

### Features

- **analytics:** Add Clickhouse based analytics ([#2988](https://github.com/juspay/hyperswitch/pull/2988)) ([`9df4e01`](https://github.com/juspay/hyperswitch/commit/9df4e0193ffeb6d1cc323bdebb7e2bdfb2a375e2))
- **ses_email:** Add email services to hyperswitch ([#2977](https://github.com/juspay/hyperswitch/pull/2977)) ([`5f5e895`](https://github.com/juspay/hyperswitch/commit/5f5e895f638701a0e6ab3deea9101ef39033dd16))

### Bug Fixes

- **router:** Make use of warning to log errors when apple pay metadata parsing fails ([#3010](https://github.com/juspay/hyperswitch/pull/3010)) ([`2e57745`](https://github.com/juspay/hyperswitch/commit/2e57745352c547323ac2df2554f6bc2dbd6da37f))

**Full Changelog:** [`v1.91.1...v1.92.0`](https://github.com/juspay/hyperswitch/compare/v1.91.1...v1.92.0)

- - -


## 1.91.1 (2023-11-29)

### Bug Fixes

- Remove `dummy_connector` from `default` features in `common_enums` ([#3005](https://github.com/juspay/hyperswitch/pull/3005)) ([`bb593ab`](https://github.com/juspay/hyperswitch/commit/bb593ab0cd1a30190b6c305f2432de83ac7fde93))
- Remove error propagation if card name not found in locker in case of temporary token ([#3006](https://github.com/juspay/hyperswitch/pull/3006)) ([`5c32b37`](https://github.com/juspay/hyperswitch/commit/5c32b3739e2c5895fe7f5cf8cc92f917c2639eac))
- Few fields were not getting updated in apply_changeset function ([#3002](https://github.com/juspay/hyperswitch/pull/3002)) ([`d289524`](https://github.com/juspay/hyperswitch/commit/d289524869f0c3835db9cf90d57ebedf560e0291))

### Miscellaneous Tasks

- **deps:** Bump openssl from 0.10.57 to 0.10.60 ([#3004](https://github.com/juspay/hyperswitch/pull/3004)) ([`1c2f35a`](https://github.com/juspay/hyperswitch/commit/1c2f35af92608fca5836448710eca9f9c23a776a))

**Full Changelog:** [`v1.91.0...v1.91.1`](https://github.com/juspay/hyperswitch/compare/v1.91.0...v1.91.1)

- - -


## 1.91.0 (2023-11-28)

### Features

- **core:**
  - [Paypal] Add Preprocessing flow to CompleteAuthorize for Card 3DS Auth Verification ([#2757](https://github.com/juspay/hyperswitch/pull/2757)) ([`77fc92c`](https://github.com/juspay/hyperswitch/commit/77fc92c99a99aaf76d270ba5b981928183a05768))
  - Enable payment refund when payment is partially captured ([#2991](https://github.com/juspay/hyperswitch/pull/2991)) ([`837480d`](https://github.com/juspay/hyperswitch/commit/837480d935cce8cc35f07c5ccb3560285909bc52))
- **currency_conversion:** Add currency conversion feature ([#2948](https://github.com/juspay/hyperswitch/pull/2948)) ([`c0116db`](https://github.com/juspay/hyperswitch/commit/c0116db271f6afc1b93c04705209bfc346228c68))
- **payment_methods:** Receive `card_holder_name` in confirm flow when using token for payment ([#2982](https://github.com/juspay/hyperswitch/pull/2982)) ([`e7ad3a4`](https://github.com/juspay/hyperswitch/commit/e7ad3a4db8823f3ae8d381771739670d8350e6da))

### Bug Fixes

- **connector:** [Adyen] `ErrorHandling` in case of Balance Check for Gift Cards ([#1976](https://github.com/juspay/hyperswitch/pull/1976)) ([`bd889c8`](https://github.com/juspay/hyperswitch/commit/bd889c834dd5e201b055233016f7226fa2187aea))
- **core:** Replace euclid enum with RoutableConnectors enum ([#2994](https://github.com/juspay/hyperswitch/pull/2994)) ([`ff6a0dd`](https://github.com/juspay/hyperswitch/commit/ff6a0dd0b515778b64a3e28ef905154eee85ec78))
- Remove error propagation if card name not found in locker ([#2998](https://github.com/juspay/hyperswitch/pull/2998)) ([`1c5a9b5`](https://github.com/juspay/hyperswitch/commit/1c5a9b5452afc33b18f45389bf3bdfd80820f476))

### Refactors

- **events:** Adding changes to type of API events to Kafka ([#2992](https://github.com/juspay/hyperswitch/pull/2992)) ([`d63f6f7`](https://github.com/juspay/hyperswitch/commit/d63f6f7224f35018e7c707353508bbacc2baed5c))
- **masking:** Use empty enums as masking:Strategy<T> types ([#2874](https://github.com/juspay/hyperswitch/pull/2874)) ([`0e66b1b`](https://github.com/juspay/hyperswitch/commit/0e66b1b5dcce6dd87c9d743c9eb73d0cd8e330b2))
- **router:** Add openapi spec support for merchant_connector apis ([#2997](https://github.com/juspay/hyperswitch/pull/2997)) ([`cdbb385`](https://github.com/juspay/hyperswitch/commit/cdbb3853cd44443f8487abc16a9ba5d99f22e475))
- Added min idle and max lifetime for database config ([#2900](https://github.com/juspay/hyperswitch/pull/2900)) ([`b3c51e6`](https://github.com/juspay/hyperswitch/commit/b3c51e6eb55c58adc024ee32b59c3910b2b72131))

### Testing

- **postman:** Update postman collection files ([`af6b05c`](https://github.com/juspay/hyperswitch/commit/af6b05c504b6fdbec7db77fa7f71535d7fea3e7a))

**Full Changelog:** [`v1.90.0...v1.91.0`](https://github.com/juspay/hyperswitch/compare/v1.90.0...v1.91.0)

- - -


## 1.90.0 (2023-11-27)

### Features

- **auth:** Add Authorization for JWT Authentication types ([#2973](https://github.com/juspay/hyperswitch/pull/2973)) ([`03c0a77`](https://github.com/juspay/hyperswitch/commit/03c0a772a99000acf4676db8ca2ce916036281d1))
- **user:** Implement change password for user ([#2959](https://github.com/juspay/hyperswitch/pull/2959)) ([`bfa1645`](https://github.com/juspay/hyperswitch/commit/bfa1645b847fb881eb2370d5dbfef6fd0b53725d))

### Bug Fixes

- **router:** Added validation to check total orderDetails amount equal to amount in request ([#2965](https://github.com/juspay/hyperswitch/pull/2965)) ([`37532d4`](https://github.com/juspay/hyperswitch/commit/37532d46f599a99e0e021b0455a6f02381005dd7))
- Add prefix to connector_transaction_id ([#2981](https://github.com/juspay/hyperswitch/pull/2981)) ([`107c3b9`](https://github.com/juspay/hyperswitch/commit/107c3b99417dd7bca7b62741ad601485700f37be))

### Refactors

- **connector:** [Nuvei] update error message ([#2867](https://github.com/juspay/hyperswitch/pull/2867)) ([`04b7c03`](https://github.com/juspay/hyperswitch/commit/04b7c0384dc9290bd60f49033fd35732527720f1))

### Testing

- **postman:** Update postman collection files ([`aee59e0`](https://github.com/juspay/hyperswitch/commit/aee59e088a8e7c1b81aca1015c90c7b4fd07511d))

### Documentation

- **try_local_system:** Add instructions to run using Docker Compose by pulling standalone images ([#2984](https://github.com/juspay/hyperswitch/pull/2984)) ([`0fa8ad1`](https://github.com/juspay/hyperswitch/commit/0fa8ad1b7c27010bf83e4035de9881d29e192e8a))

### Miscellaneous Tasks

- **connector:** Update connector addition script ([#2801](https://github.com/juspay/hyperswitch/pull/2801)) ([`34953a0`](https://github.com/juspay/hyperswitch/commit/34953a046429fe0341e8469bd9b036e176bda205))

**Full Changelog:** [`v1.89.0...v1.90.0`](https://github.com/juspay/hyperswitch/compare/v1.89.0...v1.90.0)

- - -


## 1.89.0 (2023-11-24)

### Features

- **router:** Add `connector_transaction_id` in error_response from connector flows ([#2972](https://github.com/juspay/hyperswitch/pull/2972)) ([`3322103`](https://github.com/juspay/hyperswitch/commit/3322103f5c9b7c2a5b663980246c6ca36b8dc63e))

### Bug Fixes

- **connector:** [BANKOFAMERICA] Add status VOIDED in enum Bankofameri… ([#2969](https://github.com/juspay/hyperswitch/pull/2969)) ([`203bbd7`](https://github.com/juspay/hyperswitch/commit/203bbd73751e1513206e81d7cf920ec263f83c58))
- **core:** Error propagation for not supporting partial refund ([#2976](https://github.com/juspay/hyperswitch/pull/2976)) ([`97a38a7`](https://github.com/juspay/hyperswitch/commit/97a38a78e514e4fa3b5db46b6de985be6312dcc3))
- **router:** Mark refund status as failure for not_implemented error from connector flows ([#2978](https://github.com/juspay/hyperswitch/pull/2978)) ([`d56d805`](https://github.com/juspay/hyperswitch/commit/d56d80557050336d5ed37282f1aa34b6c17389d1))
- Return none instead of err when payment method data is not found for bank debit during listing ([#2967](https://github.com/juspay/hyperswitch/pull/2967)) ([`5cc829a`](https://github.com/juspay/hyperswitch/commit/5cc829a11f515a413fe19f657a90aa05cebb99b5))
- Surcharge related status and rules fix ([#2974](https://github.com/juspay/hyperswitch/pull/2974)) ([`3db7213`](https://github.com/juspay/hyperswitch/commit/3db721388a7f0e291d7eb186661fc69a57068ea6))

### Documentation

- **README:** Updated Community Platform Mentions ([#2960](https://github.com/juspay/hyperswitch/pull/2960)) ([`e0bde43`](https://github.com/juspay/hyperswitch/commit/e0bde433282a34eb9eb28a2d9c43c2b17b5e65e5))
- Add Rust locker information in architecture doc ([#2964](https://github.com/juspay/hyperswitch/pull/2964)) ([`b2f7dd1`](https://github.com/juspay/hyperswitch/commit/b2f7dd13925a1429e316cd9eaf0e2d31d46b6d4a))

**Full Changelog:** [`v1.88.0...v1.89.0`](https://github.com/juspay/hyperswitch/compare/v1.88.0...v1.89.0)

- - -


## 1.88.0 (2023-11-23)

### Features

- **connector:** [BANKOFAMERICA] Implement Google Pay ([#2940](https://github.com/juspay/hyperswitch/pull/2940)) ([`f91d4ae`](https://github.com/juspay/hyperswitch/commit/f91d4ae11b02def92c1dde743a0c01b5aac5703f))
- **router:** Allow billing and shipping address update in payments confirm flow ([#2963](https://github.com/juspay/hyperswitch/pull/2963)) ([`59ef162`](https://github.com/juspay/hyperswitch/commit/59ef162219db3e4650dde65710850bc9f3280530))

### Bug Fixes

- **connector:** [Prophetpay] Use refund_id as reference_id for Refund ([#2966](https://github.com/juspay/hyperswitch/pull/2966)) ([`dd3e22a`](https://github.com/juspay/hyperswitch/commit/dd3e22a938714f373477e08d1d25e4b84ac796c6))
- **core:** Fix Default Values Enum FieldType ([#2934](https://github.com/juspay/hyperswitch/pull/2934)) ([`35a44ed`](https://github.com/juspay/hyperswitch/commit/35a44ed2533b748e3fabb8a2f8db4fa7e5d3cf7e))
- **drainer:** Increase jobs picked only when stream is not empty ([#2958](https://github.com/juspay/hyperswitch/pull/2958)) ([`42eedf3`](https://github.com/juspay/hyperswitch/commit/42eedf3a8c2e62fc22bcead370d129ebaf11a00b))
- Amount_captured goes to 0 for 3ds payments ([#2954](https://github.com/juspay/hyperswitch/pull/2954)) ([`75eea7e`](https://github.com/juspay/hyperswitch/commit/75eea7e81787f2e0697b930b82a8188193f8d51f))
- Make drainer sleep on every loop interval instead of cycle end ([#2951](https://github.com/juspay/hyperswitch/pull/2951)) ([`e8df690`](https://github.com/juspay/hyperswitch/commit/e8df69092f4c6acee58109aaff2a9454fceb571a))

### Refactors

- **connector:**
  - [Payeezy] update error message ([#2919](https://github.com/juspay/hyperswitch/pull/2919)) ([`cb65370`](https://github.com/juspay/hyperswitch/commit/cb653706066b889eaa9423a6227ce1df954b4759))
  - [Worldline] change error message from NotSupported to NotImplemented ([#2893](https://github.com/juspay/hyperswitch/pull/2893)) ([`e721b06`](https://github.com/juspay/hyperswitch/commit/e721b06c7077e00458450a4fb98f4497e8227dc6))

### Testing

- **postman:** Update postman collection files ([`9a3fa00`](https://github.com/juspay/hyperswitch/commit/9a3fa00426d74f6d18b3c712b292d98d80d517ba))

**Full Changelog:** [`v1.87.0...v1.88.0`](https://github.com/juspay/hyperswitch/compare/v1.87.0...v1.88.0)

- - -


## 1.87.0 (2023-11-22)

### Features

- **api_event_errors:** Error field in APIEvents ([#2808](https://github.com/juspay/hyperswitch/pull/2808)) ([`ce10579`](https://github.com/juspay/hyperswitch/commit/ce10579a729fe4a7d4ab9f1a4cbd38c3ca00e90b))
- **payment_methods:** Add support for tokenising bank details and fetching masked details while listing ([#2585](https://github.com/juspay/hyperswitch/pull/2585)) ([`9989489`](https://github.com/juspay/hyperswitch/commit/998948953ab8a444aca79957f48e7cfb3066c334))
- **router:**
  - Migrate `payment_method_data` to rust locker only if `payment_method` is card ([#2929](https://github.com/juspay/hyperswitch/pull/2929)) ([`f8261a9`](https://github.com/juspay/hyperswitch/commit/f8261a96e758498a32c988191bf314aa6c752059))
  - Add list payment link support ([#2805](https://github.com/juspay/hyperswitch/pull/2805)) ([`b441a1f`](https://github.com/juspay/hyperswitch/commit/b441a1f2f9d9d84601cf78a6e39145e8fb847593))
- **routing:** Routing prometheus metrics ([#2870](https://github.com/juspay/hyperswitch/pull/2870)) ([`4e15d77`](https://github.com/juspay/hyperswitch/commit/4e15d7792e3167de170c3d8310f33419f4dfb0db))

### Bug Fixes

- cybersource mandates and fiserv exp year ([#2920](https://github.com/juspay/hyperswitch/pull/2920)) ([`7f74ae9`](https://github.com/juspay/hyperswitch/commit/7f74ae98a1d48eed98341e4505d3801a61e69fc7))
- Kv logs when KeyNotSet is returned ([#2928](https://github.com/juspay/hyperswitch/pull/2928)) ([`6954de7`](https://github.com/juspay/hyperswitch/commit/6954de77a0fda14d87b79ec7ceee7cc8f1c491db))

### Refactors

- **macros:** Use syn2.0  ([#2890](https://github.com/juspay/hyperswitch/pull/2890)) ([`46e13d5`](https://github.com/juspay/hyperswitch/commit/46e13d54759168ad7667af08d5481ab510e5706a))
- **mca:** Add Serialization for `ConnectorAuthType` ([#2945](https://github.com/juspay/hyperswitch/pull/2945)) ([`341374b`](https://github.com/juspay/hyperswitch/commit/341374b8e5eced329587b93cbb6bd58e16dd9932))

### Testing

- **postman:** Update postman collection files ([`b96052f`](https://github.com/juspay/hyperswitch/commit/b96052f9c64dd6e49d52ba8befd1f60a843b482a))

### Documentation

- **README:** Update feature support link  ([#2894](https://github.com/juspay/hyperswitch/pull/2894)) ([`7d223ee`](https://github.com/juspay/hyperswitch/commit/7d223ee0d1b53c02421ed6bd1b5584362d7a7456))

### Miscellaneous Tasks

- Address Rust 1.74 clippy lints ([#2942](https://github.com/juspay/hyperswitch/pull/2942)) ([`c6a5a85`](https://github.com/juspay/hyperswitch/commit/c6a5a8574825dc333602f4f1cee7e26969eab030))

**Full Changelog:** [`v1.86.0...v1.87.0`](https://github.com/juspay/hyperswitch/compare/v1.86.0...v1.87.0)

- - -


## 1.86.0 (2023-11-21)

### Features

- **connector:** [Prophetpay] Save card token for Refund and remove Void flow ([#2927](https://github.com/juspay/hyperswitch/pull/2927)) ([`15a255e`](https://github.com/juspay/hyperswitch/commit/15a255ea60dffad9e4cf20d642636028c27c7c00))
- Add support for 3ds and surcharge decision through routing rules ([#2869](https://github.com/juspay/hyperswitch/pull/2869)) ([`f8618e0`](https://github.com/juspay/hyperswitch/commit/f8618e077065d94aa27d7153fc5ea6f93870bd81))

### Bug Fixes

- **mca:** Change the check for `disabled` field in mca create and update ([#2938](https://github.com/juspay/hyperswitch/pull/2938)) ([`e66ccde`](https://github.com/juspay/hyperswitch/commit/e66ccde4cf6d055b7d02c5e982d2e09364845602))
- Status goes from pending to partially captured in psync ([#2915](https://github.com/juspay/hyperswitch/pull/2915)) ([`3f3b797`](https://github.com/juspay/hyperswitch/commit/3f3b797dc65c1bc6f710b122ef00d5bcb409e600))

### Testing

- **postman:** Update postman collection files ([`245e489`](https://github.com/juspay/hyperswitch/commit/245e489d13209da19d6e9af01219056eec04e897))

**Full Changelog:** [`v1.85.0...v1.86.0`](https://github.com/juspay/hyperswitch/compare/v1.85.0...v1.86.0)

- - -


## 1.85.0 (2023-11-21)

### Features

- **mca:** Add new `auth_type` and a status field for mca ([#2883](https://github.com/juspay/hyperswitch/pull/2883)) ([`25cef38`](https://github.com/juspay/hyperswitch/commit/25cef386b8876b43893f20b93cd68ece6e68412d))
- **router:** Add unified_code, unified_message in payments response ([#2918](https://github.com/juspay/hyperswitch/pull/2918)) ([`3954001`](https://github.com/juspay/hyperswitch/commit/39540015fde476ad8492a9142c2c1bfda8444a27))

### Bug Fixes

- **connector:**
  - [fiserv] fix metadata deserialization in merchant_connector_account ([#2746](https://github.com/juspay/hyperswitch/pull/2746)) ([`644709d`](https://github.com/juspay/hyperswitch/commit/644709d95f6ecaab497cf0cf3788b9e2ed88b855))
  - [CASHTOCODE] Fix Error Response Handling ([#2926](https://github.com/juspay/hyperswitch/pull/2926)) ([`938b63a`](https://github.com/juspay/hyperswitch/commit/938b63a1fceb87b4aae4211dac4d051e024028b1))
- **router:** Associate parent payment token with `payment_method_id` as hyperswitch token for saved cards ([#2130](https://github.com/juspay/hyperswitch/pull/2130)) ([`efeebc0`](https://github.com/juspay/hyperswitch/commit/efeebc0f2365f0900de3dd3e10a1539621c9933d))
- Api lock on PaymentsCreate ([#2916](https://github.com/juspay/hyperswitch/pull/2916)) ([`cfabfa6`](https://github.com/juspay/hyperswitch/commit/cfabfa60db4d275066be72ee64153a34d38f13b8))
- Merchant_connector_id null in KV flow ([#2810](https://github.com/juspay/hyperswitch/pull/2810)) ([`e566a4e`](https://github.com/juspay/hyperswitch/commit/e566a4eff2270c2a56ec90966f42ccfd79906068))

### Refactors

- **connector:** [Paypal] Add support for both BodyKey and SignatureKey ([#2633](https://github.com/juspay/hyperswitch/pull/2633)) ([`d8fcd3c`](https://github.com/juspay/hyperswitch/commit/d8fcd3c9712480c1230590c4f23b35da79df784d))
- **core:** Query business profile only once ([#2830](https://github.com/juspay/hyperswitch/pull/2830)) ([`44deeb7`](https://github.com/juspay/hyperswitch/commit/44deeb7e7605cb5320b84c0fac1fd551877803a4))
- **payment_methods:** Added support for pm_auth_connector field in pm list response ([#2667](https://github.com/juspay/hyperswitch/pull/2667)) ([`be4aa3b`](https://github.com/juspay/hyperswitch/commit/be4aa3b913819698c6c22ddedafe1d90fbe02add))
- Add mapping for ConnectorError in payouts flow ([#2608](https://github.com/juspay/hyperswitch/pull/2608)) ([`5c4e7c9`](https://github.com/juspay/hyperswitch/commit/5c4e7c9031f62d63af35da2dcab79eac948e7dbb))

### Testing

- **postman:** Update postman collection files ([`ce725ef`](https://github.com/juspay/hyperswitch/commit/ce725ef8c680eea3fe03671c989fd4572cfc0640))

**Full Changelog:** [`v1.84.0...v1.85.0`](https://github.com/juspay/hyperswitch/compare/v1.84.0...v1.85.0)

- - -


## 1.84.0 (2023-11-17)

### Features

- **connector:** [BANKOFAMERICA] PSYNC Bugfix ([#2897](https://github.com/juspay/hyperswitch/pull/2897)) ([`bdcc138`](https://github.com/juspay/hyperswitch/commit/bdcc138e8d84577fc99f9a9aef3484b66f98209a))

**Full Changelog:** [`v1.83.1...v1.84.0`](https://github.com/juspay/hyperswitch/compare/v1.83.1...v1.84.0)

- - -


## 1.83.1 (2023-11-17)

### Bug Fixes

- **router:** Add choice to use the appropriate key for jws verification ([#2917](https://github.com/juspay/hyperswitch/pull/2917)) ([`606daa9`](https://github.com/juspay/hyperswitch/commit/606daa9367cac8c2ea926313019deab2f938b591))

**Full Changelog:** [`v1.83.0...v1.83.1`](https://github.com/juspay/hyperswitch/compare/v1.83.0...v1.83.1)

- - -


## 1.83.0 (2023-11-17)

### Features

- **events:** Add incoming webhook payload to api events logger ([#2852](https://github.com/juspay/hyperswitch/pull/2852)) ([`aea390a`](https://github.com/juspay/hyperswitch/commit/aea390a6a1c331f8e0dbea4f41218e43f7323508))
- **router:** Custom payment link config for payment create ([#2741](https://github.com/juspay/hyperswitch/pull/2741)) ([`c39beb2`](https://github.com/juspay/hyperswitch/commit/c39beb2501e63bbf7fd41bbc947280d7ff5a71dc))

### Bug Fixes

- **router:** Add rust locker url in proxy_bypass_urls ([#2902](https://github.com/juspay/hyperswitch/pull/2902)) ([`9a201ae`](https://github.com/juspay/hyperswitch/commit/9a201ae698c2cf52e617660f82d5bf1df2e797ae))

### Documentation

- **README:** Replace cloudformation deployment template with latest s3 url. ([#2891](https://github.com/juspay/hyperswitch/pull/2891)) ([`375108b`](https://github.com/juspay/hyperswitch/commit/375108b6df50e041fc9dbeb35a6a6b46b146037a))

**Full Changelog:** [`v1.82.0...v1.83.0`](https://github.com/juspay/hyperswitch/compare/v1.82.0...v1.83.0)

- - -


## 1.82.0 (2023-11-17)

### Features

- **router:** Add fallback while add card and retrieve card from rust locker ([#2888](https://github.com/juspay/hyperswitch/pull/2888)) ([`f735fb0`](https://github.com/juspay/hyperswitch/commit/f735fb0551812fd781a2db8bac5a0deef4cabb2b))

### Bug Fixes

- **core:** Introduce new attempt and intent status to handle multiple partial captures ([#2802](https://github.com/juspay/hyperswitch/pull/2802)) ([`cb88be0`](https://github.com/juspay/hyperswitch/commit/cb88be01f22725948648976c2a5606a03b5ce92a))

### Testing

- **postman:** Update postman collection files ([`7d05b74`](https://github.com/juspay/hyperswitch/commit/7d05b74b950d9e078b063e17d046cbeb501d006a))

**Full Changelog:** [`v1.81.0...v1.82.0`](https://github.com/juspay/hyperswitch/compare/v1.81.0...v1.82.0)

- - -


## 1.81.0 (2023-11-16)

### Features

- **connector:**
  - [BANKOFAMERICA] Implement Cards for Bank of America ([#2765](https://github.com/juspay/hyperswitch/pull/2765)) ([`e8de3a7`](https://github.com/juspay/hyperswitch/commit/e8de3a710710b92f5c2351c5d67c22352c2b0a30))
  - [ProphetPay] Implement Card Redirect PaymentMethodType and flows for Authorize, CompleteAuthorize, Psync, Refund, Rsync and Void ([#2641](https://github.com/juspay/hyperswitch/pull/2641)) ([`8d4adc5`](https://github.com/juspay/hyperswitch/commit/8d4adc52af57ed0994e6efbb5b2d0d3df3fb3150))

### Testing

- **postman:** Update postman collection files ([`f829197`](https://github.com/juspay/hyperswitch/commit/f8291973c38bde874c45ca15ff8d48c1f2de9781))

**Full Changelog:** [`v1.80.0...v1.81.0`](https://github.com/juspay/hyperswitch/compare/v1.80.0...v1.81.0)

- - -


## 1.80.0 (2023-11-16)

### Features

- **router:** Add api to migrate card from basilisk to rust ([#2853](https://github.com/juspay/hyperswitch/pull/2853)) ([`b8b20c4`](https://github.com/juspay/hyperswitch/commit/b8b20c412df0485bf395f9aa21e6e34e90d97acd))
- Spawn webhooks and async scheduling in background ([#2780](https://github.com/juspay/hyperswitch/pull/2780)) ([`f248fe2`](https://github.com/juspay/hyperswitch/commit/f248fe2889c9cb68af4464ab0db1735224ab5c8d))

### Refactors

- **router:** Add openapi spec support for gsm apis ([#2871](https://github.com/juspay/hyperswitch/pull/2871)) ([`62c9cca`](https://github.com/juspay/hyperswitch/commit/62c9ccae6ab0d128c54962675b88739ad7797fe6))

**Full Changelog:** [`v1.79.0...v1.80.0`](https://github.com/juspay/hyperswitch/compare/v1.79.0...v1.80.0)

- - -


## 1.79.0 (2023-11-16)

### Features

- Change async-bb8 fork and tokio spawn for concurrent database calls ([#2774](https://github.com/juspay/hyperswitch/pull/2774)) ([`d634fde`](https://github.com/juspay/hyperswitch/commit/d634fdeac349b92e3619234580299a6c6c38e6d4))

### Bug Fixes

- **connector:** [noon] add validate psync reference ([#2886](https://github.com/juspay/hyperswitch/pull/2886)) ([`b129023`](https://github.com/juspay/hyperswitch/commit/b1290234ba13de2dd8cc4210f63bae514c2988b4))
- **payment_link:** Render SDK for status requires_payment_method ([#2887](https://github.com/juspay/hyperswitch/pull/2887)) ([`d4d2c2c`](https://github.com/juspay/hyperswitch/commit/d4d2c2c7076a46996aa0aa74d1df827169f73155))
- Paypal postman collection changes for surcharge feature ([#2884](https://github.com/juspay/hyperswitch/pull/2884)) ([`5956242`](https://github.com/juspay/hyperswitch/commit/5956242588ef7bdbaa1804a952d48dc47c6e15f1))

### Testing

- **postman:** Update postman collection files ([`5c31365`](https://github.com/juspay/hyperswitch/commit/5c313656a129362b0e905e5fbf349dbbec57199c))

**Full Changelog:** [`v1.78.0...v1.79.0`](https://github.com/juspay/hyperswitch/compare/v1.78.0...v1.79.0)

- - -


## 1.78.0 (2023-11-14)

### Features

- **router:** Add automatic retries and step up 3ds flow ([#2834](https://github.com/juspay/hyperswitch/pull/2834)) ([`d2968c9`](https://github.com/juspay/hyperswitch/commit/d2968c94978a57422fa46a8195d906736a95b864))
- Payment link status page UI ([#2740](https://github.com/juspay/hyperswitch/pull/2740)) ([`856c7af`](https://github.com/juspay/hyperswitch/commit/856c7af77e17599ca0d4d119744ac582e9c3c971))

### Bug Fixes

- Handle session and confirm flow discrepancy in surcharge details ([#2696](https://github.com/juspay/hyperswitch/pull/2696)) ([`cafea45`](https://github.com/juspay/hyperswitch/commit/cafea45982d7b520fe68fde967984ce88f68c6c0))

**Full Changelog:** [`v1.77.0...v1.78.0`](https://github.com/juspay/hyperswitch/compare/v1.77.0...v1.78.0)

- - -


## 1.77.0 (2023-11-13)

### Features

- **apievent:** Added hs latency to api event ([#2734](https://github.com/juspay/hyperswitch/pull/2734)) ([`c124511`](https://github.com/juspay/hyperswitch/commit/c124511052ed8911a2ccfcf648c0793b5c1ca690))
- **router:**
  - Add new JWT authentication variants and use them ([#2835](https://github.com/juspay/hyperswitch/pull/2835)) ([`f88eee7`](https://github.com/juspay/hyperswitch/commit/f88eee7362be2cc3e8e8dc2bb7bfd263892ff01e))
  - Profile specific fallback derivation while routing payments ([#2806](https://github.com/juspay/hyperswitch/pull/2806)) ([`8e538db`](https://github.com/juspay/hyperswitch/commit/8e538dbd5c189047d0a0b24fa752b9a1c67554f5))

### Build System / Dependencies

- **deps:** Remove unused dependencies and features ([#2854](https://github.com/juspay/hyperswitch/pull/2854)) ([`0553587`](https://github.com/juspay/hyperswitch/commit/05535871152f4a6ac24ce6b5b5390da13cc29b96))

**Full Changelog:** [`v1.76.0...v1.77.0`](https://github.com/juspay/hyperswitch/compare/v1.76.0...v1.77.0)

- - -


## 1.76.0 (2023-11-12)

### Features

- **analytics:** Analytics APIs  ([#2792](https://github.com/juspay/hyperswitch/pull/2792)) ([`f847802`](https://github.com/juspay/hyperswitch/commit/f847802339bfedb24cbaa47ad55e31d80cefddca))
- **router:** Added Payment link new design ([#2731](https://github.com/juspay/hyperswitch/pull/2731)) ([`2a4f5d1`](https://github.com/juspay/hyperswitch/commit/2a4f5d13717a78dc2e2e4fc9a492a45b92151dbe))
- **user:** Setup user tables ([#2803](https://github.com/juspay/hyperswitch/pull/2803)) ([`20c4226`](https://github.com/juspay/hyperswitch/commit/20c4226a36e4650a3ba8811b758ac5f7969bcfb3))

### Refactors

- **connector:** [Zen] change error message from NotSupported to NotImplemented ([#2831](https://github.com/juspay/hyperswitch/pull/2831)) ([`b5ea8db`](https://github.com/juspay/hyperswitch/commit/b5ea8db2d2b7e7544931704a7191b42d3a8299be))
- **core:** Remove connector response table and use payment_attempt instead ([#2644](https://github.com/juspay/hyperswitch/pull/2644)) ([`966369b`](https://github.com/juspay/hyperswitch/commit/966369b6f2c205b59524c23ad3b21ebab547631f))
- **events:** Update api events to follow snake case naming ([#2828](https://github.com/juspay/hyperswitch/pull/2828)) ([`b3d5062`](https://github.com/juspay/hyperswitch/commit/b3d5062dc07676ec12e903b1999fdd9138c0891d))

### Documentation

- **README:** Add bootstrap button for cloudformation deployment ([#2827](https://github.com/juspay/hyperswitch/pull/2827)) ([`e67e808`](https://github.com/juspay/hyperswitch/commit/e67e808d70d41c371fff168824e5a4dbb8b3a040))

**Full Changelog:** [`v1.75.0...v1.76.0`](https://github.com/juspay/hyperswitch/compare/v1.75.0...v1.76.0)

- - -


## 1.75.0 (2023-11-09)

### Features

- **events:** Add extracted fields based on req/res types ([#2795](https://github.com/juspay/hyperswitch/pull/2795)) ([`8985794`](https://github.com/juspay/hyperswitch/commit/89857941b09c5fbe0f3e7d5b4f908bb144ae162d))
- **router:**
  - Added merchant custom name support for payment link ([#2685](https://github.com/juspay/hyperswitch/pull/2685)) ([`8b15189`](https://github.com/juspay/hyperswitch/commit/8b151898dc0d8eefe5ed2bbdafe59e8f58b4698c))
  - Add `gateway_status_map` CRUD APIs ([#2809](https://github.com/juspay/hyperswitch/pull/2809)) ([`5c9e235`](https://github.com/juspay/hyperswitch/commit/5c9e235bd30dd3e03d086a83613edfcc62b2ead2))

### Bug Fixes

- **analytics:** Added hs latency to api event for paymentconfirm call ([#2787](https://github.com/juspay/hyperswitch/pull/2787)) ([`aab8f60`](https://github.com/juspay/hyperswitch/commit/aab8f6035c16ca19009f8f1e0db688c17bc0b2b6))
- [mollie] locale validation irrespective of auth type ([#2814](https://github.com/juspay/hyperswitch/pull/2814)) ([`25a73c2`](https://github.com/juspay/hyperswitch/commit/25a73c29a4c4715a54862dd6a28c875fd3752f63))

**Full Changelog:** [`v1.74.0...v1.75.0`](https://github.com/juspay/hyperswitch/compare/v1.74.0...v1.75.0)

- - -


## 1.74.0 (2023-11-08)

### Features

- **core:** Use redis as temp locker instead of basilisk ([#2789](https://github.com/juspay/hyperswitch/pull/2789)) ([`6678689`](https://github.com/juspay/hyperswitch/commit/6678689265ae9a4fbb7a43c1938237d349c5a68e))
- **events:** Add request details to api events ([#2769](https://github.com/juspay/hyperswitch/pull/2769)) ([`164d1c6`](https://github.com/juspay/hyperswitch/commit/164d1c66fbcb84104db07412496114db2f8c5c0c))
- **router:** Add `gateway_status_map` interface ([#2804](https://github.com/juspay/hyperswitch/pull/2804)) ([`a429b23`](https://github.com/juspay/hyperswitch/commit/a429b23c7f21c9d08a79895c0b770b35aab725f7))
- **test_utils:** Add custom-headers and custom delay support to rustman ([#2636](https://github.com/juspay/hyperswitch/pull/2636)) ([`1effddd`](https://github.com/juspay/hyperswitch/commit/1effddd0a0d3985d6df03c4ae9be28712befc05e))

### Bug Fixes

- **connector:** Add attempt_status in field in error_response ([#2794](https://github.com/juspay/hyperswitch/pull/2794)) ([`5642fef`](https://github.com/juspay/hyperswitch/commit/5642fef52a6d591d12c5745ed381f41a1593f183))

### Refactors

- **config:** Update payment method filter of Klarna in Stripe ([#2807](https://github.com/juspay/hyperswitch/pull/2807)) ([`21ce807`](https://github.com/juspay/hyperswitch/commit/21ce8079f4cb11d70c5eaae78f83773141c67d0c))
- **router:** Add parameter connectors to get_request_body function ([#2708](https://github.com/juspay/hyperswitch/pull/2708)) ([`7623ea9`](https://github.com/juspay/hyperswitch/commit/7623ea93bee61b0bb22b68e86f44de17f04f876b))

### Documentation

- **README:** Update README ([#2800](https://github.com/juspay/hyperswitch/pull/2800)) ([`bef0a04`](https://github.com/juspay/hyperswitch/commit/bef0a04edc6323b3b7a2e0dd7eeb7954915ba7cf))

**Full Changelog:** [`v1.73.0...v1.74.0`](https://github.com/juspay/hyperswitch/compare/v1.73.0...v1.74.0)

- - -


## 1.73.0 (2023-11-07)

### Features

- **connector:**
  - [BANKOFAMERICA] Add Connector Template Code ([#2764](https://github.com/juspay/hyperswitch/pull/2764)) ([`4563935`](https://github.com/juspay/hyperswitch/commit/4563935372d2cdff3f746fa86a47f1166ffd32ac))
  - [Bitpay] Add order id as the reference id ([#2591](https://github.com/juspay/hyperswitch/pull/2591)) ([`d47d4ac`](https://github.com/juspay/hyperswitch/commit/d47d4ac682705d6ac692f9381149bbf08ad71264))
- **router:** Make webhook events config disabled only and by default enable all the events ([#2770](https://github.com/juspay/hyperswitch/pull/2770)) ([`d335879`](https://github.com/juspay/hyperswitch/commit/d335879f9289b57a90a76c6587a58a0b3e12c9ad))
- Make drainer logs queryable with request_id and global_id ([#2771](https://github.com/juspay/hyperswitch/pull/2771)) ([`ff73aba`](https://github.com/juspay/hyperswitch/commit/ff73aba8e72d8e072027881760335c0c818df665))

### Bug Fixes

- **connector:** Fix amount conversion incase of minor unit  ([#2793](https://github.com/juspay/hyperswitch/pull/2793)) ([`34f5226`](https://github.com/juspay/hyperswitch/commit/34f52260d3fa68b54e5b46207afaf2ad07a8d8ba))

### Refactors

- **payment_methods:** Added support for account subtype in pmd ([#2651](https://github.com/juspay/hyperswitch/pull/2651)) ([`e7375d0`](https://github.com/juspay/hyperswitch/commit/e7375d0e26099a7e0e6efd1b83b8eb9c7b1c5411))

### Documentation

- **README:** Add one-click deployment information using CDK ([#2798](https://github.com/juspay/hyperswitch/pull/2798)) ([`bb39cd4`](https://github.com/juspay/hyperswitch/commit/bb39cd4081fdcaf68b2b5de2234e93493dbd84b6))

**Full Changelog:** [`v1.72.0...v1.73.0`](https://github.com/juspay/hyperswitch/compare/v1.72.0...v1.73.0)

- - -


## 1.72.0 (2023-11-05)

### Features

- **connector:**
  - [ACI] Currency Unit Conversion ([#2750](https://github.com/juspay/hyperswitch/pull/2750)) ([`cdead78`](https://github.com/juspay/hyperswitch/commit/cdead78ea6a1f2dce92187f499f54498ba4bb173))
  - [Fiserv] Currency Unit Conversion ([#2715](https://github.com/juspay/hyperswitch/pull/2715)) ([`b6b9e4f`](https://github.com/juspay/hyperswitch/commit/b6b9e4f912e1c61cd31ab91be587ffb08c9f3a5b))
  - [Bitpay] Use `connector_request_reference_id` as reference to the connector ([#2697](https://github.com/juspay/hyperswitch/pull/2697)) ([`7141b89`](https://github.com/juspay/hyperswitch/commit/7141b89d231bae0c3b1c10095b88df16129b1665))
  - [NMI] Currency Unit Conversion ([#2707](https://github.com/juspay/hyperswitch/pull/2707)) ([`1b45a30`](https://github.com/juspay/hyperswitch/commit/1b45a302630ed8affc5abff0de1325fb5c6f870e))
  - [Payeezy] Currency Unit Conversion ([#2710](https://github.com/juspay/hyperswitch/pull/2710)) ([`25245b9`](https://github.com/juspay/hyperswitch/commit/25245b965371d93449f4584667adeb38ab7e0e59))

### Refactors

- **connector:** [Stax] Currency Unit Conversion ([#2711](https://github.com/juspay/hyperswitch/pull/2711)) ([`2782923`](https://github.com/juspay/hyperswitch/commit/278292322c7c06f4239dd73861469e436bd941fa))

### Testing

- **postman:** Update postman collection files ([`d11e7fd`](https://github.com/juspay/hyperswitch/commit/d11e7fd5642efe7da4b5021d87cf40f16d9eeded))

**Full Changelog:** [`v1.71.0...v1.72.0`](https://github.com/juspay/hyperswitch/compare/v1.71.0...v1.72.0)

- - -


## 1.71.0 (2023-11-03)

### Features

- **merchant_connector_account:** Add cache for querying by `merchant_connector_id` ([#2738](https://github.com/juspay/hyperswitch/pull/2738)) ([`1ba6282`](https://github.com/juspay/hyperswitch/commit/1ba6282699b7dff5e6e95c9a14e51c0f8bf749cd))
- **router:** Add Smart Routing to route payments efficiently ([#2665](https://github.com/juspay/hyperswitch/pull/2665)) ([`9b618d2`](https://github.com/juspay/hyperswitch/commit/9b618d24476967d364835d04010d9076a80aeb9c))

### Bug Fixes

- **connector:**
  - [Cryptopay]Remove default case handling for Cryptopay ([#2699](https://github.com/juspay/hyperswitch/pull/2699)) ([`255a4f8`](https://github.com/juspay/hyperswitch/commit/255a4f89a8e0124310d42bb63ad459bd8cde2cba))
  - [Bluesnap] fix psync status to failure when it is '403'  ([#2772](https://github.com/juspay/hyperswitch/pull/2772)) ([`9314d14`](https://github.com/juspay/hyperswitch/commit/9314d1446326fd8a69f1f69657a976bbe7c27901))
- Response spelling ([#2779](https://github.com/juspay/hyperswitch/pull/2779)) ([`5859372`](https://github.com/juspay/hyperswitch/commit/585937204d9071baa37d402f73159f8f650d0a07))

### Testing

- **postman:** Update postman collection files ([`21e8a10`](https://github.com/juspay/hyperswitch/commit/21e8a105f9b47ded232b457a0420ad71ec2414ed))

**Full Changelog:** [`v1.70.1...v1.71.0`](https://github.com/juspay/hyperswitch/compare/v1.70.1...v1.71.0)

- - -


## 1.70.1 (2023-11-03)

### Revert

- Fix(analytics): feat(analytics): analytics APIs ([#2777](https://github.com/juspay/hyperswitch/pull/2777)) ([`169d33b`](https://github.com/juspay/hyperswitch/commit/169d33bf8157b1a9910c841c8c55eddc4d2ad168))

**Full Changelog:** [`v1.70.0...v1.70.1`](https://github.com/juspay/hyperswitch/compare/v1.70.0...v1.70.1)

- - -


## 1.70.0 (2023-11-03)

### Features

- **analytics:** Analytics APIs ([#2676](https://github.com/juspay/hyperswitch/pull/2676)) ([`c0a5e7b`](https://github.com/juspay/hyperswitch/commit/c0a5e7b7d945095053606e35c9bb23a06090c4e3))
- **connector:** [Multisafepay] add error handling ([#2595](https://github.com/juspay/hyperswitch/pull/2595)) ([`b3c846d`](https://github.com/juspay/hyperswitch/commit/b3c846d637dd32a2d6d7044c118abbb2616642f0))
- **events:** Add api auth type details to events ([#2760](https://github.com/juspay/hyperswitch/pull/2760)) ([`1094493`](https://github.com/juspay/hyperswitch/commit/10944937a02502e0727f16368d8d055e575dd518))

### Bug Fixes

- **router:** Make customer_id optional when billing and shipping address is passed in payments create, update ([#2762](https://github.com/juspay/hyperswitch/pull/2762)) ([`e40a293`](https://github.com/juspay/hyperswitch/commit/e40a29351c7aa7b86a5684959a84f0236104cafd))
- Null fields in payments response ([#2745](https://github.com/juspay/hyperswitch/pull/2745)) ([`42261a5`](https://github.com/juspay/hyperswitch/commit/42261a5306bb99d3e20eb3aa734a895e589b1d94))

### Testing

- **postman:** Update postman collection files ([`772f03e`](https://github.com/juspay/hyperswitch/commit/772f03ee3836ce86de3874f6a5e7f636718e6034))

**Full Changelog:** [`v1.69.0...v1.70.0`](https://github.com/juspay/hyperswitch/compare/v1.69.0...v1.70.0)

- - -


## 1.69.0 (2023-10-31)

### Features

- **connector:**
  - [VOLT] Implement payment flows and bank redirect payment method ([#2582](https://github.com/juspay/hyperswitch/pull/2582)) ([`23bd364`](https://github.com/juspay/hyperswitch/commit/23bd364a7819a48c3f5f89ff5b71cc237d6e2d46))
  - [NMI] add orderid to PaymentRequest ([#2727](https://github.com/juspay/hyperswitch/pull/2727)) ([`aad3f0f`](https://github.com/juspay/hyperswitch/commit/aad3f0f6fafdb08f1c5f1feb2588d6d0fb9162ff))
  - Worldline Use `connector_response_reference_id` as reference to merchant ([#2721](https://github.com/juspay/hyperswitch/pull/2721)) ([`a261f1a`](https://github.com/juspay/hyperswitch/commit/a261f1a2fce84354b3741429b629928d1bd06aab))
  - [Authorizedotnet] Use connector_request_reference_id as reference to the connector ([#2593](https://github.com/juspay/hyperswitch/pull/2593)) ([`3d7c6b0`](https://github.com/juspay/hyperswitch/commit/3d7c6b004d5f6399858925b40c3010fca486bbd5))
  - [Multisafepay] Currency Unit Conversion ([#2679](https://github.com/juspay/hyperswitch/pull/2679)) ([`42b13f7`](https://github.com/juspay/hyperswitch/commit/42b13f737a53143057ab23867f32017ea8c17780))
  - [Iatapay] currency unit conversion ([#2592](https://github.com/juspay/hyperswitch/pull/2592)) ([`0f5406c`](https://github.com/juspay/hyperswitch/commit/0f5406c620e9cdd20841898e9451a35f434f5b8a))
  - [BitPay] Currency Unit Conversion ([#2736](https://github.com/juspay/hyperswitch/pull/2736)) ([`e377279`](https://github.com/juspay/hyperswitch/commit/e377279d9cc872238fcfd8de324b44b0249b95c2))
- **organization:** Add organization table ([#2669](https://github.com/juspay/hyperswitch/pull/2669)) ([`d682471`](https://github.com/juspay/hyperswitch/commit/d6824710015b134a50986b3e85d3840902322711))
- Add one-click deploy script for HyperSwitch on AWS (EC2, RDS, Redis) ([#2730](https://github.com/juspay/hyperswitch/pull/2730)) ([`838372a`](https://github.com/juspay/hyperswitch/commit/838372ab3f6f3f35b8d884958810bab54cc17244))
- Implement list_merchant_connector_accounts_by_merchant_id_connector_name function ([#2742](https://github.com/juspay/hyperswitch/pull/2742)) ([`15a6b5a`](https://github.com/juspay/hyperswitch/commit/15a6b5a855def5650e16b96e6529ad7fa0845e6b))

### Bug Fixes

- **connector:** [Stripe] add decline_code in error_reason ([#2735](https://github.com/juspay/hyperswitch/pull/2735)) ([`0a44f56`](https://github.com/juspay/hyperswitch/commit/0a44f5699ed7b0c0ea0352b67c65df496ebe61f3))
- **typo:** Add commit id to allowed typos ([#2733](https://github.com/juspay/hyperswitch/pull/2733)) ([`8984627`](https://github.com/juspay/hyperswitch/commit/8984627d1cfd1a773e931617a3351884b12399a5))
- Make kv log extraction easier ([#2666](https://github.com/juspay/hyperswitch/pull/2666)) ([`577ef1a`](https://github.com/juspay/hyperswitch/commit/577ef1ae1a4718aaf90175d49e2a786af255fd63))

### Refactors

- **connector:**
  - [Noon] Remove Default Case Handling ([#2677](https://github.com/juspay/hyperswitch/pull/2677)) ([`452090d`](https://github.com/juspay/hyperswitch/commit/452090d56d713a5cc5c8fae3cc2f9f3d26e27a53))
  - [Payme] Remove Default Case Handling ([#2719](https://github.com/juspay/hyperswitch/pull/2719)) ([`94947bd`](https://github.com/juspay/hyperswitch/commit/94947bdb33ca4eb91daad13b2a427592d3b69851))
  - [Payeezy] remove default case handling ([#2712](https://github.com/juspay/hyperswitch/pull/2712)) ([`ceed76f`](https://github.com/juspay/hyperswitch/commit/ceed76fb2e67771048e563a13703eb801eeaae08))
- **core:** Use `business_profile` to read merchant configs ([#2729](https://github.com/juspay/hyperswitch/pull/2729)) ([`8c85173`](https://github.com/juspay/hyperswitch/commit/8c85173ecdd13db5ec7c4c0fe18456a31c8ee57e))
- **db:** Migrate to payment_attempt from connector_response  ([#2656](https://github.com/juspay/hyperswitch/pull/2656)) ([`9d9fc2a`](https://github.com/juspay/hyperswitch/commit/9d9fc2a8c5e9e30ed7ed4eeb2417365fc06be711))

### Testing

- **postman:** Update postman collection files ([`db8f58b`](https://github.com/juspay/hyperswitch/commit/db8f58b145feef371c958086a1ec02128680d018))

### Miscellaneous Tasks

- **env:** Add ttl as env variable ([#2653](https://github.com/juspay/hyperswitch/pull/2653)) ([`8b1499e`](https://github.com/juspay/hyperswitch/commit/8b1499e121678c5df3ca0197e2ec14074fd96eb5))

**Full Changelog:** [`v1.68.0...v1.69.0`](https://github.com/juspay/hyperswitch/compare/v1.68.0...v1.69.0)

- - -


## 1.68.0 (2023-10-29)

### Features

- **connector:**
  - [OpenNode] Currency Unit Conversion ([#2645](https://github.com/juspay/hyperswitch/pull/2645)) ([`88e1f29`](https://github.com/juspay/hyperswitch/commit/88e1f29dae13622bc58b8f5df1cd84b929b28ac6))
  - [Mollie] Currency Unit Conversion ([#2671](https://github.com/juspay/hyperswitch/pull/2671)) ([`3578db7`](https://github.com/juspay/hyperswitch/commit/3578db7640d8eda8f063e11b8bb64452fb987eef))
  - [Dlocal] Implement feature to use connector_request_reference_id as reference to the connector ([#2704](https://github.com/juspay/hyperswitch/pull/2704)) ([`af90089`](https://github.com/juspay/hyperswitch/commit/af90089010e06ed45a70c51d4143260eec45b6dc))
- **events:** Add masked json serializer for logging PII values ([#2681](https://github.com/juspay/hyperswitch/pull/2681)) ([`13c66df`](https://github.com/juspay/hyperswitch/commit/13c66df92c5b7db9e44852d4afee7a4e5ae52a15))

### Bug Fixes

- **connector:** [Forte] Response Handling for Verify Action ([#2601](https://github.com/juspay/hyperswitch/pull/2601)) ([`efed596`](https://github.com/juspay/hyperswitch/commit/efed5968236a8ae3b26a7697e4972f243add4292))

### Refactors

- **connector:**
  - [Airwallex] Remove default case handling ([#2703](https://github.com/juspay/hyperswitch/pull/2703)) ([`4138c8f`](https://github.com/juspay/hyperswitch/commit/4138c8f5431dea4fe400b47c919c68b7c8f7b402))
  - Use connector_request_reference_id for Fiserv ([#2698](https://github.com/juspay/hyperswitch/pull/2698)) ([`05c2f84`](https://github.com/juspay/hyperswitch/commit/05c2f842e3b9c579f611716b08a10766a6d13a30))
  - [Rapyd] add and implement the get_currency_unit function ([#2664](https://github.com/juspay/hyperswitch/pull/2664)) ([`78e5cd0`](https://github.com/juspay/hyperswitch/commit/78e5cd00b55ad2bd25083aecceaa8762efe3b48d))
  - [Square] remove default case handling ([#2701](https://github.com/juspay/hyperswitch/pull/2701)) ([`05100ea`](https://github.com/juspay/hyperswitch/commit/05100ea38d540d17e211e06ea99fcfeae7958975))
  - Use connector_request_reference_id for Iatapay ([#2692](https://github.com/juspay/hyperswitch/pull/2692)) ([`4afe552`](https://github.com/juspay/hyperswitch/commit/4afe552563c6a0cb9544a9a2f870bb9d07d7cf18))

### Testing

- **postman:** Update postman collection files ([`8eca66a`](https://github.com/juspay/hyperswitch/commit/8eca66a2eb8770783c671b299765aa15d7fa72f8))

### Documentation

- **changelog:** Fix typo in changelog ([#2713](https://github.com/juspay/hyperswitch/pull/2713)) ([`2815443`](https://github.com/juspay/hyperswitch/commit/2815443c1b147e005a2384ff817292b1845a9f88))

**Full Changelog:** [`v1.67.0...v1.68.0`](https://github.com/juspay/hyperswitch/compare/v1.67.0...v1.68.0)

- - -


## 1.67.0 (2023-10-26)

### Features

- **connector:** [OpenNode] Use connector_request_reference_id as reference to connector ([#2596](https://github.com/juspay/hyperswitch/pull/2596)) ([`96b790c`](https://github.com/juspay/hyperswitch/commit/96b790cb4b44cd4867be62e2889cb4aa23622161))

### Bug Fixes

- **connector:** [Paypal]fix paypal error reason mapping when it is empty string. ([#2700](https://github.com/juspay/hyperswitch/pull/2700)) ([`2c00767`](https://github.com/juspay/hyperswitch/commit/2c007675aec13b0696c74568af36eea2c799d9ef))

### Refactors

- **connector:**
  - [Worldpay] Remove Default Case Handling ([#2488](https://github.com/juspay/hyperswitch/pull/2488)) ([`2b2c381`](https://github.com/juspay/hyperswitch/commit/2b2c38146dc6dcf8d967dcc557281d3689bf746b))
  - Added default case for Opayo ([#2687](https://github.com/juspay/hyperswitch/pull/2687)) ([`1186f8c`](https://github.com/juspay/hyperswitch/commit/1186f8c4e2f04f470f4d6c058c18cd63f35b3804))
- **router:** Tsys default case handling ([#2672](https://github.com/juspay/hyperswitch/pull/2672)) ([`9ff2721`](https://github.com/juspay/hyperswitch/commit/9ff272121a4b6d8d5e1565863d7f13caf06785b1))

### Testing

- **postman:** Update postman collection files ([`9875687`](https://github.com/juspay/hyperswitch/commit/9875687e044a3b5f916fd65b9e457caec7f4e0f6))

### Build System / Dependencies

- **docker:** Copy over `.gitignore` as `.dockerignore` ([#2691](https://github.com/juspay/hyperswitch/pull/2691)) ([`d680eb2`](https://github.com/juspay/hyperswitch/commit/d680eb2b49f85795daafdda9caa0fd3fe6db8108))

**Full Changelog:** [`v1.66.0...v1.67.0`](https://github.com/juspay/hyperswitch/compare/v1.66.0...v1.67.0)

- - -


## 1.66.0 (2023-10-25)

### Features

- **core:** Add support for multiple `merchant_connector_account`  ([#2655](https://github.com/juspay/hyperswitch/pull/2655)) ([`5988d8d`](https://github.com/juspay/hyperswitch/commit/5988d8d42605af006fdf7d7821bbdf66e4468669))

**Full Changelog:** [`v1.65.0...v1.66.0`](https://github.com/juspay/hyperswitch/compare/v1.65.0...v1.66.0)

- - -


## 1.65.0 (2023-10-25)

### Features

- **router_env:** Add support for UUID v7 for tracing actix web ([#2661](https://github.com/juspay/hyperswitch/pull/2661)) ([`65319fe`](https://github.com/juspay/hyperswitch/commit/65319fe958aaf88e48e06f731ffae8273f7b586c))

### Bug Fixes

- **core:** Address clippy config changes ([#2654](https://github.com/juspay/hyperswitch/pull/2654)) ([`cfe9c25`](https://github.com/juspay/hyperswitch/commit/cfe9c2529e3c16f4d43df37f6357c70f7ca39aa6))
- **refunds:**
  - Add `profile_id` in refunds response ([#2652](https://github.com/juspay/hyperswitch/pull/2652)) ([`bb86cc2`](https://github.com/juspay/hyperswitch/commit/bb86cc2d04665ccd68eebea68a3d5b58f481c63d))
  - Fetch refund if insert fails due to duplicate response ([#2682](https://github.com/juspay/hyperswitch/pull/2682)) ([`433cdfa`](https://github.com/juspay/hyperswitch/commit/433cdfa296849a9e642eb574bf79ee1b03b89ff6))

### Refactors

- **connector:**
  - [CryptoPay] Remove Default Case Handling ([#2643](https://github.com/juspay/hyperswitch/pull/2643)) ([`6428d07`](https://github.com/juspay/hyperswitch/commit/6428d07f983026245159de4147b62bc0fc018165))
  - [CyberSource] Enhance currency Mapping with ConnectorCurrencyCommon Trait ([#2626](https://github.com/juspay/hyperswitch/pull/2626)) ([`f2f8170`](https://github.com/juspay/hyperswitch/commit/f2f8170ae1bcc2167f5bc2dfcc58f0c9f1ea0160))
  - [Cryptopay] add psync reference id validation for Cryptopay ([#2668](https://github.com/juspay/hyperswitch/pull/2668)) ([`27b9762`](https://github.com/juspay/hyperswitch/commit/27b97626245cab12dd9aefb4d85a77b5c913dba0))
  - Default case for worldline ([#2674](https://github.com/juspay/hyperswitch/pull/2674)) ([`e6272c6`](https://github.com/juspay/hyperswitch/commit/e6272c6418e5dbf9af94c48ef8814d5f415de793))

### Testing

- **postman:** Update postman collection files ([`b340673`](https://github.com/juspay/hyperswitch/commit/b34067312ee7a5bc3c1498a1ff06e52849c90081))

**Full Changelog:** [`v1.64.1...v1.65.0`](https://github.com/juspay/hyperswitch/compare/v1.64.1...v1.65.0)

- - -


## 1.64.1 (2023-10-24)

### Refactors

- Revert redis temp locker logic ([#2670](https://github.com/juspay/hyperswitch/pull/2670)) ([`eaa9720`](https://github.com/juspay/hyperswitch/commit/eaa972052024678ade122eec14261f9f33788e45))

**Full Changelog:** [`v1.64.0...v1.64.1`](https://github.com/juspay/hyperswitch/compare/v1.64.0...v1.64.1)

- - -


## 1.64.0 (2023-10-23)

### Features

- **events:** Add request body to api events logger ([#2660](https://github.com/juspay/hyperswitch/pull/2660)) ([`830eee9`](https://github.com/juspay/hyperswitch/commit/830eee94e1d35dcd14ef9989eb7b6003c1244a18))

### Bug Fixes

- **router:** Disable openapi examples ([#2648](https://github.com/juspay/hyperswitch/pull/2648)) ([`b39bdbf`](https://github.com/juspay/hyperswitch/commit/b39bdbf0c24730fea9cde0dcfa07ac43e4dd69a4))

### Refactors

- **connector:**
  - Use connector_response_reference_id  for Shift4 ([#2492](https://github.com/juspay/hyperswitch/pull/2492)) ([`83f0062`](https://github.com/juspay/hyperswitch/commit/83f0062aad9886a5a0c4ecff7412acfec63f7423))
  - [PowerTranz] refactor powertranz payments to remove default cases ([#2547](https://github.com/juspay/hyperswitch/pull/2547)) ([`664093d`](https://github.com/juspay/hyperswitch/commit/664093dc79743203196d912c17570885718b1c02))

**Full Changelog:** [`v1.63.0...v1.64.0`](https://github.com/juspay/hyperswitch/compare/v1.63.0...v1.64.0)

- - -


## 1.63.0 (2023-10-20)

### Features

- Add support for updating surcharge_applicable field intent ([#2647](https://github.com/juspay/hyperswitch/pull/2647)) ([`949937e`](https://github.com/juspay/hyperswitch/commit/949937e3644346f8b2b952944efb884f270645a8))

### Bug Fixes

- Kms decryption of redis_temp_locker_encryption_key ([#2650](https://github.com/juspay/hyperswitch/pull/2650)) ([`5a6601f`](https://github.com/juspay/hyperswitch/commit/5a6601fad4d11cd7d2f1322a6453504494d20c6f))

### Refactors

- **router:** [Nexi nets] Remove Default Case Handling ([#2639](https://github.com/juspay/hyperswitch/pull/2639)) ([`4b64c56`](https://github.com/juspay/hyperswitch/commit/4b64c563558d7c0a02b248c23921ed47ff294980))

**Full Changelog:** [`v1.62.0...v1.63.0`](https://github.com/juspay/hyperswitch/compare/v1.62.0...v1.63.0)

- - -


## 1.62.0 (2023-10-19)

### Features

- **connector:**
  - [Worldpay] Use connector_request_reference_id as reference to the connector ([#2553](https://github.com/juspay/hyperswitch/pull/2553)) ([`9ea5830`](https://github.com/juspay/hyperswitch/commit/9ea5830befe333270f8f424753e1b46a439e79bb))
  - [ProphetPay] Template generation ([#2610](https://github.com/juspay/hyperswitch/pull/2610)) ([`7e6207e`](https://github.com/juspay/hyperswitch/commit/7e6207e6ca98fe2af9a61e272735e9d2292d6a92))
  - [Bambora] Use connector_response_reference_id as reference to the connector ([#2635](https://github.com/juspay/hyperswitch/pull/2635)) ([`a9b5dc9`](https://github.com/juspay/hyperswitch/commit/a9b5dc9ab767eb54a95bcebc4fd5a7b00dbf65f6))
  - [Klarna] Add order id as the reference id to merchant ([#2614](https://github.com/juspay/hyperswitch/pull/2614)) ([`b7d5573`](https://github.com/juspay/hyperswitch/commit/b7d557367a3a5aca478ffd2087af8077bc4e7e2b))

### Bug Fixes

- Payment_method_data and description null during payment confirm ([#2618](https://github.com/juspay/hyperswitch/pull/2618)) ([`6765a1c`](https://github.com/juspay/hyperswitch/commit/6765a1c695493499d1907c56d05bdcd80a2fea93))

### Refactors

- **connector:**
  - [Dlocal] Currency Unit Conversion ([#2615](https://github.com/juspay/hyperswitch/pull/2615)) ([`1f2fe51`](https://github.com/juspay/hyperswitch/commit/1f2fe5170ae318a8b1613f6f02538a36f30f0b3d))
  - [Iatapay] remove default case handling ([#2587](https://github.com/juspay/hyperswitch/pull/2587)) ([`6494e8a`](https://github.com/juspay/hyperswitch/commit/6494e8a6e4a195ecc9ca5b2f6ac0a636f06b03f7))
  - [noon] remove cancellation_reason ([#2627](https://github.com/juspay/hyperswitch/pull/2627)) ([`41b7742`](https://github.com/juspay/hyperswitch/commit/41b7742b5498bfa9ef32b9408ab2d9a7a43b01dc))
  - [Forte] Remove Default Case Handling ([#2625](https://github.com/juspay/hyperswitch/pull/2625)) ([`418715b`](https://github.com/juspay/hyperswitch/commit/418715b816337bcaeee1aceeb911e6d329add2ad))
  - [Dlocal] remove default case handling ([#2624](https://github.com/juspay/hyperswitch/pull/2624)) ([`1584313`](https://github.com/juspay/hyperswitch/commit/158431391d560be4a79160ccea7bf5feaa4b52db))
- Remove code related to temp locker ([#2640](https://github.com/juspay/hyperswitch/pull/2640)) ([`cc0b422`](https://github.com/juspay/hyperswitch/commit/cc0b42263257b6cf6c7f94350442a74d3c02750b))
- Add surcharge_applicable to payment_intent and remove surcharge_metadata from payment_attempt ([#2642](https://github.com/juspay/hyperswitch/pull/2642)) ([`e5fbaae`](https://github.com/juspay/hyperswitch/commit/e5fbaae0d4278681e5f589aa46c867e7904c4646))

### Testing

- **postman:** Update postman collection files ([`2593dd1`](https://github.com/juspay/hyperswitch/commit/2593dd17c30d7f327b54f3c386a9fd42ae8146ca))

### Miscellaneous Tasks

- **deps:** Bump rustix from 0.37.24 to 0.37.25 ([#2637](https://github.com/juspay/hyperswitch/pull/2637)) ([`67d0062`](https://github.com/juspay/hyperswitch/commit/67d006272158372a4b9ec65cbbe7b2ae8f35eb69))

### Build System / Dependencies

- **deps:** Use `async-bb8-diesel` from `crates.io` instead of git repository ([#2619](https://github.com/juspay/hyperswitch/pull/2619)) ([`14c0821`](https://github.com/juspay/hyperswitch/commit/14c0821b8085279072db3484a3b1bcdde0f7893b))

**Full Changelog:** [`v1.61.0...v1.62.0`](https://github.com/juspay/hyperswitch/compare/v1.61.0...v1.62.0)

- - -


## 1.61.0 (2023-10-18)

### Features

- **Connector:** [Paypal] add support for dispute webhooks for paypal connector ([#2353](https://github.com/juspay/hyperswitch/pull/2353)) ([`6cf8f05`](https://github.com/juspay/hyperswitch/commit/6cf8f0582cfa4f6a58c67a868cb67846970b3835))
- **apple_pay:** Add support for decrypted apple pay token for checkout ([#2628](https://github.com/juspay/hyperswitch/pull/2628)) ([`794dbc6`](https://github.com/juspay/hyperswitch/commit/794dbc6a766d12ff3cdf0b782abb4c48b8fa77d0))
- **connector:**
  - [Aci] Update connector_response_reference_id with merchant reference ([#2551](https://github.com/juspay/hyperswitch/pull/2551)) ([`9e450b8`](https://github.com/juspay/hyperswitch/commit/9e450b81ca8bc4b1ddbbe2c1d732dbc58c61934e))
  - [Bambora] use connector_request_reference_id ([#2518](https://github.com/juspay/hyperswitch/pull/2518)) ([`73e9391`](https://github.com/juspay/hyperswitch/commit/73e93910cd3bd668d721b15edb86240adc18f46b))
  - [Tsys] Use connector_request_reference_id as reference to the connector ([#2631](https://github.com/juspay/hyperswitch/pull/2631)) ([`b145463`](https://github.com/juspay/hyperswitch/commit/b1454634259144d896716e5cef37d9b8491f55b9))
- **core:** Replace temp locker with redis ([#2594](https://github.com/juspay/hyperswitch/pull/2594)) ([`2edbd61`](https://github.com/juspay/hyperswitch/commit/2edbd6123512a6f2f4d51d5c2d1ed8b6ee502813))
- **events:** Add events for incoming API requests ([#2621](https://github.com/juspay/hyperswitch/pull/2621)) ([`7a76d6c`](https://github.com/juspay/hyperswitch/commit/7a76d6c01a0c6087c6429e58cc9dd6b4ea7fc0aa))
- **merchant_account:** Add merchant account list endpoint  ([#2560](https://github.com/juspay/hyperswitch/pull/2560)) ([`a1472c6`](https://github.com/juspay/hyperswitch/commit/a1472c6b78afa819cbe026a7db1e0c2b9016715e))
- Update surcharge_amount and tax_amount in update_trackers of payment_confirm ([#2603](https://github.com/juspay/hyperswitch/pull/2603)) ([`2f9a355`](https://github.com/juspay/hyperswitch/commit/2f9a3557f63150bcd27e27c6510a799669706718))

### Bug Fixes

- **connector:**
  - [Authorizedotnet]fix error deserialization incase of authentication failure ([#2600](https://github.com/juspay/hyperswitch/pull/2600)) ([`4859b7d`](https://github.com/juspay/hyperswitch/commit/4859b7da73125c2da72f4754863ff4485bebce29))
  - [Paypal]fix error deserelization for source verification call ([#2611](https://github.com/juspay/hyperswitch/pull/2611)) ([`da77d13`](https://github.com/juspay/hyperswitch/commit/da77d1393b8f6ab658dd7f3c202dd6c7d15c0ebd))
- **payments:** Fix payment update enum being inserted into kv ([#2612](https://github.com/juspay/hyperswitch/pull/2612)) ([`9aa1c75`](https://github.com/juspay/hyperswitch/commit/9aa1c75eca24caa14af5f4801173cd59f76d7e57))

### Refactors

- **events:** Allow box dyn for event handler ([#2629](https://github.com/juspay/hyperswitch/pull/2629)) ([`01410bb`](https://github.com/juspay/hyperswitch/commit/01410bb9f233637e98f27ebe509e859c7dad2cf4))
- **payment_connector:** Allow connector label to be updated ([#2622](https://github.com/juspay/hyperswitch/pull/2622)) ([`c86ac9b`](https://github.com/juspay/hyperswitch/commit/c86ac9b1fe5388666463aa16c899427a2bf442fb))
- **router:** Remove unnecessary function from Refunds Validate Flow ([#2609](https://github.com/juspay/hyperswitch/pull/2609)) ([`3399328`](https://github.com/juspay/hyperswitch/commit/3399328ae7f525fb72e0751182cf32d0b2470594))
- Refactor connector auth type failure to 4xx ([#2616](https://github.com/juspay/hyperswitch/pull/2616)) ([`1dad745`](https://github.com/juspay/hyperswitch/commit/1dad7455c4ae8d26d52c44d90f5b8d815d85d205))

### Testing

- **postman:** Update postman collection files ([`d899025`](https://github.com/juspay/hyperswitch/commit/d89902507486b8b97011fb63ed0343f727255ca2))

### Documentation

- **postman:** Rewrite postman documentation to help devs develop tests for their features ([#2613](https://github.com/juspay/hyperswitch/pull/2613)) ([`1548ee6`](https://github.com/juspay/hyperswitch/commit/1548ee62b661200fcb9d439d16c072a66dbfa718))

### Miscellaneous Tasks

- **scripts:** Add connector script changes ([#2620](https://github.com/juspay/hyperswitch/pull/2620)) ([`373a10b`](https://github.com/juspay/hyperswitch/commit/373a10beffc7cddef6ff76f5c8fff91ca3618581))

**Full Changelog:** [`v1.60.0...v1.61.0`](https://github.com/juspay/hyperswitch/compare/v1.60.0...v1.61.0)

- - -


## 1.60.0 (2023-10-17)

### Features

- **compatibility:** Added support to connector txn id ([#2606](https://github.com/juspay/hyperswitch/pull/2606)) ([`82980a8`](https://github.com/juspay/hyperswitch/commit/82980a86ad7966c6645d26a4abec85c8c7e3bdad))
- **router:** Better UI payment link and order details product image and merchant config support ([#2583](https://github.com/juspay/hyperswitch/pull/2583)) ([`fdd9580`](https://github.com/juspay/hyperswitch/commit/fdd95800127bb79fe2a9eeca1b7e0e158b6d2783))
- Add updated_by to tracker tables ([#2604](https://github.com/juspay/hyperswitch/pull/2604)) ([`6a74e8c`](https://github.com/juspay/hyperswitch/commit/6a74e8cba9078529fd9662d29ac7b941a191fbf4))

### Bug Fixes

- Make push to drainer generic and add application metrics for KV ([#2563](https://github.com/juspay/hyperswitch/pull/2563)) ([`274a783`](https://github.com/juspay/hyperswitch/commit/274a78343e5e3de614cfb1476570b5c449ee0c1e))

### Refactors

- **connector:** [Nuvei] remove default case handling ([#2584](https://github.com/juspay/hyperswitch/pull/2584)) ([`3807601`](https://github.com/juspay/hyperswitch/commit/3807601ee1c140310abf7a7e6ee4b83d44de9558))
- **router:** Throw bad request error on applepay verification failure ([#2607](https://github.com/juspay/hyperswitch/pull/2607)) ([`cecea87`](https://github.com/juspay/hyperswitch/commit/cecea8718a48b4e896b2bafce0f909ef8d9a6e8a))

**Full Changelog:** [`v1.59.0...v1.60.0`](https://github.com/juspay/hyperswitch/compare/v1.59.0...v1.60.0)

- - -


## 1.59.0 (2023-10-16)

### Features

- **connector:**
  - Add support for surcharge in trustpay ([#2581](https://github.com/juspay/hyperswitch/pull/2581)) ([`2d5d3b8`](https://github.com/juspay/hyperswitch/commit/2d5d3b8efbf782bf03e5f5ef1aa557d3dd3f5860))
  - Add surcharge support in paypal connector ([#2568](https://github.com/juspay/hyperswitch/pull/2568)) ([`92ee1db`](https://github.com/juspay/hyperswitch/commit/92ee1db107ac41326ecfb31b4565664a29a4b80a))
- **events:** Add basic event handler to collect application events ([#2602](https://github.com/juspay/hyperswitch/pull/2602)) ([`5d88dbc`](https://github.com/juspay/hyperswitch/commit/5d88dbc92ce470c951717debe246e182b3fe5656))

### Refactors

- **connector:** [multisafepay] Remove Default Case Handling ([#2586](https://github.com/juspay/hyperswitch/pull/2586)) ([`7adc6a0`](https://github.com/juspay/hyperswitch/commit/7adc6a05b60fa9143260b2a7f623907647557621))

**Full Changelog:** [`v1.58.0...v1.59.0`](https://github.com/juspay/hyperswitch/compare/v1.58.0...v1.59.0)

- - -


## 1.58.0 (2023-10-15)

### Features

- **connector:**
  - [HELCIM] Implement Cards for Helcim ([#2210](https://github.com/juspay/hyperswitch/pull/2210)) ([`b5feab6`](https://github.com/juspay/hyperswitch/commit/b5feab61d950921c75267ad88e944e7e2c4af3ca))
  - [Paypal] use connector request reference id as reference for paypal ([#2577](https://github.com/juspay/hyperswitch/pull/2577)) ([`500405d`](https://github.com/juspay/hyperswitch/commit/500405d78938772e0e9f8e3ce4f930d782c670fa))
  - [Airwallex] Currency Unit Conversion ([#2571](https://github.com/juspay/hyperswitch/pull/2571)) ([`8971b17`](https://github.com/juspay/hyperswitch/commit/8971b17b073315f869e3c843b0aee7644dcf6479))
  - [Klarna] Use connector_request_reference_id as reference to connector ([#2494](https://github.com/juspay/hyperswitch/pull/2494)) ([`2609ef6`](https://github.com/juspay/hyperswitch/commit/2609ef6aeb17e1e89d8f98ff84a2c33b9704e6b2))
  - [Dlocal] Use connector_response_reference_id as reference to merchant ([#2446](https://github.com/juspay/hyperswitch/pull/2446)) ([`f6677b8`](https://github.com/juspay/hyperswitch/commit/f6677b8e9300a75810a39de5b60243e34cf1d76c))
- **nexinets:** Use connector_request_reference_id as reference to the connector - Work In Progress  ([#2515](https://github.com/juspay/hyperswitch/pull/2515)) ([`088dce0`](https://github.com/juspay/hyperswitch/commit/088dce076d8d8ff86769717368150e09d7d92593))
- **router:** Add Cancel Event in Webhooks and Mapping it in Stripe ([#2573](https://github.com/juspay/hyperswitch/pull/2573)) ([`92f7918`](https://github.com/juspay/hyperswitch/commit/92f7918e6f98460fb739d50b908ae33fda2f80b8))

### Refactors

- **connector:**
  - [Worldline] Currency Unit Conversion ([#2569](https://github.com/juspay/hyperswitch/pull/2569)) ([`9f03a41`](https://github.com/juspay/hyperswitch/commit/9f03a4118ccdd6036d27074c9126a79d6e9b0495))
  - [Authorizedotnet] Enhance currency Mapping with ConnectorCurrencyCommon Trait ([#2570](https://github.com/juspay/hyperswitch/pull/2570)) ([`d401975`](https://github.com/juspay/hyperswitch/commit/d4019751ff4acbd26abb2c32a600e8e6c55893f6))
  - [noon] enhance response status mapping ([#2575](https://github.com/juspay/hyperswitch/pull/2575)) ([`053c79d`](https://github.com/juspay/hyperswitch/commit/053c79d248df0ff6ec702c3c301acc5654a1735a))
- **storage:** Update paymentintent object to provide a relation with attempts ([#2502](https://github.com/juspay/hyperswitch/pull/2502)) ([`fbf3c03`](https://github.com/juspay/hyperswitch/commit/fbf3c03d418242b1f5f1a15c69029023d0b25b4e))

### Testing

- **postman:** Update postman collection files ([`08141ab`](https://github.com/juspay/hyperswitch/commit/08141abb3e87504bb4fe54fdfea92e6c889d729a))

**Full Changelog:** [`v1.57.1+hotfix.1...v1.58.0`](https://github.com/juspay/hyperswitch/compare/v1.57.1+hotfix.1...v1.58.0)

- - -


## 1.57.1 (2023-10-12)

### Bug Fixes

- **connector:** Trigger Psync after redirection url ([#2422](https://github.com/juspay/hyperswitch/pull/2422)) ([`8029a89`](https://github.com/juspay/hyperswitch/commit/8029a895b2c27a1ac14a19aea23bbc06cc364809))

**Full Changelog:** [`v1.57.0...v1.57.1`](https://github.com/juspay/hyperswitch/compare/v1.57.0...v1.57.1)

- - -


## 1.57.0 (2023-10-12)

### Features

- **connector:**
  - [Tsys] Use `connector_response_reference_id` as reference to the connector ([#2546](https://github.com/juspay/hyperswitch/pull/2546)) ([`550377a`](https://github.com/juspay/hyperswitch/commit/550377a6c3943d9fec4ca6a8be5a5f3aafe109ab))
  - [Cybersource] Use connector_request_reference_id as reference to the connector ([#2512](https://github.com/juspay/hyperswitch/pull/2512)) ([`81cb8da`](https://github.com/juspay/hyperswitch/commit/81cb8da4d47fe2a75330d39c665bb259faa35b00))
  - [Iatapay] use connector_response_reference_id as reference to connector ([#2524](https://github.com/juspay/hyperswitch/pull/2524)) ([`ef647b7`](https://github.com/juspay/hyperswitch/commit/ef647b7ab942707a06971b6545c81168f28cb94c))
  - [ACI] Use connector_request_reference_id as reference to the connector ([#2549](https://github.com/juspay/hyperswitch/pull/2549)) ([`c2ad200`](https://github.com/juspay/hyperswitch/commit/c2ad2002c0e6d673f62ec4c72c8fd98b07a05c0b))
- **customers:** Add customer list endpoint ([#2564](https://github.com/juspay/hyperswitch/pull/2564)) ([`c26620e`](https://github.com/juspay/hyperswitch/commit/c26620e041add914abc60c6149787be62ea5985d))
- **router:**
  - Add kv implementation for update address in update payments flow ([#2542](https://github.com/juspay/hyperswitch/pull/2542)) ([`9f446bc`](https://github.com/juspay/hyperswitch/commit/9f446bc1742c06a7fab3d92128ba4e7d3be80ea6))
  - Add payment link support ([#2105](https://github.com/juspay/hyperswitch/pull/2105)) ([`642085d`](https://github.com/juspay/hyperswitch/commit/642085dc745f87b4edd2f7a744c31b8979b23cfa))

### Bug Fixes

- **connector:**
  - [noon] sync with reference_id ([#2544](https://github.com/juspay/hyperswitch/pull/2544)) ([`9ef60e4`](https://github.com/juspay/hyperswitch/commit/9ef60e425d0cbe764ce66c65c8c09b1992cbe99f))
  - [braintree] add 3ds redirection error mapping and metadata validation ([#2552](https://github.com/juspay/hyperswitch/pull/2552)) ([`28d02f9`](https://github.com/juspay/hyperswitch/commit/28d02f94c6d52d05b6f520e4d48ba88adf7be619))
- **router:** Add customer_id validation for `payment method create` flow ([#2543](https://github.com/juspay/hyperswitch/pull/2543)) ([`53d7604`](https://github.com/juspay/hyperswitch/commit/53d760460305e16f03d86f699acb035151dfdfad))
- Percentage float inconsistency problem and api models changes to support surcharge feature ([#2550](https://github.com/juspay/hyperswitch/pull/2550)) ([`1ee1184`](https://github.com/juspay/hyperswitch/commit/1ee11849d4a60afbf3d05103cb491a11e905b811))
- Consume profile_id throughout payouts flow ([#2501](https://github.com/juspay/hyperswitch/pull/2501)) ([`7eabd24`](https://github.com/juspay/hyperswitch/commit/7eabd24a4da6f82fd30f8a4be739962538654214))
- Parse allowed_payment_method_types only if there is some value p… ([#2161](https://github.com/juspay/hyperswitch/pull/2161)) ([`46f1419`](https://github.com/juspay/hyperswitch/commit/46f14191ab7e036539ef3fd58acd9376b6b6b63c))

### Refactors

- **connector:**
  - [Worldpay] Currency Unit Conversion ([#2436](https://github.com/juspay/hyperswitch/pull/2436)) ([`b78109b`](https://github.com/juspay/hyperswitch/commit/b78109bc93433e0886b0b8656231899df84da8cf))
  - [noon] use connector_request_reference_id for sync ([#2558](https://github.com/juspay/hyperswitch/pull/2558)) ([`0889a6e`](https://github.com/juspay/hyperswitch/commit/0889a6ed0691abeed7bba44e7024545abcc74aef))
  - [noon] update and add recommended fields  ([#2381](https://github.com/juspay/hyperswitch/pull/2381)) ([`751f16e`](https://github.com/juspay/hyperswitch/commit/751f16eaee254ab8f0068e2e9e81e3e4b7fe133f))
- **worldline:** Use `connector_request_reference_id` as reference to the connector ([#2498](https://github.com/juspay/hyperswitch/pull/2498)) ([`efa5320`](https://github.com/juspay/hyperswitch/commit/efa53204e8ab1ef1192bcdc07ed99306475badbc))

### Revert

- Fix(connector): [noon] sync with reference_id ([#2556](https://github.com/juspay/hyperswitch/pull/2556)) ([`13be4d3`](https://github.com/juspay/hyperswitch/commit/13be4d36eac3d1e17d8ad9b3f3ef8993547f548b))

**Full Changelog:** [`v1.56.0...v1.57.0`](https://github.com/juspay/hyperswitch/compare/v1.56.0...v1.57.0)

- - -


## 1.56.0 (2023-10-11)

### Features

- **connector:**
  - [Volt] Template generation ([#2480](https://github.com/juspay/hyperswitch/pull/2480)) ([`ee321bb`](https://github.com/juspay/hyperswitch/commit/ee321bb82686559643d8c2725b0491997af717b2))
  - [NexiNets] Update connector_response_reference_id as reference to merchant ([#2537](https://github.com/juspay/hyperswitch/pull/2537)) ([`2f6c00a`](https://github.com/juspay/hyperswitch/commit/2f6c00a1fd853876333608a7d1fa6b488c3001d3))
  - [Authorizedotnet] use connector_response_reference_id as reference to merchant ([#2497](https://github.com/juspay/hyperswitch/pull/2497)) ([`62638c4`](https://github.com/juspay/hyperswitch/commit/62638c4230bfd149c43c2805cbad0ce9be5386b3))
- **router:** Change temp locker config as enable only ([#2522](https://github.com/juspay/hyperswitch/pull/2522)) ([`7acf101`](https://github.com/juspay/hyperswitch/commit/7acf10101435ab97d93490e19eaac5373d34f531))

### Refactors

- Delete requires cvv config when merchant account is deleted ([#2525](https://github.com/juspay/hyperswitch/pull/2525)) ([`b968552`](https://github.com/juspay/hyperswitch/commit/b9685521735956659c50bc2e1c15b08cb9952aee))

### Testing

- **postman:**
  - Add proper `customer_id` in payment method create api ([#2548](https://github.com/juspay/hyperswitch/pull/2548)) ([`7994a12`](https://github.com/juspay/hyperswitch/commit/7994a1259c5852ba4ebabb906bef963c6cf81bc9))
  - Update postman collection files ([`7c561d5`](https://github.com/juspay/hyperswitch/commit/7c561d57767001e755fc9abfc32352ffdc9aacea))

### Miscellaneous Tasks

- **CODEOWNERS:** Update CODEOWNERS ([#2541](https://github.com/juspay/hyperswitch/pull/2541)) ([`d9fb5d4`](https://github.com/juspay/hyperswitch/commit/d9fb5d4a52f44809ab4a1576a99e97b4c8b8c41b))

**Full Changelog:** [`v1.55.0...v1.56.0`](https://github.com/juspay/hyperswitch/compare/v1.55.0...v1.56.0)

- - -


## 1.55.0 (2023-10-10)

### Features

- **connector:**
  - [Multisafepay] Use connector_request_reference_id as reference to the connector ([#2503](https://github.com/juspay/hyperswitch/pull/2503)) ([`c34f1bf`](https://github.com/juspay/hyperswitch/commit/c34f1bf36ffb3a3533dd51ac87e7f66ab0dcce79))
  - [GlobalPayments] Introduce connector_request_reference_id for GlobalPayments ([#2519](https://github.com/juspay/hyperswitch/pull/2519)) ([`116139b`](https://github.com/juspay/hyperswitch/commit/116139ba7ae6878b7018068b0cb8303a8e8d1f7a))
  - [Airwallex] Use connector_request_reference_id as merchant reference id #2291 ([#2516](https://github.com/juspay/hyperswitch/pull/2516)) ([`6e89e41`](https://github.com/juspay/hyperswitch/commit/6e89e4103da4ecf6d7f06f7a9ec7da64eb493a6e))
- **trace:** Add optional sampling behaviour for routes ([#2511](https://github.com/juspay/hyperswitch/pull/2511)) ([`ec51e48`](https://github.com/juspay/hyperswitch/commit/ec51e48402da63e1250328485095b8665d7eca65))
- Gracefully shutdown drainer if redis goes down ([#2391](https://github.com/juspay/hyperswitch/pull/2391)) ([`2870af1`](https://github.com/juspay/hyperswitch/commit/2870af1286e897be0d40c014bc5742eafc6795db))
- Kv for reverse lookup ([#2445](https://github.com/juspay/hyperswitch/pull/2445)) ([`13aaf96`](https://github.com/juspay/hyperswitch/commit/13aaf96db0f62dc7a706ba2ba230912ee7ef7a68))
- Add x-hs-latency header for application overhead measurement ([#2486](https://github.com/juspay/hyperswitch/pull/2486)) ([`cf0db35`](https://github.com/juspay/hyperswitch/commit/cf0db35923d39caca9bf267b7d87a3f215884b66))

### Bug Fixes

- **connector:**
  - [Airwallex] convert expiry year to four digit ([#2527](https://github.com/juspay/hyperswitch/pull/2527)) ([`4b0fa12`](https://github.com/juspay/hyperswitch/commit/4b0fa1295ca8f4e611b65fbf2458c38b89303d3b))
  - [noon] add missing response status ([#2528](https://github.com/juspay/hyperswitch/pull/2528)) ([`808ee45`](https://github.com/juspay/hyperswitch/commit/808ee45556f90b1c1360a3edbffe9ba3603439d4))

### Refactors

- **payment_methods:** Added mca_id in bank details ([#2495](https://github.com/juspay/hyperswitch/pull/2495)) ([`ac3c500`](https://github.com/juspay/hyperswitch/commit/ac3c5008f80172a575f2fb08b7a5e78016ce7595))
- **test_utils:** Refactor `test_utils` crate and add `folder` support with updated documentation ([#2487](https://github.com/juspay/hyperswitch/pull/2487)) ([`6b52ac3`](https://github.com/juspay/hyperswitch/commit/6b52ac3d398d5a180c1dc67c5b53702ad01a0773))

### Miscellaneous Tasks

- [GOCARDLESS] env changes for becs and sepa mandates ([#2535](https://github.com/juspay/hyperswitch/pull/2535)) ([`4f5a383`](https://github.com/juspay/hyperswitch/commit/4f5a383bab567a1b46b2d6990c0c23ed60f1201b))

**Full Changelog:** [`v1.54.0...v1.55.0`](https://github.com/juspay/hyperswitch/compare/v1.54.0...v1.55.0)

- - -


## 1.54.0 (2023-10-09)

### Features

- **connector:**
  - [Fiserv] update connector_response_reference_id in transformers ([#2489](https://github.com/juspay/hyperswitch/pull/2489)) ([`4eb7003`](https://github.com/juspay/hyperswitch/commit/4eb70034336e5ff42c9eea912d940ea04cae9326))
  - [Nuvei] Use "connector_request_reference_id" for as "attempt_id" to improve consistency in transmitting payment information ([#2493](https://github.com/juspay/hyperswitch/pull/2493)) ([`17393f5`](https://github.com/juspay/hyperswitch/commit/17393f5be3e9027fedf9466c6401754f3c4d6b99))
- **kv:** Add kv wrapper for executing kv tasks ([#2384](https://github.com/juspay/hyperswitch/pull/2384)) ([`8b50997`](https://github.com/juspay/hyperswitch/commit/8b50997e56307507be101c562aa70d0a9b429137))
- **process_tracker:** Make long standing payments failed ([#2380](https://github.com/juspay/hyperswitch/pull/2380)) ([`73dfc31`](https://github.com/juspay/hyperswitch/commit/73dfc31f9d16d2cf71de8433fb630bea941a7020))

### Bug Fixes

- Add release feature to drianer ([#2507](https://github.com/juspay/hyperswitch/pull/2507)) ([`224b83c`](https://github.com/juspay/hyperswitch/commit/224b83c51d53fb1ca9ae11ff2f60b7b6cc807fc8))

### Refactors

- Disable color in reports in json format ([#2509](https://github.com/juspay/hyperswitch/pull/2509)) ([`aa176c7`](https://github.com/juspay/hyperswitch/commit/aa176c7c5d79f68c8bd97a3248fd4d40e937a3ce))

### Miscellaneous Tasks

- Address Rust 1.73 clippy lints ([#2474](https://github.com/juspay/hyperswitch/pull/2474)) ([`e02838e`](https://github.com/juspay/hyperswitch/commit/e02838eb5d3da97ef573926ded4a318ed24b6f1c))

**Full Changelog:** [`v1.53.0...v1.54.0`](https://github.com/juspay/hyperswitch/compare/v1.53.0...v1.54.0)

- - -


## 1.53.0 (2023-10-09)

### Features

- **connector:**
  - [Braintree] implement dispute webhook  ([#2031](https://github.com/juspay/hyperswitch/pull/2031)) ([`eeccd10`](https://github.com/juspay/hyperswitch/commit/eeccd106ae569bd60011ed71495d7978998161f8))
  - [Paypal] Implement 3DS for Cards ([#2443](https://github.com/juspay/hyperswitch/pull/2443)) ([`d95a64d`](https://github.com/juspay/hyperswitch/commit/d95a64d6c9b870bdc38aa091cf9bf660b1ea404e))
  - [Cybersource] Use connector_response_reference_id as reference to merchant  ([#2470](https://github.com/juspay/hyperswitch/pull/2470)) ([`a2dfc48`](https://github.com/juspay/hyperswitch/commit/a2dfc48318363db051f311ee7f911de0db0eb868))
  - [Coinbase] Add order id as the reference id  ([#2469](https://github.com/juspay/hyperswitch/pull/2469)) ([`9c2fff5`](https://github.com/juspay/hyperswitch/commit/9c2fff5ab44cdd4f285b6d1437f37869b517963e))
  - [Multisafepay] Use transaction_id as reference to transaction ([#2451](https://github.com/juspay/hyperswitch/pull/2451)) ([`ba2efac`](https://github.com/juspay/hyperswitch/commit/ba2efac4fa2af22f81b0841350a334bc36e91022))

### Bug Fixes

- Add startup config log to drainer ([#2482](https://github.com/juspay/hyperswitch/pull/2482)) ([`5038234`](https://github.com/juspay/hyperswitch/commit/503823408b782968fb59f6ff5d7df417b9aa7dbe))
- Fetch data directly from DB in OLAP functions ([#2475](https://github.com/juspay/hyperswitch/pull/2475)) ([`12b5341`](https://github.com/juspay/hyperswitch/commit/12b534197276ccc4aa9575e6b518bcc50b597bee))

### Refactors

- **connector:** [trustpay] refactor trustpay and handled variants errors ([#2484](https://github.com/juspay/hyperswitch/pull/2484)) ([`3f1e7c2`](https://github.com/juspay/hyperswitch/commit/3f1e7c2152a839a6fe69f60b906277ca831e7611))
- **merchant_account:** Make `organization_id` as mandatory ([#2458](https://github.com/juspay/hyperswitch/pull/2458)) ([`53b4816`](https://github.com/juspay/hyperswitch/commit/53b4816d27fe7794cb482887ed17ddb4386bd2f7))

### Miscellaneous Tasks

- Env changes for gocardless mandate ([#2485](https://github.com/juspay/hyperswitch/pull/2485)) ([`65ca5f1`](https://github.com/juspay/hyperswitch/commit/65ca5f12da54715e5db785d122e2ec9714147c68))

**Full Changelog:** [`v1.52.0...v1.53.0`](https://github.com/juspay/hyperswitch/compare/v1.52.0...v1.53.0)

- - -


## 1.52.0 (2023-10-06)

### Features

- **connector:**
  - [Forte] Use connector_response_reference_id as reference to merchant ([#2456](https://github.com/juspay/hyperswitch/pull/2456)) ([`cc7e90f`](https://github.com/juspay/hyperswitch/commit/cc7e90f2293f27b74b14669a0c2d5bd6d45c4d99))
  - [PayU] Use connector_response_response_id as reference to merchant ([#2452](https://github.com/juspay/hyperswitch/pull/2452)) ([`e24897c`](https://github.com/juspay/hyperswitch/commit/e24897cd5d3859124636760a4eb42ee007f00c3e))
  - [Gocardless] Implement mandate flow ([#2461](https://github.com/juspay/hyperswitch/pull/2461)) ([`4149965`](https://github.com/juspay/hyperswitch/commit/414996592b3016bfa9f3399319c6e02ccd333c68))
  - [Gocardless] Add mandate webhoooks ([#2468](https://github.com/juspay/hyperswitch/pull/2468)) ([`8d53c66`](https://github.com/juspay/hyperswitch/commit/8d53c663a5e25817d1facda3352f84f1435efdee))
  - [Noon] Use connector_request_reference_id as Order reference ([#2466](https://github.com/juspay/hyperswitch/pull/2466)) ([`2897b6e`](https://github.com/juspay/hyperswitch/commit/2897b6ecd1a357bae93ca22fe9e7aeed18738b95))
- **core:** Add surcharge_details field to ResponsePaymentMethodTypes struct ([#2435](https://github.com/juspay/hyperswitch/pull/2435)) ([`3f0d927`](https://github.com/juspay/hyperswitch/commit/3f0d927cb8db503c4dede98c691c1b7e6ebd441a))
- **router:** Add mandates incoming webhooks flow ([#2464](https://github.com/juspay/hyperswitch/pull/2464)) ([`1cf8b6c`](https://github.com/juspay/hyperswitch/commit/1cf8b6cf53ee5fdde9a7a3996e5a5e5c5b8341c6))

### Bug Fixes

- Update connector_mandate_id column in generate mandate flow ([#2472](https://github.com/juspay/hyperswitch/pull/2472)) ([`61288d5`](https://github.com/juspay/hyperswitch/commit/61288d541f654bcb102465e4da9883aaaac43f5b))

### Refactors

- **connector:** [nmi] refactor nmi and handled variants errors ([#2463](https://github.com/juspay/hyperswitch/pull/2463)) ([`f364a06`](https://github.com/juspay/hyperswitch/commit/f364a069b90dd63a28cf25457b2cd4fda0829a8b))
- Add support for passing context generic to api calls ([#2433](https://github.com/juspay/hyperswitch/pull/2433)) ([`601c174`](https://github.com/juspay/hyperswitch/commit/601c1744b6f15eb14ecfa3edede3159c32c53492))

**Full Changelog:** [`v1.51.1...v1.52.0`](https://github.com/juspay/hyperswitch/compare/v1.51.1...v1.52.0)

- - -


## 1.51.1 (2023-10-05)

### Bug Fixes

- **router:** Make payment type optional in payments request ([#2465](https://github.com/juspay/hyperswitch/pull/2465)) ([`b5cc748`](https://github.com/juspay/hyperswitch/commit/b5cc7483f99dcd995b9022d21c94f2f9710ea7fe))

### Refactors

- **router:**
  - Renamed Verify flow to SetupMandate ([#2455](https://github.com/juspay/hyperswitch/pull/2455)) ([`80f3b1e`](https://github.com/juspay/hyperswitch/commit/80f3b1edaeae9a13ea291a0315f1be2686336914))
  - Remove the payment type column in payment intent ([#2462](https://github.com/juspay/hyperswitch/pull/2462)) ([`980aa44`](https://github.com/juspay/hyperswitch/commit/980aa448634de86f11fb67aabefc15884f1b8ced))

### Miscellaneous Tasks

- Fix the failing formatting check for external contributors ([#2467](https://github.com/juspay/hyperswitch/pull/2467)) ([`bb2ba08`](https://github.com/juspay/hyperswitch/commit/bb2ba0815330578295de8036ea1a5e6d66a36277))

**Full Changelog:** [`v1.51.0...v1.51.1`](https://github.com/juspay/hyperswitch/compare/v1.51.0...v1.51.1)

- - -


## 1.51.0 (2023-10-05)

### Features

- **connector:**
  - [Noon] Use connector_response_reference_id as reference ([#2442](https://github.com/juspay/hyperswitch/pull/2442)) ([`688557e`](https://github.com/juspay/hyperswitch/commit/688557ef95826622fe87a4de1dfbc09446496686))
  - [Opayo] Add connector id ([#2418](https://github.com/juspay/hyperswitch/pull/2418)) ([`8e51073`](https://github.com/juspay/hyperswitch/commit/8e51073c837909838b92a9eadea32e5a577e2b54))
- **payment_methods:** Bank details support for payment method data in pmt ([#2385](https://github.com/juspay/hyperswitch/pull/2385)) ([`e86c032`](https://github.com/juspay/hyperswitch/commit/e86c0325f51d06ecfcbc810f3320c97850716825))
- **router:** Add support for payment_type field in payment intent ([#2448](https://github.com/juspay/hyperswitch/pull/2448)) ([`f116728`](https://github.com/juspay/hyperswitch/commit/f116728d1cba458a1e184c2fdf5a1cc012430c35))

### Bug Fixes

- **connector:** Use enum to deserialize latest_charge in stripe psync response ([#2444](https://github.com/juspay/hyperswitch/pull/2444)) ([`05ee47a`](https://github.com/juspay/hyperswitch/commit/05ee47a6e90bd68a0faa6dcc381c48a1f0f274d8))
- **payments:** Move validations of payment intent before attempt ([#2440](https://github.com/juspay/hyperswitch/pull/2440)) ([`7fb5c04`](https://github.com/juspay/hyperswitch/commit/7fb5c044bc9611e375b271065859308792773f30))
- Return appropriate error message during webhook call for invalid merchant_secret adyen ([#2450](https://github.com/juspay/hyperswitch/pull/2450)) ([`db7f9fa`](https://github.com/juspay/hyperswitch/commit/db7f9fa801d2bacf3abda6cc447220d254f56382))

### Testing

- **postman:** Update postman collection files ([`a9221d4`](https://github.com/juspay/hyperswitch/commit/a9221d44d192a3baadf9978d517e8153ef7a739a))

**Full Changelog:** [`v1.50.0...v1.51.0`](https://github.com/juspay/hyperswitch/compare/v1.50.0...v1.51.0)

- - -


## 1.50.0 (2023-10-04)

### Features

- **connector:**
  - [Stax] Use connector_response_reference_id as reference to merchant ([#2415](https://github.com/juspay/hyperswitch/pull/2415)) ([`099b241`](https://github.com/juspay/hyperswitch/commit/099b241096c69879a805ca81b1c5a23118e10b52))
  - [PowerTranz] Use connector_response_reference_id as reference to merchant ([#2413](https://github.com/juspay/hyperswitch/pull/2413)) ([`0d703c7`](https://github.com/juspay/hyperswitch/commit/0d703c7ab85c68f433767a70a0feabe8daa4f24c))
  - [Payeezy] Use connector_response_reference_id as reference to merchant ([#2410](https://github.com/juspay/hyperswitch/pull/2410)) ([`485c09d`](https://github.com/juspay/hyperswitch/commit/485c09d16743d73b446d6313f0ee6462c8a77028))
  - [Square] Use reference_id as reference to merchant ([#2434](https://github.com/juspay/hyperswitch/pull/2434)) ([`591c9b7`](https://github.com/juspay/hyperswitch/commit/591c9b70d9b7f8df5d7f5d2cb2d19cfaa1457fe1))
- **router:**
  - Remove unnecessary lookups in refund and payment_attempt kv flow ([#2425](https://github.com/juspay/hyperswitch/pull/2425)) ([`f720aec`](https://github.com/juspay/hyperswitch/commit/f720aecf1fb676cec71e636b877a46f9791d713a))
  - [OpenNode] response reference id ([#2416](https://github.com/juspay/hyperswitch/pull/2416)) ([`3bfea72`](https://github.com/juspay/hyperswitch/commit/3bfea72df34f4ce0ffdb61e49960fdf09b96eb5a))
  - Add profile id and extra filters in lists ([#2379](https://github.com/juspay/hyperswitch/pull/2379)) ([`ab2cde7`](https://github.com/juspay/hyperswitch/commit/ab2cde799371a66eb045cf8b20431b3b108dac44))

### Bug Fixes

- **CI:** Fix spell check for CI pull request ([#2439](https://github.com/juspay/hyperswitch/pull/2439)) ([`04f2e11`](https://github.com/juspay/hyperswitch/commit/04f2e11cd4f3fd327408cddec36ccf4fb486b935))
- **router:** Merchant account delete does not delete the merchant_key_store ([#2367](https://github.com/juspay/hyperswitch/pull/2367)) ([`35f7ce0`](https://github.com/juspay/hyperswitch/commit/35f7ce0f4d9e16184e2bb94360d3ced60f8b5af2))

### Refactors

- **config:** Update payment method filter for apple pay ([#2423](https://github.com/juspay/hyperswitch/pull/2423)) ([`d177b4d`](https://github.com/juspay/hyperswitch/commit/d177b4d94f08fb8ef44b5c07ec1bdc771baa016d))
- **payment_methods:** Add `requires_cvv` config while creating merchant account ([#2431](https://github.com/juspay/hyperswitch/pull/2431)) ([`6e5ab0d`](https://github.com/juspay/hyperswitch/commit/6e5ab0d121d6345f18bccc7f917064caa2737475))
- **webhook:** Add a function to retrieve payment_id ([#2447](https://github.com/juspay/hyperswitch/pull/2447)) ([`409913f`](https://github.com/juspay/hyperswitch/commit/409913fd75076e4ee1dac1e4dc5b2f164528bc23))

### Build System / Dependencies

- **deps:** Address `undeclared crate or module` errors on Windows for `scheduler` crate ([#2411](https://github.com/juspay/hyperswitch/pull/2411)) ([`4225238`](https://github.com/juspay/hyperswitch/commit/422523848e6516643a6beef1ba15af4e967f0c5b))

**Full Changelog:** [`v1.49.0...v1.50.0`](https://github.com/juspay/hyperswitch/compare/v1.49.0...v1.50.0)

- - -


## 1.49.0 (2023-10-03)

### Features

- **connector:** [Nuvei] Add order id as the reference id ([#2408](https://github.com/juspay/hyperswitch/pull/2408)) ([`d5d876b`](https://github.com/juspay/hyperswitch/commit/d5d876b821187648994ea53c358467966e99cd23))
- **pm_auth:** Added pm_auth_config to merchant_connector_account ([#2183](https://github.com/juspay/hyperswitch/pull/2183)) ([`abfdea2`](https://github.com/juspay/hyperswitch/commit/abfdea20b06a8804ec83fe9431f9a034465bb924))
- **pm_list:** [Trustpay] add bank_redirect -  blik pm type required field info for trustpay ([#2390](https://github.com/juspay/hyperswitch/pull/2390)) ([`d81762a`](https://github.com/juspay/hyperswitch/commit/d81762a8b430ca1f197d7dabb26167f54e235735))
- **webhooks:** Webhooks effect tracker ([#2260](https://github.com/juspay/hyperswitch/pull/2260)) ([`5048d24`](https://github.com/juspay/hyperswitch/commit/5048d248e59b8ecaf8585ffd5134953cf62e74ef))

### Bug Fixes

- **CI:** Fix spell check for CI pull request  ([#2420](https://github.com/juspay/hyperswitch/pull/2420)) ([`3b10b1c`](https://github.com/juspay/hyperswitch/commit/3b10b1c473209e36183271a81eb9014a8f5cddfa))
- **cards:** Allow card cvc 000 ([#2387](https://github.com/juspay/hyperswitch/pull/2387)) ([`f0dc374`](https://github.com/juspay/hyperswitch/commit/f0dc37438b7a6c4b25acff941aca13545217d307))
- **configs:** Add `lock_settings` in `docker_compose.toml` ([#2396](https://github.com/juspay/hyperswitch/pull/2396)) ([`14fec5c`](https://github.com/juspay/hyperswitch/commit/14fec5c3980397079fe8861caca589157a8ba242))
- **connector:** [noon] add connector_auth params and update description ([#2429](https://github.com/juspay/hyperswitch/pull/2429)) ([`0aa6b30`](https://github.com/juspay/hyperswitch/commit/0aa6b30d2c9056e9a21a88bdc064daa7e8659bd6))
- **payment_methods:** prioritized `apple_pay_combined` deserialization over `apple_pay` ([#2393](https://github.com/juspay/hyperswitch/pull/2393)) ([`f12ce9c`](https://github.com/juspay/hyperswitch/commit/f12ce9c72d94674e0ae0ec7f1c91d8b5c43481e8))
- Temp support for ach gocardless with existing api contracts ([#2395](https://github.com/juspay/hyperswitch/pull/2395)) ([`d43fbcc`](https://github.com/juspay/hyperswitch/commit/d43fbccd54011d0de6f8d39adbd264d9ada77e7e))

### Refactors

- **connector:**
  - [Klarna] Expand wildcard match arms ([#2403](https://github.com/juspay/hyperswitch/pull/2403)) ([`89cb63b`](https://github.com/juspay/hyperswitch/commit/89cb63be3328010d26b5f6322449fc50e80593e4))
  - [Klarna] Enhance currency Mapping with ConnectorCurrencyCommon Trait ([#2414](https://github.com/juspay/hyperswitch/pull/2414)) ([`ee7efd0`](https://github.com/juspay/hyperswitch/commit/ee7efd05adbe14bab1d2862d7ab2bf244c226433))
  - [Cryptopay] Update PSync with connector_request_reference_id  ([#2388](https://github.com/juspay/hyperswitch/pull/2388)) ([`3680541`](https://github.com/juspay/hyperswitch/commit/36805411772da00719a716d05c650f10ca990d49))
- **router:** Add `#[cfg(not(feature = "kms"))]` feature flag to test the simplified apple pay flow locally ([#2200](https://github.com/juspay/hyperswitch/pull/2200)) ([`e5ad9c5`](https://github.com/juspay/hyperswitch/commit/e5ad9c5c35f386486afedded90c46793196a17d0))

### Testing

- **postman:** Update postman collection files ([`34099ba`](https://github.com/juspay/hyperswitch/commit/34099baa2ec2f73598c4433b0a481dec3fde8c05))

### Documentation

- **README:**
  - Include Hacktoberfest information ([#2386](https://github.com/juspay/hyperswitch/pull/2386)) ([`e8eb929`](https://github.com/juspay/hyperswitch/commit/e8eb929d5b4d99d09940532e3abbca2b811bcf36))
  - Fixed TOC links ([#2402](https://github.com/juspay/hyperswitch/pull/2402)) ([`c81d8e9`](https://github.com/juspay/hyperswitch/commit/c81d8e9a180da8f71d156d39c9f85847f6d7a572))

### Miscellaneous Tasks

- **deps:** Bump webpki from 0.22.0 to 0.22.2 ([#2419](https://github.com/juspay/hyperswitch/pull/2419)) ([`6bf0e75`](https://github.com/juspay/hyperswitch/commit/6bf0e75b69608ea07fd7601906982a19cdc81400))

**Full Changelog:** [`v1.48.1+hotfix.1...v1.49.0`](https://github.com/juspay/hyperswitch/compare/v1.48.1+hotfix.1...v1.49.0)

- - -


## 1.48.1 (2023-09-28)

### Bug Fixes

- [Gocardless] add region in customer create request based on country ([#2389](https://github.com/juspay/hyperswitch/pull/2389)) ([`c293cb6`](https://github.com/juspay/hyperswitch/commit/c293cb6ffafd61702ee16233cf06a206c0093f3d))

**Full Changelog:** [`v1.48.0...v1.48.1`](https://github.com/juspay/hyperswitch/compare/v1.48.0...v1.48.1)

- - -


## 1.48.0 (2023-09-27)

### Features

- **core:** Create surcharge_metadata field in payment attempt ([#2371](https://github.com/juspay/hyperswitch/pull/2371)) ([`934542e`](https://github.com/juspay/hyperswitch/commit/934542e92625620d71b940e99d4ae58239a60ce4))
- **router:**
  - Append payment_id to secondary key for payment_intent in kv flow ([#2378](https://github.com/juspay/hyperswitch/pull/2378)) ([`ee91552`](https://github.com/juspay/hyperswitch/commit/ee9155208d6c0a3d5d5422b469bfa7a80671cd86))
  - Pass customers address in retrieve customer ([#2376](https://github.com/juspay/hyperswitch/pull/2376)) ([`f6cfb05`](https://github.com/juspay/hyperswitch/commit/f6cfb05fa042b5f68a5cb6fa17090d2beb91303b))

### Bug Fixes

- **db:** Merchant_account cache invalidation based on publishable_key ([#2365](https://github.com/juspay/hyperswitch/pull/2365)) ([`22a8291`](https://github.com/juspay/hyperswitch/commit/22a8291ea66bc564218af0a4a2695eef70ce6790))
- **router:** Allow address updates in payments update flow ([#2375](https://github.com/juspay/hyperswitch/pull/2375)) ([`0d3dd00`](https://github.com/juspay/hyperswitch/commit/0d3dd0033c5ec9eabc967cb1872f0699546aba89))

### Refactors

- **connector:**
  - [Payme]Enhance currency Mapping with ConnectorCurrencyCommon Trait  ([#2194](https://github.com/juspay/hyperswitch/pull/2194)) ([`77b51d5`](https://github.com/juspay/hyperswitch/commit/77b51d5cbe531526f2f20a0ee4a78e95b00d87de))
  - [bluesnap] add refund status and webhooks ([#2374](https://github.com/juspay/hyperswitch/pull/2374)) ([`fe43458`](https://github.com/juspay/hyperswitch/commit/fe43458ddc0fa1cc31f2b326056baea54af57136))
- Insert requires cvv config to configs table if not found in db ([#2208](https://github.com/juspay/hyperswitch/pull/2208)) ([`68b3310`](https://github.com/juspay/hyperswitch/commit/68b3310993c5196f9f9038f27c5cd7dad82b24d1))

**Full Changelog:** [`v1.47.0...v1.48.0`](https://github.com/juspay/hyperswitch/compare/v1.47.0...v1.48.0)

- - -


## 1.47.0 (2023-09-27)

### Features

- **connector_response:** Kv for connector response table ([#2207](https://github.com/juspay/hyperswitch/pull/2207)) ([`cefa291`](https://github.com/juspay/hyperswitch/commit/cefa291c00c7d4a40213cc6c6087946c031ae0b5))

### Bug Fixes

- **connector:**
  - Make webhook source verification mandatory for adyen ([#2360](https://github.com/juspay/hyperswitch/pull/2360)) ([`3d7e22a`](https://github.com/juspay/hyperswitch/commit/3d7e22a4f106e4d7c4224fecf455e2f2aa417cd0))
  - [noon] Create psync struct from webhook resource object ([#2370](https://github.com/juspay/hyperswitch/pull/2370)) ([`f12a438`](https://github.com/juspay/hyperswitch/commit/f12a43817787faedfdca26ec7f956bf5734c5ee3))
- **merchant_connector_account:** Use appropriate key when redacting ([#2363](https://github.com/juspay/hyperswitch/pull/2363)) ([`54645cd`](https://github.com/juspay/hyperswitch/commit/54645cdbf422d59b8751fa9dbb9a61cd72770b0a))
- **router:**
  - Fix refunds and payment_attempts kv flow ([#2362](https://github.com/juspay/hyperswitch/pull/2362)) ([`ef0df71`](https://github.com/juspay/hyperswitch/commit/ef0df7195d9a7c7cd384f6df9eb5a8b886914e2d))
  - Removed dynamic error messages ([#2168](https://github.com/juspay/hyperswitch/pull/2168)) ([`9c9d453`](https://github.com/juspay/hyperswitch/commit/9c9d45353596edb5dc5c19e1a6d8d42d05bae78c))
- [stripe] Add customer balance in StripePaymentMethodDetailsResponse ([#2369](https://github.com/juspay/hyperswitch/pull/2369)) ([`67a3e8f`](https://github.com/juspay/hyperswitch/commit/67a3e8f534aa98a7331cb20a3877579efed6a348))

### Refactors

- **connector:**
  - [bluesnap]Enhance currency Mapping with ConnectorCurrencyCommon Trait ([#2193](https://github.com/juspay/hyperswitch/pull/2193)) ([`6db60b8`](https://github.com/juspay/hyperswitch/commit/6db60b8cd4319d0246c72494fa65082108ffd06e))
  - [Zen] Enhance currency Mapping with ConnectorCurrencyCommon Trait ([#2196](https://github.com/juspay/hyperswitch/pull/2196)) ([`7fd79e0`](https://github.com/juspay/hyperswitch/commit/7fd79e05d54e6f135fbd4151d6638060660e6c85))
  - [Paypal]Enhance currency Mapping with ConnectorCurrencyCommon Trait  ([#2191](https://github.com/juspay/hyperswitch/pull/2191)) ([`2e97869`](https://github.com/juspay/hyperswitch/commit/2e97869fa0e284e1ab3bcaf940b627acf47d98e3))
  - [Cryptopay]Enhance currency Mapping with ConnectorCurrencyCommon Trait ([#2195](https://github.com/juspay/hyperswitch/pull/2195)) ([`d8c3845`](https://github.com/juspay/hyperswitch/commit/d8c384573e1f31ed4c8fd252b8d753a04a4df75d))

### Miscellaneous Tasks

- **config:** [Multisafepay] Add configs for card mandates for Multisafepay ([#2372](https://github.com/juspay/hyperswitch/pull/2372)) ([`af3b9e9`](https://github.com/juspay/hyperswitch/commit/af3b9e90dbc733b436f84e47ebd62ef0b467c39c))

**Full Changelog:** [`v1.46.0...v1.47.0`](https://github.com/juspay/hyperswitch/compare/v1.46.0...v1.47.0)

- - -


## 1.46.0 (2023-09-25)

### Features

- **payment_attempt:** Add kv for find last successful attempt ([#2206](https://github.com/juspay/hyperswitch/pull/2206)) ([`d3157f0`](https://github.com/juspay/hyperswitch/commit/d3157f0bd6a0246c28182c88335d95ed6ae298a9))
- **payments:** Add api locking for payments core ([#1898](https://github.com/juspay/hyperswitch/pull/1898)) ([`5d66156`](https://github.com/juspay/hyperswitch/commit/5d661561322a21f792e2cdb2ae8c30de96ce7d02))

### Bug Fixes

- **compatibility:** Update BillingDetails mappings in SCL ([#1926](https://github.com/juspay/hyperswitch/pull/1926)) ([`a48f986`](https://github.com/juspay/hyperswitch/commit/a48f9865bcd29d5c3fc5c380dde34b11c6bb254f))
- **connector:** [stripe] use display impl for expiry date  ([#2359](https://github.com/juspay/hyperswitch/pull/2359)) ([`35622af`](https://github.com/juspay/hyperswitch/commit/35622aff7a042764729565db1ed5aca2257603ba))
- **drainer:** Ignore errors in case the stream is empty ([#2261](https://github.com/juspay/hyperswitch/pull/2261)) ([`53de86f`](https://github.com/juspay/hyperswitch/commit/53de86f60d14981087626e1a2a5856089b6f3899))
- Add health metric to drainer ([#2217](https://github.com/juspay/hyperswitch/pull/2217)) ([`4e8471b`](https://github.com/juspay/hyperswitch/commit/4e8471be501806ceeb96c7683be00600c3c1a0d2))

### Refactors

- Enable `logs` feature flag in router crate ([#2358](https://github.com/juspay/hyperswitch/pull/2358)) ([`e4af381`](https://github.com/juspay/hyperswitch/commit/e4af3812d55689aefb5bb8ed6f12a6c9c0643a51))

### Testing

- **postman:** Update postman collection files ([`d7affab`](https://github.com/juspay/hyperswitch/commit/d7affab455adf1eeccaca3005797a81e51c902ac))

**Full Changelog:** [`v1.45.0...v1.46.0`](https://github.com/juspay/hyperswitch/compare/v1.45.0...v1.46.0)

- - -


## 1.45.0 (2023-09-22)

### Features

- **router:** Add mertics to apple pay flow ([#2235](https://github.com/juspay/hyperswitch/pull/2235)) ([`b9f25c4`](https://github.com/juspay/hyperswitch/commit/b9f25c4a4ee540fe13257df193f9f921233156a6))

### Bug Fixes

- **router:** Fix attempt status for technical failures in psync flow ([#2252](https://github.com/juspay/hyperswitch/pull/2252)) ([`2b8bd03`](https://github.com/juspay/hyperswitch/commit/2b8bd03a7243c887c17be658f1d9e9faa462b0c7))

### Refactors

- **connector:**
  - [Checkout]Enhance currency Mapping with ConnectorCurrencyCommon Trait ([#2192](https://github.com/juspay/hyperswitch/pull/2192)) ([`aa8d0dd`](https://github.com/juspay/hyperswitch/commit/aa8d0ddda17adb7c87cea9ff5fbf83b8d0e7fde1))
  - [Trustpay] Enhance currency Mapping with ConnectorCurrencyCommon Trait ([#2197](https://github.com/juspay/hyperswitch/pull/2197)) ([`583b9aa`](https://github.com/juspay/hyperswitch/commit/583b9aa33b15f09cf8ea61b4e6dee002fb562e03))
- **core:** Eliminate business profile database queries in payments confirm flow ([#2190](https://github.com/juspay/hyperswitch/pull/2190)) ([`90e4392`](https://github.com/juspay/hyperswitch/commit/90e43929a0c05e39feac4f13d75b2eea60b858a0))

**Full Changelog:** [`v1.44.0...v1.45.0`](https://github.com/juspay/hyperswitch/compare/v1.44.0...v1.45.0)

- - -


## 1.44.0 (2023-09-22)

### Features

- **connector:** [Trustpay] Add descriptor for card payment method for trustpay ([#2256](https://github.com/juspay/hyperswitch/pull/2256)) ([`b9ddc4f`](https://github.com/juspay/hyperswitch/commit/b9ddc4fb69396a2ced73bc24e3d947eb8c4e091a))
- **db:** Add find_config_by_key_unwrap_or ([#2214](https://github.com/juspay/hyperswitch/pull/2214)) ([`2bd2526`](https://github.com/juspay/hyperswitch/commit/2bd25261b43b8b89ff2042e944ffa6008cc77c8f))

### Bug Fixes

- **connector:** Fix dispute webhook failure bug in checkout during get_webhook_resource_object ([#2257](https://github.com/juspay/hyperswitch/pull/2257)) ([`1d73be0`](https://github.com/juspay/hyperswitch/commit/1d73be08fb3a747ab22ee42eed9f396d78a949dd))

### Refactors

- **connector:**
  - [Stripe] refactor stripe payment method not implemented errors ([#1927](https://github.com/juspay/hyperswitch/pull/1927)) ([`417f793`](https://github.com/juspay/hyperswitch/commit/417f793284a11218fc520319ed717759f60e3934))
  - [Adyen] Enhance currency Mapping with ConnectorCurrencyCommon Trait  ([#2209](https://github.com/juspay/hyperswitch/pull/2209)) ([`3d18f20`](https://github.com/juspay/hyperswitch/commit/3d18f2062e5d7c14fc5725547eeaf80d7b2a86da))

### Miscellaneous Tasks

- **CODEOWNERS:** Update CODEOWNERS ([#2254](https://github.com/juspay/hyperswitch/pull/2254)) ([`7af4c92`](https://github.com/juspay/hyperswitch/commit/7af4c92ef25b8e2b71a6839fcd80925c09897779))
- **deps:** Bump phonenumber from 0.3.2+8.13.9 to 0.3.3+8.13.9 ([#2255](https://github.com/juspay/hyperswitch/pull/2255)) ([`8f3721d`](https://github.com/juspay/hyperswitch/commit/8f3721d16b27962923bff0968f7074cef2471e36))

**Full Changelog:** [`v1.43.1...v1.44.0`](https://github.com/juspay/hyperswitch/compare/v1.43.1...v1.44.0)

- - -


## 1.43.1 (2023-09-21)

### Bug Fixes

- Add flow_name setter ([#2234](https://github.com/juspay/hyperswitch/pull/2234)) ([`30e2c90`](https://github.com/juspay/hyperswitch/commit/30e2c906724a610ec5072e3a103eb3ce21a5ef0e))

**Full Changelog:** [`v1.43.0...v1.43.1`](https://github.com/juspay/hyperswitch/compare/v1.43.0...v1.43.1)

- - -


## 1.43.0 (2023-09-21)

### Features

- **connector:** [Gocardless] add support for Ach, Sepa, Becs payment methods ([#2180](https://github.com/juspay/hyperswitch/pull/2180)) ([`3efce90`](https://github.com/juspay/hyperswitch/commit/3efce9013d0572be9162216f134830ccf7e04905))
- **core:** Add support for webhook additional source verification call for paypal ([#2058](https://github.com/juspay/hyperswitch/pull/2058)) ([`2a9e09d`](https://github.com/juspay/hyperswitch/commit/2a9e09d812ca11960cabab289b32be162bc5cfc9))
- **db:** Enable caching for merchant_account fetch using publishable key ([#2186](https://github.com/juspay/hyperswitch/pull/2186)) ([`eb10aca`](https://github.com/juspay/hyperswitch/commit/eb10aca6313b3b3cb1763ca20b54b11c31b93b26))
- **router:** Add kv implementation for address for payment flows ([#2177](https://github.com/juspay/hyperswitch/pull/2177)) ([`afff3e1`](https://github.com/juspay/hyperswitch/commit/afff3e1789b99a586f0b7ff6c5880743a996f565))

### Bug Fixes

- **connector:**
  - [trustpay] add missing error_codes ([#2204](https://github.com/juspay/hyperswitch/pull/2204)) ([`8098322`](https://github.com/juspay/hyperswitch/commit/809832213eb0f961853bf0db8b2830a606f9ed37))
  - [Trustpay] Add missing error code ([#2212](https://github.com/juspay/hyperswitch/pull/2212)) ([`e4b3cc7`](https://github.com/juspay/hyperswitch/commit/e4b3cc790580f04012dba3d926e170dce4cec5d1))
- **env:** Remove EUR currency from clearpay_afterpay in stripe connector ([#2213](https://github.com/juspay/hyperswitch/pull/2213)) ([`9009ab2`](https://github.com/juspay/hyperswitch/commit/9009ab2896ef9c8df9045c288af5ad601ec7fcd7))

### Refactors

- **router:** Refactor customer <> address in customers and payments flow ([#2158](https://github.com/juspay/hyperswitch/pull/2158)) ([`8ee2ce1`](https://github.com/juspay/hyperswitch/commit/8ee2ce1f4fc416ac33a5e4def22ce2debdc6a6f9))

**Full Changelog:** [`v1.42.0...v1.43.0`](https://github.com/juspay/hyperswitch/compare/v1.42.0...v1.43.0)

- - -


## 1.42.0 (2023-09-20)

### Features

- **connector:** [Trustpay] Add Blik payment method for trustpay ([#2152](https://github.com/juspay/hyperswitch/pull/2152)) ([`d0eec9e`](https://github.com/juspay/hyperswitch/commit/d0eec9e357a2ef6074c9a02239337378fbf8412a))

### Bug Fixes

- **connector:** [SQUARE] Fix payments cancel issue ([#2162](https://github.com/juspay/hyperswitch/pull/2162)) ([`081545e`](https://github.com/juspay/hyperswitch/commit/081545e9121861ac7c1867a5e3f4c59ef848eeeb))

### Refactors

- **configs:** Make TOML file an optional source of application configuration ([#2185](https://github.com/juspay/hyperswitch/pull/2185)) ([`69fbebf`](https://github.com/juspay/hyperswitch/commit/69fbebf4630047ac33defc010811d1b4c4c9051a))
- **core:** Error thrown for wrong mca in applepay_verification flow change from 5xx to 4xx ([#2189](https://github.com/juspay/hyperswitch/pull/2189)) ([`656e710`](https://github.com/juspay/hyperswitch/commit/656e7106b44ba27a9058191259596e0a399aa20b))

**Full Changelog:** [`v1.41.0...v1.42.0`](https://github.com/juspay/hyperswitch/compare/v1.41.0...v1.42.0)

- - -


## 1.41.0 (2023-09-20)

### Features

- **connector:** [Gocardless] add boilerplate code ([#2179](https://github.com/juspay/hyperswitch/pull/2179)) ([`6a64180`](https://github.com/juspay/hyperswitch/commit/6a641806172e0fad6425a19baffda97ff7eb8c96))

### Bug Fixes

- **core:** Add merchant_id to gpay merchant info ([#2170](https://github.com/juspay/hyperswitch/pull/2170)) ([`5643ecf`](https://github.com/juspay/hyperswitch/commit/5643ecf07521abdebd162ed0c0fe389ae7942a17))
- Remove x-request-id from headers before connector calls ([#2182](https://github.com/juspay/hyperswitch/pull/2182)) ([`680505f`](https://github.com/juspay/hyperswitch/commit/680505f21ad0c809f007773517dd444b211f4c99))
- Handle 5xx during multiple capture call ([#2148](https://github.com/juspay/hyperswitch/pull/2148)) ([`e8d948e`](https://github.com/juspay/hyperswitch/commit/e8d948efeed3e9e4475ebc01d2be2ce3addd92a6))

### Refactors

- **connector:** [Adyen] psync validation check for adyen ([#2160](https://github.com/juspay/hyperswitch/pull/2160)) ([`386e820`](https://github.com/juspay/hyperswitch/commit/386e820fb85acfadd234670c6da2622bd2e38460))
- **core:** Add additional parameters in AppState and refactor AppState references ([#2123](https://github.com/juspay/hyperswitch/pull/2123)) ([`a0a8ef2`](https://github.com/juspay/hyperswitch/commit/a0a8ef27b319bdef01e72995081c7664c1e99127))
- **router:** Use billing address for payment method list filters as opposed to shipping address ([#2176](https://github.com/juspay/hyperswitch/pull/2176)) ([`b3d5d3b`](https://github.com/juspay/hyperswitch/commit/b3d5d3b3dcdde7480df8493714986b5e737e97e0))
- Remove redundant validate_capture_method call ([#2171](https://github.com/juspay/hyperswitch/pull/2171)) ([`1ea823b`](https://github.com/juspay/hyperswitch/commit/1ea823b0488c783315da156a474dedce2556d334))

**Full Changelog:** [`v1.40.1+hotfix.1...v1.41.0`](https://github.com/juspay/hyperswitch/compare/v1.40.1+hotfix.1...v1.41.0)

- - -


## 1.40.1 (2023-09-18)

### Refactors

- **connector:** [Bluesnap] Enahnce 3ds Flow ([#2115](https://github.com/juspay/hyperswitch/pull/2115)) ([`272f5e4`](https://github.com/juspay/hyperswitch/commit/272f5e4c1f34710fe13b1ede1b938d2f0b76e251))
- Set merchant_id as `MERCHANT_ID_NOT_FOUND` for traces and metrics if not found ([#2156](https://github.com/juspay/hyperswitch/pull/2156)) ([`d40fae8`](https://github.com/juspay/hyperswitch/commit/d40fae87feb509718059ab2d72539f37f26a8251))

**Full Changelog:** [`v1.40.0...v1.40.1`](https://github.com/juspay/hyperswitch/compare/v1.40.0...v1.40.1)

- - -


## 1.40.0 (2023-09-15)

### Features

- **connector:** (adyen) add support for multiple partial capture adyen ([#2102](https://github.com/juspay/hyperswitch/pull/2102)) ([`9668a74`](https://github.com/juspay/hyperswitch/commit/9668a74a79daf7b15069d5c21ebc43749e705558))
- **pm_auth:** Add plaid to connector list ([#2166](https://github.com/juspay/hyperswitch/pull/2166)) ([`0bc99ad`](https://github.com/juspay/hyperswitch/commit/0bc99ad327d1857dba67504ff12088e4bdd7102e))

### Bug Fixes

- **router:** Move `get_connector_tokenization_action_when_confirm_true` above `call_create_connector_customer_if_required` ([#2167](https://github.com/juspay/hyperswitch/pull/2167)) ([`15418a6`](https://github.com/juspay/hyperswitch/commit/15418a6d0f9429a69eaa179e5f7d9d798bf505e6))
- Make amount_capturable zero when payment intent status is processing ([#2163](https://github.com/juspay/hyperswitch/pull/2163)) ([`d848b55`](https://github.com/juspay/hyperswitch/commit/d848b55a119e426f809b46bd9d30b356ecd7ba2a))

### Refactors

- **router:** Add camel_case for the applepay request ([#2172](https://github.com/juspay/hyperswitch/pull/2172)) ([`4c36fcb`](https://github.com/juspay/hyperswitch/commit/4c36fcb34f086bb727c87fc5ede6e3bea138685a))

### Testing

- **postman:** Update postman collection files ([`b30d82d`](https://github.com/juspay/hyperswitch/commit/b30d82d9398ced95847eecdc22403febc32f3505))

**Full Changelog:** [`v1.39.2...v1.40.0`](https://github.com/juspay/hyperswitch/compare/v1.39.2...v1.40.0)

- - -


## 1.39.2 (2023-09-14)

### Bug Fixes

- **router:** Add scoped error enum for customer error ([#1988](https://github.com/juspay/hyperswitch/pull/1988)) ([`5c5058d`](https://github.com/juspay/hyperswitch/commit/5c5058de8765f2a0818115ee584a39981395213a))

### Refactors

- **connector:** [BraintreeGraphQl] Enhance currency Mapping with ConnectorCurrencyCommon Trait  ([#2143](https://github.com/juspay/hyperswitch/pull/2143)) ([`05696d3`](https://github.com/juspay/hyperswitch/commit/05696d326f87a08919f177e67bfa54e09fba5147))
- **router:**
  - Changed the storage of applepay_verified_domains from business_profile to merchant_connector_account table ([#2147](https://github.com/juspay/hyperswitch/pull/2147)) ([`caa385a`](https://github.com/juspay/hyperswitch/commit/caa385a5a6635a4bf7910e2d56e2660069c146a9))
  - Get route for applepay_verified_domains ([#2157](https://github.com/juspay/hyperswitch/pull/2157)) ([`fb1760b`](https://github.com/juspay/hyperswitch/commit/fb1760b1d8b5ca55dbaa93ab18f9fba9e7930e17))
- Add instrument to trackers for payment_confirm ([#2164](https://github.com/juspay/hyperswitch/pull/2164)) ([`c804954`](https://github.com/juspay/hyperswitch/commit/c8049542dea9b129ce81e6e550b9267642b8d027))

### Testing

- **postman:** Update postman collection files ([`089bb64`](https://github.com/juspay/hyperswitch/commit/089bb64e21451fa095acb93792ea745e1275d74e))

**Full Changelog:** [`v1.39.1+hotfix.1...v1.39.2`](https://github.com/juspay/hyperswitch/compare/v1.39.1+hotfix.1...v1.39.2)

- - -


## 1.39.1 (2023-09-13)

### Bug Fixes

- **connector:** [SQUARE] Add uri authority in Webhooks ([#2138](https://github.com/juspay/hyperswitch/pull/2138)) ([`daa0759`](https://github.com/juspay/hyperswitch/commit/daa07598922d1bf0c61e2482752570153f62cdb1))
- **core:** Update amount_capturable in update trackers ([#2142](https://github.com/juspay/hyperswitch/pull/2142)) ([`bed8326`](https://github.com/juspay/hyperswitch/commit/bed8326597febd89bb4c961c9085a78b09f99f49))
- Payment status fix in trustpay for 3ds and wallets ([#2146](https://github.com/juspay/hyperswitch/pull/2146)) ([`9b92d04`](https://github.com/juspay/hyperswitch/commit/9b92d046de9fb794d67163582af4360d5e558037))

### Refactors

- **connector:** [Stripe] add support for more incoming woocommerce Stripe disputes webhooks ([#2150](https://github.com/juspay/hyperswitch/pull/2150)) ([`e023eb8`](https://github.com/juspay/hyperswitch/commit/e023eb800d17ffc24cfaf2335d2560fb0f529e50))
- **masking:** Move masking implementations to masking crate ([#2135](https://github.com/juspay/hyperswitch/pull/2135)) ([`9d74a75`](https://github.com/juspay/hyperswitch/commit/9d74a75ddbd49e7ef7fa0cbfab1528da342dd5a0))
- Move `Request` and `RequestBuilder` structs to common_utils crate ([#2145](https://github.com/juspay/hyperswitch/pull/2145)) ([`21be67a`](https://github.com/juspay/hyperswitch/commit/21be67ada07e41f3ff8824f608a82b606201892a))

### Testing

- **postman:** Update postman collection files ([`be397de`](https://github.com/juspay/hyperswitch/commit/be397dec48d143d9180f316659aa033f668c1a55))

**Full Changelog:** [`v1.39.0...v1.39.1`](https://github.com/juspay/hyperswitch/compare/v1.39.0...v1.39.1)

- - -


## 1.39.0 (2023-09-12)

### Features

- **connector:**
  - [Braintree] implement 3DS card payment for braintree ([#2095](https://github.com/juspay/hyperswitch/pull/2095)) ([`d63cbbd`](https://github.com/juspay/hyperswitch/commit/d63cbbd4ad8eb2438967b1538da363b67964750f))
  - [payme] Add support for dispute webhooks ([#2089](https://github.com/juspay/hyperswitch/pull/2089)) ([`341163b`](https://github.com/juspay/hyperswitch/commit/341163b4814fe9671d5d40305168046c065f4908))
- **core:**
  - Enable payments void for multiple partial capture ([#2048](https://github.com/juspay/hyperswitch/pull/2048)) ([`a81bfe2`](https://github.com/juspay/hyperswitch/commit/a81bfe28edd7fc543af19b9546cbe30492716c97))
  - Add runtime flag to disable dummy connector ([#2100](https://github.com/juspay/hyperswitch/pull/2100)) ([`d52fe7f`](https://github.com/juspay/hyperswitch/commit/d52fe7f1403b6b1fc71b275b6bc22345dd6d1e8a))
- **db:** Implement `ReverseLookupInterface` for `MockDb` ([#2119](https://github.com/juspay/hyperswitch/pull/2119)) ([`f2df2d6`](https://github.com/juspay/hyperswitch/commit/f2df2d6d01a1bf71541bf18d2ecf6dc1e667942f))
- **router:**
  - Disable temp locker call for connector-payment_method flow based on env ([#2120](https://github.com/juspay/hyperswitch/pull/2120)) ([`fea5c4d`](https://github.com/juspay/hyperswitch/commit/fea5c4d8c186f3b4e732f7d503e49724c3e4d308))
  - New get route for derivation of verified applepay domain ([#2121](https://github.com/juspay/hyperswitch/pull/2121)) ([`177d8e5`](https://github.com/juspay/hyperswitch/commit/177d8e5237241d7deea5fd911749ea0a934abcb0))
  - Added new webhook URL to support `merchant_connector_id` ([#2006](https://github.com/juspay/hyperswitch/pull/2006)) ([`82b36e8`](https://github.com/juspay/hyperswitch/commit/82b36e885d346a9bcc50968f0c1f8ba85b9d3378))

### Bug Fixes

- **connector:** [SQUARE] Throw Error for Partial Capture of Payments ([#2133](https://github.com/juspay/hyperswitch/pull/2133)) ([`cc8847c`](https://github.com/juspay/hyperswitch/commit/cc8847cce0022b375626b3c86e5b07048833be71))
- **core:** [Bluesnap] Add secondary_base_url for script ([#2124](https://github.com/juspay/hyperswitch/pull/2124)) ([`1407049`](https://github.com/juspay/hyperswitch/commit/1407049b56bd07237b2f9ad6c12a92837995abfa))
- **payment_methods:** Default card fetch to locker call ([#2125](https://github.com/juspay/hyperswitch/pull/2125)) ([`ffe9009`](https://github.com/juspay/hyperswitch/commit/ffe9009d6525f214e02f51998b7916f649170222))
- **refactor:** [Paypal] refactor paypal not implemented payment methods errors ([#1974](https://github.com/juspay/hyperswitch/pull/1974)) ([`ca9fb0c`](https://github.com/juspay/hyperswitch/commit/ca9fb0caf018715a77a4364a28537e99d76b1d32))
- **router:** Move connector customer create flow to `call_connector_service` ([#2137](https://github.com/juspay/hyperswitch/pull/2137)) ([`4d3e6bc`](https://github.com/juspay/hyperswitch/commit/4d3e6bcb6c806a86a24694bb35cfa0293525c5ad))
- **router/scheduler:** Replace the occurrences of gen_range with a safer alternative ([#2126](https://github.com/juspay/hyperswitch/pull/2126)) ([`94ac5c0`](https://github.com/juspay/hyperswitch/commit/94ac5c03b2280827ac2efa5a040cf4cb9073f6c6))
- **webhooks:** Fix database queries in webhook  ([#2139](https://github.com/juspay/hyperswitch/pull/2139)) ([`eff280f`](https://github.com/juspay/hyperswitch/commit/eff280f2fbaba392a61d6f55fb251de106273a41))
- Eliminate recursive call while updating config in database ([#2128](https://github.com/juspay/hyperswitch/pull/2128)) ([`a3dd8b7`](https://github.com/juspay/hyperswitch/commit/a3dd8b7d1e4fb7bc7a6ab6e3903cb990c9f2171b))

### Refactors

- **connector:** [Zen] refactor Zen payment methods not implemented errors ([#1955](https://github.com/juspay/hyperswitch/pull/1955)) ([`b0c4ee2`](https://github.com/juspay/hyperswitch/commit/b0c4ee2cf28daa147cc333f3c1e6c3ac0c0b115b))
- **pm_list:** Get profile_id from business_details in list pm ([#2131](https://github.com/juspay/hyperswitch/pull/2131)) ([`90868b9`](https://github.com/juspay/hyperswitch/commit/90868b93d6ac20b025ed52781d18c7ffffc5ee78))

### Testing

- **postman:** Update postman collection files ([`7e29adb`](https://github.com/juspay/hyperswitch/commit/7e29adb5c9dee8b03ef58ccbd85b07b106459380))

**Full Changelog:** [`v1.38.0...v1.39.0`](https://github.com/juspay/hyperswitch/compare/v1.38.0...v1.39.0)

- - -


## 1.38.0 (2023-09-11)

### Features

- **confirm:** Reduce the database calls to 2 stages in case of non-retry ([#2113](https://github.com/juspay/hyperswitch/pull/2113)) ([`28b102d`](https://github.com/juspay/hyperswitch/commit/28b102de2496c0880b6b232ddc82b1ef227af4da))
- **core:** Accept payment_confirm_source header in capture call and store in payment_intent ([#2116](https://github.com/juspay/hyperswitch/pull/2116)) ([`2f272d2`](https://github.com/juspay/hyperswitch/commit/2f272d2962901b3e52b547bc0363bfbfb8030277))
- **router:** Saving verified domains to business_profile table ([#2109](https://github.com/juspay/hyperswitch/pull/2109)) ([`73da641`](https://github.com/juspay/hyperswitch/commit/73da641b58bbfc1b0bd4bf8872b7b316a135b5c7))

### Bug Fixes

- **router:** `validate_psync_reference_id` only if call_connector_action is trigger in psync flow ([#2106](https://github.com/juspay/hyperswitch/pull/2106)) ([`60c5fdb`](https://github.com/juspay/hyperswitch/commit/60c5fdb89a771b7d1e4d41f3ed11daa00bd10f91))
- Implement persistent caching for config table retrieval ([#2044](https://github.com/juspay/hyperswitch/pull/2044)) ([`25e82a1`](https://github.com/juspay/hyperswitch/commit/25e82a1f7f2cb547e9c42c5bab4b898dd1886d6f))

### Refactors

- **core:** Use profile id to find connector ([#2020](https://github.com/juspay/hyperswitch/pull/2020)) ([`5b29c25`](https://github.com/juspay/hyperswitch/commit/5b29c25210ed118dcd97dafd608170c41b1fba58))
- **storage_impl:** Split payment attempt models to domain + diesel ([#2010](https://github.com/juspay/hyperswitch/pull/2010)) ([`ad4b7de`](https://github.com/juspay/hyperswitch/commit/ad4b7de628ca4e0f56a06d8b9f5e2c8c5bace67a))

### Testing

- **connector:** Skip ui sanity tests for external contributors ([#2118](https://github.com/juspay/hyperswitch/pull/2118)) ([`f5fed94`](https://github.com/juspay/hyperswitch/commit/f5fed9413083a6635c3d2222d28bd67d5d994eea))

**Full Changelog:** [`v1.37.0...v1.38.0`](https://github.com/juspay/hyperswitch/compare/v1.37.0...v1.38.0)

- - -


## 1.37.0 (2023-09-10)

### Features

- **connector:**
  - (checkout.com) add support for multiple captures PSync ([#2043](https://github.com/juspay/hyperswitch/pull/2043)) ([`517c5c4`](https://github.com/juspay/hyperswitch/commit/517c5c41655f82ab773f6875447d7d88390d538e))
  - [Cryptopay]Add reference id for cryptopay ([#2107](https://github.com/juspay/hyperswitch/pull/2107)) ([`576648b`](https://github.com/juspay/hyperswitch/commit/576648b5a5d7775d295479df3438c913ae855827))
- **db:** Implement `BusinessProfileInterface` for `MockDb` ([#2101](https://github.com/juspay/hyperswitch/pull/2101)) ([`0792605`](https://github.com/juspay/hyperswitch/commit/07926050887cdd5d9e3a558ede4212074d17e257))
- **payments:** Make database calls parallel for `payments_confirm` operation ([#2098](https://github.com/juspay/hyperswitch/pull/2098)) ([`fea075e`](https://github.com/juspay/hyperswitch/commit/fea075e32efd5031b5d38a9e34bedb85b0f99e95))

### Bug Fixes

- **connector:** Revert checkout apple pay to tokenization flow ([#2110](https://github.com/juspay/hyperswitch/pull/2110)) ([`cc5add6`](https://github.com/juspay/hyperswitch/commit/cc5add625da44aeb9d30f02d21d415be12ce0c48))
- Null value in session token in next action   ([#2111](https://github.com/juspay/hyperswitch/pull/2111)) ([`f015394`](https://github.com/juspay/hyperswitch/commit/f015394e7ac52f891b32e8147ae8aabf2ef9b593))

### Refactors

- **connector:**
  - [Stripe] Using `connector_request_reference_id` as object_reference_id for Webhooks ([#2064](https://github.com/juspay/hyperswitch/pull/2064)) ([`e659e70`](https://github.com/juspay/hyperswitch/commit/e659e7029e758ef46b4fd12b262a58d0c3f5e5c0))
  - [Adyen] refactor adyen payment method not implemented errors ([#1950](https://github.com/juspay/hyperswitch/pull/1950)) ([`955534e`](https://github.com/juspay/hyperswitch/commit/955534e9535b3add4841d2bcfe51536c81fd9244))

**Full Changelog:** [`v1.36.0...v1.37.0`](https://github.com/juspay/hyperswitch/compare/v1.36.0...v1.37.0)

- - -


## 1.36.0 (2023-09-07)

### Features

- **apple_pay:** Add support for pre decrypted apple pay token ([#2056](https://github.com/juspay/hyperswitch/pull/2056)) ([`75ee632`](https://github.com/juspay/hyperswitch/commit/75ee6327820fe31ff2c379250eae3e7974e6ae6c))

### Refactors

- **connector:**
  - [Payme] Rename types to follow naming conventions ([#2096](https://github.com/juspay/hyperswitch/pull/2096)) ([`98d7005`](https://github.com/juspay/hyperswitch/commit/98d70054e25ad8b2473110f7cde803f119b69d37))
  - [Payme] Response Handling for Preprocessing ([#2097](https://github.com/juspay/hyperswitch/pull/2097)) ([`bdf4832`](https://github.com/juspay/hyperswitch/commit/bdf48320f9d4f1dc8c13f42f6e1e06d1056acf33))
- **router:** Changed auth of verify_apple_pay from mid to jwt ([#2094](https://github.com/juspay/hyperswitch/pull/2094)) ([`8246f4e`](https://github.com/juspay/hyperswitch/commit/8246f4e9c336152ca79e916375cd11618af4d90a))

### Miscellaneous Tasks

- **deps:** Bump webpki from 0.22.0 to 0.22.1 ([#2104](https://github.com/juspay/hyperswitch/pull/2104)) ([`81c6480`](https://github.com/juspay/hyperswitch/commit/81c6480bdf2ab65b433ff2e89fcc299198019307))
- Address Rust 1.72 clippy lints ([#2099](https://github.com/juspay/hyperswitch/pull/2099)) ([`cbbebe2`](https://github.com/juspay/hyperswitch/commit/cbbebe2408093d84a51b3916ea5a43d79404b4e9))

**Full Changelog:** [`v1.35.0...v1.36.0`](https://github.com/juspay/hyperswitch/compare/v1.35.0...v1.36.0)

- - -


## 1.35.0 (2023-09-06)

### Features

- **connector:**
  - [Payme] Implement Card 3DS with sdk flow ([#2082](https://github.com/juspay/hyperswitch/pull/2082)) ([`99f1780`](https://github.com/juspay/hyperswitch/commit/99f1780fd76c7761693df1b22db9104bfa12270b))
  - [SQUARE] Implement webhooks ([#1980](https://github.com/juspay/hyperswitch/pull/1980)) ([`5a49802`](https://github.com/juspay/hyperswitch/commit/5a49802f56cd3521bbdd38581a1417fa072fb696))
- **payment_methods:** Store necessary payment method data in payment_methods table ([#2073](https://github.com/juspay/hyperswitch/pull/2073)) ([`3c93552`](https://github.com/juspay/hyperswitch/commit/3c935521019c5882674e0e6d16e9d331b5b9f756))

### Bug Fixes

- **connector:** [STAX] Incoming amount will be processed in higher unit ([#2091](https://github.com/juspay/hyperswitch/pull/2091)) ([`de9e0fe`](https://github.com/juspay/hyperswitch/commit/de9e0feac0e002a022356233e8f0b62500ce75ed))
- **router:** Send connection_closed errors as 5xx instead of 2xx ([#2093](https://github.com/juspay/hyperswitch/pull/2093)) ([`4d58bdb`](https://github.com/juspay/hyperswitch/commit/4d58bdbe2939b9952baf6c8faa48fff09a2409f7))

### Refactors

- **refunds:** Add success RefundStatus in should_call_refund check ([#2081](https://github.com/juspay/hyperswitch/pull/2081)) ([`9cae5de`](https://github.com/juspay/hyperswitch/commit/9cae5de5ffa27ce71110d703a221da65ac586d29))
- **scheduler:** Move scheduler to new crate to support workflows in multiple crates ([#1681](https://github.com/juspay/hyperswitch/pull/1681)) ([`d4221f3`](https://github.com/juspay/hyperswitch/commit/d4221f33689b2c26b2e5753f9a3b7943811b20a3))

### Testing

- **postman:** Update postman collection files ([`25f8c35`](https://github.com/juspay/hyperswitch/commit/25f8c3556f366a92a2f6e2121afe895091c3fae8))

**Full Changelog:** [`v1.34.1...v1.35.0`](https://github.com/juspay/hyperswitch/compare/v1.34.1...v1.35.0)

- - -


## 1.34.1 (2023-09-05)

### Bug Fixes

- Add accounts_cache for release ([#2087](https://github.com/juspay/hyperswitch/pull/2087)) ([`e5d3180`](https://github.com/juspay/hyperswitch/commit/e5d31801ec671191ab0365cf9650fb467f252102))

### Refactors

- **router:** New separate routes for applepay merchant verification ([#2083](https://github.com/juspay/hyperswitch/pull/2083)) ([`dc908f6`](https://github.com/juspay/hyperswitch/commit/dc908f6902d3260b08ebf0019b2466553871de0e))

### Testing

- **postman:** Update postman collection files ([#2070](https://github.com/juspay/hyperswitch/pull/2070)) ([`cfa6ae8`](https://github.com/juspay/hyperswitch/commit/cfa6ae895d72cb6c0e79d1ee6616183f35121be1))

**Full Changelog:** [`v1.34.0...v1.34.1`](https://github.com/juspay/hyperswitch/compare/v1.34.0...v1.34.1)

- - -


## 1.34.0 (2023-09-04)

### Features

- **frm:**
  - Enum variant misspelled changed from fullfillment to fulfillment ([#2065](https://github.com/juspay/hyperswitch/pull/2065)) ([`e1cebd4`](https://github.com/juspay/hyperswitch/commit/e1cebd41798172b586f81d2668bedf18fa82001d))
  - Add support to accept and decline payment when manually reviewed by merchant for risky transaction ([#2071](https://github.com/juspay/hyperswitch/pull/2071)) ([`229f111`](https://github.com/juspay/hyperswitch/commit/229f111f6cb4ea30caa7b89328a047a1be8b9be0))

### Refactors

- Include binary name in `service` field in log entries ([#2077](https://github.com/juspay/hyperswitch/pull/2077)) ([`20d44ac`](https://github.com/juspay/hyperswitch/commit/20d44acd20757c333382cd78875c8c9a7c35503c))

### Documentation

- **postman:** Update documentation for postman tests ([#2057](https://github.com/juspay/hyperswitch/pull/2057)) ([`119aeb4`](https://github.com/juspay/hyperswitch/commit/119aeb49ca3810cf095590fd65fdfc74a6efc27e))

**Full Changelog:** [`v1.33.0...v1.34.0`](https://github.com/juspay/hyperswitch/compare/v1.33.0...v1.34.0)

- - -


## 1.33.0 (2023-09-03)

### Features

- **api:** Use `ApiClient` trait in AppState ([#2067](https://github.com/juspay/hyperswitch/pull/2067)) ([`29fd2ea`](https://github.com/juspay/hyperswitch/commit/29fd2eaab1f7d028a833d0cf87dfde2a4327da99))
- **connector:**
  - [Zen] Use `connector_request_reference_id` as Transaction Id to Retrieve Payments ([#2052](https://github.com/juspay/hyperswitch/pull/2052)) ([`5b92c39`](https://github.com/juspay/hyperswitch/commit/5b92c39470e5a0268f9e53ecf2527772b1384802))
  - [Bluesnap] Add dispute webhooks support ([#2053](https://github.com/juspay/hyperswitch/pull/2053)) ([`f8410b5`](https://github.com/juspay/hyperswitch/commit/f8410b5b2a5191866a4631bcdc475b608440b17b))
  - [Paypal] Add manual capture for paypal wallet  ([#2072](https://github.com/juspay/hyperswitch/pull/2072)) ([`99ff82e`](https://github.com/juspay/hyperswitch/commit/99ff82ef6d42899d6cb16f05c7a0c2bc193074a3))
- **pm_list:** Add card - credit pm type required field info for connectors ([#2075](https://github.com/juspay/hyperswitch/pull/2075)) ([`a882d76`](https://github.com/juspay/hyperswitch/commit/a882d7604c68b9360d0cbe6c6ef43815a39e669a))
- **webhooks:** Webhook source verification ([#2069](https://github.com/juspay/hyperswitch/pull/2069)) ([`8b22f38`](https://github.com/juspay/hyperswitch/commit/8b22f38dd6b897c5b349c25d41c89fffa07f5135))

### Bug Fixes

- **connector:**
  - [Paypal] fix PSync for redirection flow for PayPal ([#2068](https://github.com/juspay/hyperswitch/pull/2068)) ([`e730c73`](https://github.com/juspay/hyperswitch/commit/e730c73516888d9b29209e805d1409ccdc2d4525))
  - [STAX] Add ACH Payment Filter for Bank Debits ([#2074](https://github.com/juspay/hyperswitch/pull/2074)) ([`a12a370`](https://github.com/juspay/hyperswitch/commit/a12a370bf6a7349acf6ff585adf55b56446a425e))
- **router:** Correct limit for payments list by filters ([#2060](https://github.com/juspay/hyperswitch/pull/2060)) ([`b7d6d31`](https://github.com/juspay/hyperswitch/commit/b7d6d31504c1f8705c5bbcdda9afdd5f3575657b))

### Refactors

- **connector:** [Shift4] refactor connector authorize request struct  ([#1888](https://github.com/juspay/hyperswitch/pull/1888)) ([`e44c32d`](https://github.com/juspay/hyperswitch/commit/e44c32dd80a72aef37674a5fcc630f5ea88e6343))
- **router:** Return generic message for UnprocessableEntity in make_pm_data ([#2050](https://github.com/juspay/hyperswitch/pull/2050)) ([`38ab6e5`](https://github.com/juspay/hyperswitch/commit/38ab6e54f1aa0e2cf03c67164d6787850d40e070))

**Full Changelog:** [`v1.32.0...v1.33.0`](https://github.com/juspay/hyperswitch/compare/v1.32.0...v1.33.0)

- - -


## 1.32.0 (2023-08-31)

### Features

- **connector:** [Square] Implement Card Payments for Square ([#1902](https://github.com/juspay/hyperswitch/pull/1902)) ([`c9fe389`](https://github.com/juspay/hyperswitch/commit/c9fe389b2c04817a843e34de0aab3d024bb31f19))
- **core:** Connector specific validation for Payment Sync ([#2005](https://github.com/juspay/hyperswitch/pull/2005)) ([`098dc89`](https://github.com/juspay/hyperswitch/commit/098dc89d0cc9c1a2e0fbbb5384fa6f55a3a6a9a2))
- **router:**
  - Verify service for applepay merchant registration ([#2009](https://github.com/juspay/hyperswitch/pull/2009)) ([`636b871`](https://github.com/juspay/hyperswitch/commit/636b871b1199703ce8e9c7c4b15284c45eff37ac))
  - Send connector timeouts and connection closures as 2xx response instead of giving 5xx response ([#2047](https://github.com/juspay/hyperswitch/pull/2047)) ([`31088b6`](https://github.com/juspay/hyperswitch/commit/31088b606261d2524f2f84ea0c34a40ab56a7e9d))

### Bug Fixes

- **connector:** [Bluesnap] make error_name as optional field ([#2045](https://github.com/juspay/hyperswitch/pull/2045)) ([`ab85617`](https://github.com/juspay/hyperswitch/commit/ab8561793549712ac50755525eab4dc6b5b19925))
- **mock_db:** Insert merchant for mock_db ([#1984](https://github.com/juspay/hyperswitch/pull/1984)) ([`fb39795`](https://github.com/juspay/hyperswitch/commit/fb397956adf20219e039548b6a3682ba526a23f4))

### Refactors

- **router:** Fixed unprocessable entity error message to custom message ([#1979](https://github.com/juspay/hyperswitch/pull/1979)) ([`655b388`](https://github.com/juspay/hyperswitch/commit/655b388358ecb7d3c3e990d19989febea9f9d4c9))

### Testing

- **postman:** Update event file format to latest supported ([#2055](https://github.com/juspay/hyperswitch/pull/2055)) ([`eeee0ed`](https://github.com/juspay/hyperswitch/commit/eeee0ed5dc830279d57b07f48f6b3f6ecc95f8f1))

### Documentation

- **CONTRIBUTING:** Fix open a discussion link ([#2054](https://github.com/juspay/hyperswitch/pull/2054)) ([`58105d4`](https://github.com/juspay/hyperswitch/commit/58105d4ae2eedea137c179c91775e5ec5524897a))

### Miscellaneous Tasks

- Add metrics for external api call ([#2021](https://github.com/juspay/hyperswitch/pull/2021)) ([`08fb2a9`](https://github.com/juspay/hyperswitch/commit/08fb2a93c19981f5f8e81ce9a8d267929933f832))

**Full Changelog:** [`v1.31.0...v1.32.0`](https://github.com/juspay/hyperswitch/compare/v1.31.0...v1.32.0)

- - -


## 1.31.0 (2023-08-30)

### Features

- **core:** Conditionally return captures list during payment sync. ([#2033](https://github.com/juspay/hyperswitch/pull/2033)) ([`c2aa014`](https://github.com/juspay/hyperswitch/commit/c2aa0142ed5af0b5fcf21b35cb129addd92c6125))

### Bug Fixes

- **configs:** Fix supported connectors in `multiple_api_version_supported_connectors` table ([#2051](https://github.com/juspay/hyperswitch/pull/2051)) ([`416ad8f`](https://github.com/juspay/hyperswitch/commit/416ad8fd97e423bfdb95409271628085aa97af76))
- **connector:** [Cryptopay] fix amount to its currency base unit  ([#2049](https://github.com/juspay/hyperswitch/pull/2049)) ([`d3f1858`](https://github.com/juspay/hyperswitch/commit/d3f18584f8e8a6090f24c4a469c6a18440d6711e))

**Full Changelog:** [`v1.30.0...v1.31.0`](https://github.com/juspay/hyperswitch/compare/v1.30.0...v1.31.0)

- - -


## 1.30.0 (2023-08-29)

### Features

- **connector:**
  - [HELCIM] Add template code for Helcim ([#2019](https://github.com/juspay/hyperswitch/pull/2019)) ([`d804b23`](https://github.com/juspay/hyperswitch/commit/d804b2328274189cf5ddab9aac5bee56838618da))
  - (globalpay) add support for multilple partial capture ([#2035](https://github.com/juspay/hyperswitch/pull/2035)) ([`a93eea7`](https://github.com/juspay/hyperswitch/commit/a93eea734f2645132d05332f7e25eca486ef0eda))
  - (checkout_dot_com) add support for multiple partial captures ([#1977](https://github.com/juspay/hyperswitch/pull/1977)) ([`784702d`](https://github.com/juspay/hyperswitch/commit/784702d9c55313179e59a5cf62f14f94b46317a5))
- **router:** Add total count for payments list ([#1912](https://github.com/juspay/hyperswitch/pull/1912)) ([`7a5c841`](https://github.com/juspay/hyperswitch/commit/7a5c8413cfcaa4d33a59dfa7035645b5cd310cb5))

### Bug Fixes

- **connector:** Change 5xx to 4xx for Coinbase and Iatapay ([#1975](https://github.com/juspay/hyperswitch/pull/1975)) ([`e64d5a3`](https://github.com/juspay/hyperswitch/commit/e64d5a3fc286df0f60f65fcedf7bc4d8aa974721))

### Refactors

- **recon:** Updating user flow for recon ([#2029](https://github.com/juspay/hyperswitch/pull/2029)) ([`1510623`](https://github.com/juspay/hyperswitch/commit/15106233e973fb7539799b96975a1004c2925663))

**Full Changelog:** [`v1.29.0...v1.30.0`](https://github.com/juspay/hyperswitch/compare/v1.29.0...v1.30.0)

- - -


## 1.29.0 (2023-08-29)

### Features

- **connector:** [Paypal] add support for payment and refund webhooks ([#2003](https://github.com/juspay/hyperswitch/pull/2003)) ([`ade27f0`](https://github.com/juspay/hyperswitch/commit/ade27f01686d2a0cdee86d4d366cecaa12370ba6))

### Bug Fixes

- **connector:** [Payme] populate error message in case of 2xx payment failures ([#2037](https://github.com/juspay/hyperswitch/pull/2037)) ([`aeebc5b`](https://github.com/juspay/hyperswitch/commit/aeebc5b52584ad8d8c128fa896d39fe8576dca0c))
- **router:** Remove `attempt_count` in payments list response and add it in payments response ([#2008](https://github.com/juspay/hyperswitch/pull/2008)) ([`23b8d34`](https://github.com/juspay/hyperswitch/commit/23b8d3412c7d14e450b87b3ccb35a394d954d0a7))

### Miscellaneous Tasks

- **creds:** Update connector API credentials ([#2034](https://github.com/juspay/hyperswitch/pull/2034)) ([`f04bee2`](https://github.com/juspay/hyperswitch/commit/f04bee261141622b63e34e1ebd4b0de4641e0210))
- Address Rust 1.72 clippy lints ([#2011](https://github.com/juspay/hyperswitch/pull/2011)) ([`eaefa6e`](https://github.com/juspay/hyperswitch/commit/eaefa6e15c4facc28440d7fdc3aac9be0976324d))

**Full Changelog:** [`v1.28.1...v1.29.0`](https://github.com/juspay/hyperswitch/compare/v1.28.1...v1.29.0)

- - -


## 1.28.1 (2023-08-28)

### Bug Fixes

- **connector:** [Noon] handle 2 digit exp year and 3ds checked status ([#2022](https://github.com/juspay/hyperswitch/pull/2022)) ([`322c615`](https://github.com/juspay/hyperswitch/commit/322c615c56c37554ae9760b9a584bf3b0032cf43))

### Refactors

- **postman:** Remove `routing algorithm` struct from `merchant account create` ([#2032](https://github.com/juspay/hyperswitch/pull/2032)) ([`3d4f750`](https://github.com/juspay/hyperswitch/commit/3d4f750089b97f0fde0e74b833bf386327fb4a52))

**Full Changelog:** [`v1.28.0...v1.28.1`](https://github.com/juspay/hyperswitch/compare/v1.28.0...v1.28.1)

- - -


## 1.28.0 (2023-08-28)

### Features

- **connector:** [CashToCode] perform currency based connector credentials mapping ([#2025](https://github.com/juspay/hyperswitch/pull/2025)) ([`7c0c3b6`](https://github.com/juspay/hyperswitch/commit/7c0c3b6b35f2654bbb64c9631c308925bbf5226d))

**Full Changelog:** [`v1.27.2...v1.28.0`](https://github.com/juspay/hyperswitch/compare/v1.27.2...v1.28.0)

- - -


## 1.27.2 (2023-08-27)

### Bug Fixes

- **request:** Add `idle_pool_connection_timeout` as a config ([#2016](https://github.com/juspay/hyperswitch/pull/2016)) ([`6247996`](https://github.com/juspay/hyperswitch/commit/6247996ddead66086551eef0de8f0b5d678eec27))

### Refactors

- **core:** Authenticate client secret with fulfilment time ([#2026](https://github.com/juspay/hyperswitch/pull/2026)) ([`1e44c8d`](https://github.com/juspay/hyperswitch/commit/1e44c8df1e57351bc5d704d7fc0bee66c5e84aec))

**Full Changelog:** [`v1.27.1...v1.27.2`](https://github.com/juspay/hyperswitch/compare/v1.27.1...v1.27.2)

- - -


## 1.27.1 (2023-08-25)

### Bug Fixes

- **locker:** Accept the incoming token as the basilisk token if it is a mandate payment ([#2013](https://github.com/juspay/hyperswitch/pull/2013)) ([`ac63794`](https://github.com/juspay/hyperswitch/commit/ac637941623ffe7e2b3d6445ea18b5aabbee513f))
- **payment:** Fix max limit on payment intents list ([#2014](https://github.com/juspay/hyperswitch/pull/2014)) ([`a888953`](https://github.com/juspay/hyperswitch/commit/a8889530043efb455b6a20ebffd2e972b5224b6f))

### Testing

- **connector:** Add support for adyen webhooks ([#1999](https://github.com/juspay/hyperswitch/pull/1999)) ([`fcaca76`](https://github.com/juspay/hyperswitch/commit/fcaca76c72bdea19125ae07d927bfd6119353c45))

**Full Changelog:** [`v1.27.0...v1.27.1`](https://github.com/juspay/hyperswitch/compare/v1.27.0...v1.27.1)

- - -


## 1.27.0 (2023-08-24)

### Features

- **api_client:** Add api client trait ([#1919](https://github.com/juspay/hyperswitch/pull/1919)) ([`97b2747`](https://github.com/juspay/hyperswitch/commit/97b2747458fbc9d823d56a6d69eaa5f914c64054))
- **connector:** [Braintree] Add Authorize, Capture, Void, PSync, Refund, Rsync for Braintree GraphQL API ([#1962](https://github.com/juspay/hyperswitch/pull/1962)) ([`820f615`](https://github.com/juspay/hyperswitch/commit/820f6153af10a288afb458089d4bbb2495cd5488))

### Bug Fixes

- **connector:**
  - [Paypal] fix amount to its currency base unit for Paypal Bank redirects ([#2002](https://github.com/juspay/hyperswitch/pull/2002)) ([`4accb41`](https://github.com/juspay/hyperswitch/commit/4accb41ef4ffaec8ac177b938c0f61b0737cc2c8))
  - [Trustpay] Add missing payment status codes in failure check ([#1997](https://github.com/juspay/hyperswitch/pull/1997)) ([`e889749`](https://github.com/juspay/hyperswitch/commit/e8897491b1395e9007d47108c42b789ded354592))
  - Fix payme error response deserialization error ([#1989](https://github.com/juspay/hyperswitch/pull/1989)) ([`16facdf`](https://github.com/juspay/hyperswitch/commit/16facdfa71049a968d448167f63963deb8b50cd0))
  - [Bluesnap] Update incoming Webhooks flow  ([#1982](https://github.com/juspay/hyperswitch/pull/1982)) ([`8c066d3`](https://github.com/juspay/hyperswitch/commit/8c066d3ea73481106982ced5f09058383bc97953))

### Testing

- Move Postman collections to directory structure ([#1995](https://github.com/juspay/hyperswitch/pull/1995)) ([`b7e4048`](https://github.com/juspay/hyperswitch/commit/b7e4048e56fc73bb741a5b25487cd3f56febf90e))

### Miscellaneous Tasks

- **creds:** Updated the API Keys to not use wrong creds ([#2001](https://github.com/juspay/hyperswitch/pull/2001)) ([`ad991c0`](https://github.com/juspay/hyperswitch/commit/ad991c04ecedd85ca4c432126487042c2fd03a67))

**Full Changelog:** [`v1.26.0...v1.27.0`](https://github.com/juspay/hyperswitch/compare/v1.26.0...v1.27.0)

- - -


## 1.26.0 (2023-08-23)

### Features

- **business_profile:** Add profile id in affected tables and modify api contract ([#1971](https://github.com/juspay/hyperswitch/pull/1971)) ([`fe8d4c2`](https://github.com/juspay/hyperswitch/commit/fe8d4c2eeca21e0d79c7a056505790c8cadaef9d))
- **connector:** Fail payment authorize when capture_method is manual_method ([#1893](https://github.com/juspay/hyperswitch/pull/1893)) ([`bca9d50`](https://github.com/juspay/hyperswitch/commit/bca9d5013b902d813a41f04286ea6cb645e1f199))
- **core:** Add psync for multiple partial captures ([#1934](https://github.com/juspay/hyperswitch/pull/1934)) ([`5657ad6`](https://github.com/juspay/hyperswitch/commit/5657ad6933bb407d2ae32f2e068e56c9b9698ed3))
- **pm_list:** Add  card pm required field info for connectors ([#1918](https://github.com/juspay/hyperswitch/pull/1918)) ([`52e0176`](https://github.com/juspay/hyperswitch/commit/52e01769d405308b0b882647e2e824f38aeef3dc))
- **router:**
  - Add relevant metrics and logs for manual retries flow ([#1985](https://github.com/juspay/hyperswitch/pull/1985)) ([`1b346fc`](https://github.com/juspay/hyperswitch/commit/1b346fcf5649a24becff2751aa6f93d7a863ee61))
  - Add fields in payments list response ([#1987](https://github.com/juspay/hyperswitch/pull/1987)) ([`abc736b`](https://github.com/juspay/hyperswitch/commit/abc736bbc13288d9b35c74ed12ec7da443643ee0))
  - Add `attempt_count` in list payments response ([#1990](https://github.com/juspay/hyperswitch/pull/1990)) ([`f0cc0fb`](https://github.com/juspay/hyperswitch/commit/f0cc0fba1684f200ce6dbf3e4bc951de23a60f94))

### Bug Fixes

- **test_utils:** Remove `cmd` alias for `std::process::Command` ([#1981](https://github.com/juspay/hyperswitch/pull/1981)) ([`c161530`](https://github.com/juspay/hyperswitch/commit/c161530a6c8a5486e1cd3fe16f8f01e0ca580108))
- **webhooks:**
  - Send stripe compatible webhooks for stripe compatible merchants ([#1986](https://github.com/juspay/hyperswitch/pull/1986)) ([`36631ad`](https://github.com/juspay/hyperswitch/commit/36631ad97be509d397b91babd4cd1a492703bb5c))
  - Handling errors inside source verification ([#1994](https://github.com/juspay/hyperswitch/pull/1994)) ([`f690c5f`](https://github.com/juspay/hyperswitch/commit/f690c5f3ead64c353ac1d36401e009582c1f0ecf))

### Performance

- **db:** Add index for attempt_id merchant_id ([#1993](https://github.com/juspay/hyperswitch/pull/1993)) ([`57d22b9`](https://github.com/juspay/hyperswitch/commit/57d22b966b911ee8948440072bf9ce23dbd21dd3))

### Refactors

- **core:** Made authenticate_client_secret function public ([#1992](https://github.com/juspay/hyperswitch/pull/1992)) ([`6986772`](https://github.com/juspay/hyperswitch/commit/698677263be56c4ad16cbf90f5607623a18e3d8b))

**Full Changelog:** [`v1.25.1...v1.26.0`](https://github.com/juspay/hyperswitch/compare/v1.25.1...v1.26.0)

- - -


## 1.25.1 (2023-08-22)

### Bug Fixes

- Storage of generic payment methods in permanent locker ([#1799](https://github.com/juspay/hyperswitch/pull/1799)) ([`19ee324`](https://github.com/juspay/hyperswitch/commit/19ee324d373262aea873bb7a120f0e80b918fd84))

**Full Changelog:** [`v1.25.0...v1.25.1`](https://github.com/juspay/hyperswitch/compare/v1.25.0...v1.25.1)

- - -


## 1.25.0 (2023-08-22)

### Features

- **storage_impl:** Split payment intent interface implementation ([#1946](https://github.com/juspay/hyperswitch/pull/1946)) ([`88d65a6`](https://github.com/juspay/hyperswitch/commit/88d65a62fc81f217ade71b2d4903d3bbe85e5c94))

### Bug Fixes

- **core:** Update Webhooks Event Mapping and Forced Psync preconditions ([#1970](https://github.com/juspay/hyperswitch/pull/1970)) ([`8cf1f75`](https://github.com/juspay/hyperswitch/commit/8cf1f75fb1705aa020db5f966e15c3d9a80dd908))

**Full Changelog:** [`v1.24.0...v1.25.0`](https://github.com/juspay/hyperswitch/compare/v1.24.0...v1.25.0)

- - -


## 1.24.0 (2023-08-21)

### Features

- **router:** Add total count for refunds list ([#1935](https://github.com/juspay/hyperswitch/pull/1935)) ([`84967d3`](https://github.com/juspay/hyperswitch/commit/84967d396e628d4cc256ff86d82145c478a91422))

### Bug Fixes

- **typo:** Add typo `daa` to allow list ([#1968](https://github.com/juspay/hyperswitch/pull/1968)) ([`875dbce`](https://github.com/juspay/hyperswitch/commit/875dbce927d86384dd41c2e900ae8074f9540b75))

**Full Changelog:** [`v1.23.0...v1.24.0`](https://github.com/juspay/hyperswitch/compare/v1.23.0...v1.24.0)

- - -


## 1.23.0 (2023-08-18)

### Features

- **business_profile:** Add business profile table and CRUD endpoints ([#1928](https://github.com/juspay/hyperswitch/pull/1928)) ([`53956d6`](https://github.com/juspay/hyperswitch/commit/53956d6f8379f90e4070b49bd2322950aa11a7f2))

### Bug Fixes

- **connector:** [CashToCode] Transform minor units to major units ([#1964](https://github.com/juspay/hyperswitch/pull/1964)) ([`ff2efe8`](https://github.com/juspay/hyperswitch/commit/ff2efe88357a253a22bb8467136717b7809218b6))
- **payment_methods:** Return parent_payment_method_token for other payment methods (BankTransfer, Wallet, BankRedirect)  ([#1951](https://github.com/juspay/hyperswitch/pull/1951)) ([`156430a`](https://github.com/juspay/hyperswitch/commit/156430a5703f40b6bb899caf9904323e39003986))

### Refactors

- **compatibility:** Changed MCA decode 500 error to 422 ([#1958](https://github.com/juspay/hyperswitch/pull/1958)) ([`0d85c1f`](https://github.com/juspay/hyperswitch/commit/0d85c1f8bb3e7d0e1d359d737a1e8a2f0d7885d2))

**Full Changelog:** [`v1.22.0...v1.23.0`](https://github.com/juspay/hyperswitch/compare/v1.22.0...v1.23.0)

- - -


## 1.22.0 (2023-08-18)

### Features

- **router:** Send 2xx payments response for all the connector http responses (2xx, 4xx etc.) ([#1924](https://github.com/juspay/hyperswitch/pull/1924)) ([`0ab6827`](https://github.com/juspay/hyperswitch/commit/0ab6827f6cf54b0a124856487f5359b91048736c))

### Bug Fixes

- **connector:** [Payme] Fix for partial capture validation ([#1939](https://github.com/juspay/hyperswitch/pull/1939)) ([`3d62cb0`](https://github.com/juspay/hyperswitch/commit/3d62cb07dd94d827b18e664a3454352f300575fe))

**Full Changelog:** [`v1.21.2...v1.22.0`](https://github.com/juspay/hyperswitch/compare/v1.21.2...v1.22.0)

- - -


## 1.21.2 (2023-08-17)

### Bug Fixes

- **connector:** [Braintree] fix status mapping for braintree ([#1941](https://github.com/juspay/hyperswitch/pull/1941)) ([`d30fefb`](https://github.com/juspay/hyperswitch/commit/d30fefb2c08d4a086f4d8c0519196d83fa228d45))
- **frm:** Added fraud_check_last_step field in fraud_check table to support 3DS transaction in frm ([#1944](https://github.com/juspay/hyperswitch/pull/1944)) ([`9a39345`](https://github.com/juspay/hyperswitch/commit/9a393455dd6643caf61747633698191ba8c59d49))

### Refactors

- **connector:** Remove payment experience from Not Supported Payment Methods error ([#1937](https://github.com/juspay/hyperswitch/pull/1937)) ([`c5cf029`](https://github.com/juspay/hyperswitch/commit/c5cf029d1f20dc27f6b246094d61a381669feb68))

**Full Changelog:** [`v1.21.1...v1.21.2`](https://github.com/juspay/hyperswitch/compare/v1.21.1...v1.21.2)

- - -


## 1.21.1 (2023-08-15)

### Bug Fixes

- **connector:** [Braintree] add merchant_account_id field in authorize request ([#1916](https://github.com/juspay/hyperswitch/pull/1916)) ([`68df9d6`](https://github.com/juspay/hyperswitch/commit/68df9d617c825e9a4fec88695c3c22588cf3673b))

### Refactors

- **storage_impl:** Integrate the composite store from external crate ([#1921](https://github.com/juspay/hyperswitch/pull/1921)) ([`9f199d9`](https://github.com/juspay/hyperswitch/commit/9f199d9ab8fb7360bda2661a7014aea8906b74f9))

### Documentation

- Documentation changes for clarity ([#1875](https://github.com/juspay/hyperswitch/pull/1875)) ([`b1e4e38`](https://github.com/juspay/hyperswitch/commit/b1e4e3883d4d039c3ed06272d984526da0e657af))

**Full Changelog:** [`v1.21.0...v1.21.1`](https://github.com/juspay/hyperswitch/compare/v1.21.0...v1.21.1)

- - -


## 1.21.0 (2023-08-14)

### Features

- **generics:** Add metrics for database calls ([#1901](https://github.com/juspay/hyperswitch/pull/1901)) ([`bb6ec49`](https://github.com/juspay/hyperswitch/commit/bb6ec49a66bc9380ff0f5eca44cad381b7dc4368))

### Bug Fixes

- **frm:** Add new column frm_config instead of alterning the existing… ([#1925](https://github.com/juspay/hyperswitch/pull/1925)) ([`8d916fe`](https://github.com/juspay/hyperswitch/commit/8d916feb3fe9fd5dd843cb6a4dbc29f5807aa205))
- Add diesel migration to update local db ([#1812](https://github.com/juspay/hyperswitch/pull/1812)) ([`97a495c`](https://github.com/juspay/hyperswitch/commit/97a495cfa700835fd2dbf4f4be1b404a1e4a264a))

### Refactors

- **storage:** Add redis structs to storage impls ([#1910](https://github.com/juspay/hyperswitch/pull/1910)) ([`3e26966`](https://github.com/juspay/hyperswitch/commit/3e269663c36c8a9f11108d01f96bd612f318cc15))

**Full Changelog:** [`v1.20.0...v1.21.0`](https://github.com/juspay/hyperswitch/compare/v1.20.0...v1.21.0)

- - -


## 1.20.0 (2023-08-11)

### Features

- **connector:** [PayMe] Implement preprocessing flow for cards ([#1904](https://github.com/juspay/hyperswitch/pull/1904)) ([`38b9c07`](https://github.com/juspay/hyperswitch/commit/38b9c077b7cd9563aaf3f39876670df7484f519d))
- **router:** Add webhook source verification support for multiple mca of the same connector ([#1897](https://github.com/juspay/hyperswitch/pull/1897)) ([`3554fec`](https://github.com/juspay/hyperswitch/commit/3554fec1c1ab6084480600c73fbefe39085723e0))

### Bug Fixes

- **connector:**
  - [STAX] Add currency filter for payments through Stax ([#1911](https://github.com/juspay/hyperswitch/pull/1911)) ([`5bc7592`](https://github.com/juspay/hyperswitch/commit/5bc7592af3c8587a402809c050e58b257b7af8bf))
  - [Paypal] send valid error_reason in all the error responses ([#1914](https://github.com/juspay/hyperswitch/pull/1914)) ([`3df9441`](https://github.com/juspay/hyperswitch/commit/3df944196f710587eee32be871eaef1d764b694a))
- **payment_methods:** Delete token when a payment reaches terminal state ([#1818](https://github.com/juspay/hyperswitch/pull/1818)) ([`07020d0`](https://github.com/juspay/hyperswitch/commit/07020d01b5d08d9ba5a146d62fbb8c23c6a6d3c2))

### Refactors

- **storage:** Add a separate crate to represent store implementations ([#1853](https://github.com/juspay/hyperswitch/pull/1853)) ([`32b731d`](https://github.com/juspay/hyperswitch/commit/32b731d9591ff4921b7d80556c7ebe050b53121f))

### Miscellaneous Tasks

- **webhooks:** Ignore payment not found in webhooks ([#1886](https://github.com/juspay/hyperswitch/pull/1886)) ([`29f068b`](https://github.com/juspay/hyperswitch/commit/29f068b20581fca280be9a1a98524368d635191f))

**Full Changelog:** [`v1.19.0...v1.20.0`](https://github.com/juspay/hyperswitch/compare/v1.19.0...v1.20.0)

- - -


## 1.19.0 (2023-08-10)

### Features

- **connector:** [Adyen] implement Japanese convenience stores ([#1819](https://github.com/juspay/hyperswitch/pull/1819)) ([`a6fdf6d`](https://github.com/juspay/hyperswitch/commit/a6fdf6dc34901a9985062fd5532d967910bcf3c0))
- **docs:** Add multiple examples support and webhook schema ([#1864](https://github.com/juspay/hyperswitch/pull/1864)) ([`f8ef52c`](https://github.com/juspay/hyperswitch/commit/f8ef52c645d353aac438d6af5b00d9097332fdcb))

### Bug Fixes

- **connector:**
  - [ACI] Response Handling in case of `ErrorResponse` ([#1870](https://github.com/juspay/hyperswitch/pull/1870)) ([`14f599d`](https://github.com/juspay/hyperswitch/commit/14f599d1be272afcfd16dfac58c47dbbb649423d))
  - [Adyen] Response Handling in case of RefusalResponse ([#1877](https://github.com/juspay/hyperswitch/pull/1877)) ([`c35a571`](https://github.com/juspay/hyperswitch/commit/c35a5719eb08ff76a10d554a0e61d0af81ff26e6))
- **router:** Handle JSON connector response parse error ([#1892](https://github.com/juspay/hyperswitch/pull/1892)) ([`393c2ab`](https://github.com/juspay/hyperswitch/commit/393c2ab94cf1052f6f8fa0b40c09e36555ffecd7))

### Refactors

- **connector:** Update the `connector_template`  ([#1895](https://github.com/juspay/hyperswitch/pull/1895)) ([`5fe96d4`](https://github.com/juspay/hyperswitch/commit/5fe96d4d9683d8eae25f214f3823d3765dce326a))
- Remove unnecessary debug logs from payment method list api ([#1884](https://github.com/juspay/hyperswitch/pull/1884)) ([`ba82f17`](https://github.com/juspay/hyperswitch/commit/ba82f173dbccfc2312677ec96fdd85813a417dc6))

### Documentation

- Add architecture and monitoring diagram of hyperswitch ([#1825](https://github.com/juspay/hyperswitch/pull/1825)) ([`125ef2b`](https://github.com/juspay/hyperswitch/commit/125ef2b4f82c922209bcfe161ce4790fe2ee3a86))

### Miscellaneous Tasks

- **configs:** Add `payout_connector_list` config to toml ([#1909](https://github.com/juspay/hyperswitch/pull/1909)) ([`c1e5626`](https://github.com/juspay/hyperswitch/commit/c1e56266df6aabd1c498d6a7ebec324b0df23c12))
- Add connector functionality validation based on connector_type ([#1849](https://github.com/juspay/hyperswitch/pull/1849)) ([`33c6d71`](https://github.com/juspay/hyperswitch/commit/33c6d71a8a71619f811accbc21f3c22c3c279c47))
- Remove spaces at beginning of commit messages when generating changelogs ([#1906](https://github.com/juspay/hyperswitch/pull/1906)) ([`7d13226`](https://github.com/juspay/hyperswitch/commit/7d13226740dbc4c1b6ec19631bb93ba89281d303))

**Full Changelog:** [`v1.18.0...v1.19.0`](https://github.com/juspay/hyperswitch/compare/v1.18.0...v1.19.0)

- - -


## 1.18.0 (2023-08-09)

### Features

- **connector:**
  - [Adyen] Add support for card redirection (KNET, BENEFIT) ([#1816](https://github.com/juspay/hyperswitch/pull/1816)) ([`62461f1`](https://github.com/juspay/hyperswitch/commit/62461f1b3849bfde3d0c0608b9efd96334e30f97))
  - [Checkout] unify error code, message and reason in error response ([#1855](https://github.com/juspay/hyperswitch/pull/1855)) ([`e8a51c2`](https://github.com/juspay/hyperswitch/commit/e8a51c2abeaead3a78ec7fbe9580cf742f7dfbe3))
  - Unified error message & errorCode for blueSnap connector ([#1856](https://github.com/juspay/hyperswitch/pull/1856)) ([`222afee`](https://github.com/juspay/hyperswitch/commit/222afee5d5e18132ae40509fb792d6fd13600069))
  - [Adyen] Implement Open Banking Uk in Bank Redirects ([#1802](https://github.com/juspay/hyperswitch/pull/1802)) ([`b9f1270`](https://github.com/juspay/hyperswitch/commit/b9f12708e108c3ac691314d32b7976d7e381eee7))
  - [Adyen] Implement Momo Atm(Napas) in Card Redirects ([#1820](https://github.com/juspay/hyperswitch/pull/1820)) ([`8ae6737`](https://github.com/juspay/hyperswitch/commit/8ae67377cca506b4d7017bfd167a5ccdb03e8707))
  - [Stax] Implement Bank Debits and Webhooks for Connector Stax ([#1832](https://github.com/juspay/hyperswitch/pull/1832)) ([`0f2bb6c`](https://github.com/juspay/hyperswitch/commit/0f2bb6c09bb929a7274af6049ecff5a5f9049ca1))
- **pm_list:** Add pm required field info for crypto pay ([#1891](https://github.com/juspay/hyperswitch/pull/1891)) ([`c205f06`](https://github.com/juspay/hyperswitch/commit/c205f064b91df483cbf0fb4d581d8908bf8fa673))
- **router:** Add support for multiple partial capture ([#1721](https://github.com/juspay/hyperswitch/pull/1721)) ([`c333fb7`](https://github.com/juspay/hyperswitch/commit/c333fb7fc02cf19d74ca80093552e4c4628f248a))

### Bug Fixes

- **router:**
  - Add `serde(transparent)` annotation for `PaymentMethodMetadata` ([#1899](https://github.com/juspay/hyperswitch/pull/1899)) ([`2d83917`](https://github.com/juspay/hyperswitch/commit/2d839170fe889051772f5d99cdaff33573b4fb20))
  - Send error_reason as error_message in payments and refund flows ([#1878](https://github.com/juspay/hyperswitch/pull/1878)) ([`6982194`](https://github.com/juspay/hyperswitch/commit/69821948c0a31b224e1b519388071b66c0d67eb1))

### Refactors

- **access_token:** Handle timeout errors gracefully ([#1882](https://github.com/juspay/hyperswitch/pull/1882)) ([`cc4136f`](https://github.com/juspay/hyperswitch/commit/cc4136f85f0a64b56c4a09157f9bc4847b920b54))
- **authorize_flow:** Suppress error while saving a card to locker after successful payment ([#1874](https://github.com/juspay/hyperswitch/pull/1874)) ([`3cc4548`](https://github.com/juspay/hyperswitch/commit/3cc4548eee4289455da99de2bf54c6b312291374))

### Testing

- **connector:** Add support for webhook tests  ([#1863](https://github.com/juspay/hyperswitch/pull/1863)) ([`7b2c419`](https://github.com/juspay/hyperswitch/commit/7b2c419ce5c8f429dad3ace852891f76d2281646))

**Full Changelog:** [`v1.17.1...v1.18.0`](https://github.com/juspay/hyperswitch/compare/v1.17.1...v1.18.0)

- - -


## 1.17.1 (2023-08-07)

### Bug Fixes

- **connector:** [DummyConnector] add new icons and fix `we_chat_pay` ([#1845](https://github.com/juspay/hyperswitch/pull/1845)) ([`985ff6b`](https://github.com/juspay/hyperswitch/commit/985ff6ba419b6ed13fc9e2f74dfa824a27bdd3e3))
- **kms:** Fix kms decryption for jwe keys ([#1872](https://github.com/juspay/hyperswitch/pull/1872)) ([`ddc0302`](https://github.com/juspay/hyperswitch/commit/ddc0302298aefab0860b49210ce73abd4d121fb9))

### Revert

- Ci: use `sccache-action` for caching compilation artifacts ([#1880](https://github.com/juspay/hyperswitch/pull/1880)) ([`a988018`](https://github.com/juspay/hyperswitch/commit/a988018350dccebe94b4cac66b54375b95fcbbbe))

**Full Changelog:** [`v1.17.0...v1.17.1`](https://github.com/juspay/hyperswitch/compare/v1.17.0...v1.17.1)

- - -


## 1.17.0 (2023-08-07)

### Features

- **config:** Add config support to pt_mapping along with redis ([#1861](https://github.com/juspay/hyperswitch/pull/1861)) ([`b03dd24`](https://github.com/juspay/hyperswitch/commit/b03dd244561641f5b3481c79035766561bcd0a8a))
- **connector:** [Payme] Add Sync, RSync & webhook flow support ([#1862](https://github.com/juspay/hyperswitch/pull/1862)) ([`8057980`](https://github.com/juspay/hyperswitch/commit/80579805f9dd7c387eb3c0b5c48e01fa69e48299))

### Bug Fixes

- **core:** If frm is not called, send None in frm_message instead of initial values in update tracker ([#1867](https://github.com/juspay/hyperswitch/pull/1867)) ([`3250204`](https://github.com/juspay/hyperswitch/commit/3250204acc1e32f92dad725378b19dd3e4da33f6))

### Revert

- Fix(core): add validation for all the connector auth_type ([#1833](https://github.com/juspay/hyperswitch/pull/1833)) ([`ae3d25e`](https://github.com/juspay/hyperswitch/commit/ae3d25e6899af0d78171d40c980146d58f8fc03f))

**Full Changelog:** [`v1.16.0...v1.17.0`](https://github.com/juspay/hyperswitch/compare/v1.16.0...v1.17.0)

- - -


## 1.16.0 (2023-08-04)

### Features

- **connector:**
  - [Adyen] implement PaySafe ([#1805](https://github.com/juspay/hyperswitch/pull/1805)) ([`0f09199`](https://github.com/juspay/hyperswitch/commit/0f0919963fd1c887d3315039420a939bb377e738))
  - [Adyen] Add support for gift cards balance ([#1672](https://github.com/juspay/hyperswitch/pull/1672)) ([`c4796ff`](https://github.com/juspay/hyperswitch/commit/c4796ffdb77a6270e7abc2e65e142ee4e7639b54))
  - [Square] Add template code for connector Square ([#1834](https://github.com/juspay/hyperswitch/pull/1834)) ([`80b74e0`](https://github.com/juspay/hyperswitch/commit/80b74e096d56e08685ad52fb3049f6b611d587b3))
  - [Adyen] implement Oxxo ([#1808](https://github.com/juspay/hyperswitch/pull/1808)) ([`5ed3f34`](https://github.com/juspay/hyperswitch/commit/5ed3f34c24c82d182921d317361bc9fc72be58ce))

### Bug Fixes

- **webhooks:** Do not send duplicate webhooks  ([#1850](https://github.com/juspay/hyperswitch/pull/1850)) ([`0d996b8`](https://github.com/juspay/hyperswitch/commit/0d996b8960c7445289e451744c4bdeeb87d7d567))

### Refactors

- **connector:** Use utility function to raise payment method not implemented errors ([#1847](https://github.com/juspay/hyperswitch/pull/1847)) ([`f2fcc25`](https://github.com/juspay/hyperswitch/commit/f2fcc2595ae6f1c0ac5553c1a21ab33a6078b3e2))
- **payment_methods:** Add `requires_cvv` field to customer payment method list api object ([#1852](https://github.com/juspay/hyperswitch/pull/1852)) ([`2dec2ca`](https://github.com/juspay/hyperswitch/commit/2dec2ca50bbac0eed6f9fc562662b86436b4b656))

**Full Changelog:** [`v1.15.0...v1.16.0`](https://github.com/juspay/hyperswitch/compare/v1.15.0...v1.16.0)

- - -


## 1.15.0 (2023-08-03)

### Features

- **connector:**
  - [Boku] Implement Authorize, Psync, Refund and Rsync flow ([#1699](https://github.com/juspay/hyperswitch/pull/1699)) ([`9cba7da`](https://github.com/juspay/hyperswitch/commit/9cba7da0d3d4b87101debef8ec25b52a908975c5))
  - add support for bank redirect for Paypal ([#1107](https://github.com/juspay/hyperswitch/pull/1107)) ([`57887bd`](https://github.com/juspay/hyperswitch/commit/57887bdf3a892548afea80859c2553d5a1cca49d))
  - [Adyen] implement Adyen bank transfers and voucher payments in Indonesia   ([#1804](https://github.com/juspay/hyperswitch/pull/1804)) ([`9977f9d`](https://github.com/juspay/hyperswitch/commit/9977f9d40ea349cada6171af7166a533e694450f))
  - Unified errorCode and errorMessage map error reason as errorMessage in Stripe Connector ([#1797](https://github.com/juspay/hyperswitch/pull/1797)) ([`c464cc5`](https://github.com/juspay/hyperswitch/commit/c464cc510ded595ea846e7da95f60919614e2bd3))

### Refactors

- **common_enums:** Added derive for additional traits in FutureU… ([#1848](https://github.com/juspay/hyperswitch/pull/1848)) ([`8f6583f`](https://github.com/juspay/hyperswitch/commit/8f6583fbeeb7ab7ac31566adf9d182a839ed9a51))
- **config:** Add new type for kms encrypted values ([#1823](https://github.com/juspay/hyperswitch/pull/1823)) ([`73ed7ae`](https://github.com/juspay/hyperswitch/commit/73ed7ae7e305c391f413e3ac88775148db304779))

**Full Changelog:** [`v1.14.1...v1.15.0`](https://github.com/juspay/hyperswitch/compare/v1.14.1...v1.15.0)

- - -


## 1.14.1 (2023-08-02)

### Bug Fixes

- Include merchant reference in CreateIntentRequest ([#1846](https://github.com/juspay/hyperswitch/pull/1846)) ([`db55ed0`](https://github.com/juspay/hyperswitch/commit/db55ed0f6dcb2442784da5d38d76810541c95051))

**Full Changelog:** [`v1.14.0...v1.14.1`](https://github.com/juspay/hyperswitch/compare/v1.14.0...v1.14.1)

- - -


## 1.14.0 (2023-08-02)

### Features

- **Connector:** [Stripe] Implement Cashapp Wallet  ([#1103](https://github.com/juspay/hyperswitch/pull/1103)) ([`dadd13e`](https://github.com/juspay/hyperswitch/commit/dadd13e3819095273e710a1c6ba6e5f2fef2ed7e))
- **connector:**
  - [iatapay] fix refund amount, hardcode IN for UPI, send merchant payment id ([#1824](https://github.com/juspay/hyperswitch/pull/1824)) ([`505aa21`](https://github.com/juspay/hyperswitch/commit/505aa218cf2b417929a7e2caaa8d820b5a68fe75))
  - [Adyen] implement Swish for Adyen ([#1701](https://github.com/juspay/hyperswitch/pull/1701)) ([`cf30255`](https://github.com/juspay/hyperswitch/commit/cf3025562ffdb9cbab77fe40795051faad750fd5))
  - [Trustpay] unify error_code, error_message and error_reason in error response ([#1817](https://github.com/juspay/hyperswitch/pull/1817)) ([`8a638e4`](https://github.com/juspay/hyperswitch/commit/8a638e4a089c772cd53742fa48f22f4bf8585c79))
  - [Stax] Implement Cards for Connector Stax ([#1773](https://github.com/juspay/hyperswitch/pull/1773)) ([`f492d0a`](https://github.com/juspay/hyperswitch/commit/f492d0a943ed57aadc7abed721f90ed9e19e0c88))
  - [Adyen] Implement Boleto Bancario in Vouchers and Add support for Voucher in Next Action ([#1657](https://github.com/juspay/hyperswitch/pull/1657)) ([`801946f`](https://github.com/juspay/hyperswitch/commit/801946f29f5701e3018f7fd54d3b3d0b4a13bc8e))
  - [Adyen] Add support for Blik ([#1727](https://github.com/juspay/hyperswitch/pull/1727)) ([`30e41a9`](https://github.com/juspay/hyperswitch/commit/30e41a9f2f73fa7406696c6bf3bb6b4a38c24405))
- **core:** Added key should_cancel_transaction in update trackers to support Frm Pre flow cancellation ([#1811](https://github.com/juspay/hyperswitch/pull/1811)) ([`5d6510e`](https://github.com/juspay/hyperswitch/commit/5d6510eddf71b574f8d36743a56d1e6236af0bef))
- **payment_methods:** Added value Field in required Field for Pre-filling ([#1827](https://github.com/juspay/hyperswitch/pull/1827)) ([`e047a11`](https://github.com/juspay/hyperswitch/commit/e047a11dedbceaf9778a0f4aed1f9658f4af6783))
- **pii:** Implement a masking strategy for UPI VPAs ([#1641](https://github.com/juspay/hyperswitch/pull/1641)) ([`e3a33bb`](https://github.com/juspay/hyperswitch/commit/e3a33bb5c281ddf9c5746fad485bffa274a48b44))

### Bug Fixes

- **connector:**
  - [Stripe] change payment_method name Wechatpay to wechatpayqr ([#1813](https://github.com/juspay/hyperswitch/pull/1813)) ([`208d619`](https://github.com/juspay/hyperswitch/commit/208d619409ee03b7115b7c6268457df12149bee1))
  - Refactor capture and refund flow for Connectors ([#1821](https://github.com/juspay/hyperswitch/pull/1821)) ([`d06adc7`](https://github.com/juspay/hyperswitch/commit/d06adc705c7c92307cdf3dd63b41c5ee1583a189))
  - [Payme] Fix refund request fields ([#1831](https://github.com/juspay/hyperswitch/pull/1831)) ([`6f8be0c`](https://github.com/juspay/hyperswitch/commit/6f8be0c675cb55237da9deffb857dc4958fb6828))
  - [Airwallex] Psync response ([#1826](https://github.com/juspay/hyperswitch/pull/1826)) ([`8f65819`](https://github.com/juspay/hyperswitch/commit/8f65819f1265577c9886f9c14ddfe16f2318d3d5))
  - Refactor psync and rsync for connectors ([#1830](https://github.com/juspay/hyperswitch/pull/1830)) ([`7a0d6f6`](https://github.com/juspay/hyperswitch/commit/7a0d6f69211a44d4e362fe0857cdda2ff5167f0a))
- **payments:**
  - All AdditionalCardInfo fields optional ([#1840](https://github.com/juspay/hyperswitch/pull/1840)) ([`a1cb255`](https://github.com/juspay/hyperswitch/commit/a1cb255765394e7c91aa33bea72b2e48b597b443))
  - Write a foreign_from implementation for payment_method_data and add missing payment methods in helpers.rs ([#1801](https://github.com/juspay/hyperswitch/pull/1801)) ([`50298c1`](https://github.com/juspay/hyperswitch/commit/50298c19674cf75fe6a6aee4fa099a4885902357))
- **ui-tests:**
  - Run ui-tests for each PR on approval ([#1839](https://github.com/juspay/hyperswitch/pull/1839)) ([`f2b370f`](https://github.com/juspay/hyperswitch/commit/f2b370f2855ccd77604fe73526a7edef81a90a47))
  - Allow ui tests on workflow dispatch ([#1843](https://github.com/juspay/hyperswitch/pull/1843)) ([`c9fd421`](https://github.com/juspay/hyperswitch/commit/c9fd421d09db5746e2a21a8132813d8e2bf5ec35))
- Request amount fix for trustpay apple pay ([#1837](https://github.com/juspay/hyperswitch/pull/1837)) ([`3da69f3`](https://github.com/juspay/hyperswitch/commit/3da69f3ee160b022a3e2cf64c78833eb3fd95aea))

### Refactors

- **multiple_mca:** Make `primary_business_detail` optional and remove default values ([#1677](https://github.com/juspay/hyperswitch/pull/1677)) ([`9c7ac62`](https://github.com/juspay/hyperswitch/commit/9c7ac6246d6cf434855bc61f7cd625101665de5c))
- **redis:** Invoke `redis_conn()` method instead of cloning `redis_conn` property in `StorageInterface` ([#1552](https://github.com/juspay/hyperswitch/pull/1552)) ([`f32fdec`](https://github.com/juspay/hyperswitch/commit/f32fdec290a2f303887550d8db1ae2a3c065bafe))
- **router:** Include currency conversion utility functions as `Currency` methods ([#1790](https://github.com/juspay/hyperswitch/pull/1790)) ([`2c9c8f0`](https://github.com/juspay/hyperswitch/commit/2c9c8f081d7a99574dacae471ca2996ea2b2aa44))
- **ui_tests:** Move ui_tests to test_utils crate to reduce development time ([#1822](https://github.com/juspay/hyperswitch/pull/1822)) ([`5773faf`](https://github.com/juspay/hyperswitch/commit/5773faf739f1525cfe442c2df9d33f7475cf6b7c))

**Full Changelog:** [`v1.13.2...v1.14.0`](https://github.com/juspay/hyperswitch/compare/v1.13.2...v1.14.0)

- - -


## 1.13.2 (2023-08-01)

### Bug Fixes

- **webhook:** Provide acknowledgment for webhooks with unsupported event types ([#1815](https://github.com/juspay/hyperswitch/pull/1815)) ([`28a371b`](https://github.com/juspay/hyperswitch/commit/28a371b24a590787a569f08d84149515b46ebda6))

**Full Changelog:** [`v1.13.1...v1.13.2`](https://github.com/juspay/hyperswitch/compare/v1.13.1...v1.13.2)

- - -


## 1.13.1 (2023-07-31)

### Bug Fixes

- **connector:** [Trustpay] send billing address name as cardholder name ([#1806](https://github.com/juspay/hyperswitch/pull/1806)) ([`71b75c6`](https://github.com/juspay/hyperswitch/commit/71b75c653845685b71c6fb6007a718b6cb2c65c5))
- **logs:** Remove request from logs ([#1810](https://github.com/juspay/hyperswitch/pull/1810)) ([`5ad3950`](https://github.com/juspay/hyperswitch/commit/5ad3950892fc0c84b26092b0732dd18d2d913d12))

### Testing

- **connector:** Refactor UI test for connectors ([#1807](https://github.com/juspay/hyperswitch/pull/1807)) ([`34ff408`](https://github.com/juspay/hyperswitch/commit/34ff4080aeb4e8dacdeb13f2b5c17d8ead9561c8))

**Full Changelog:** [`v1.13.0...v1.13.1`](https://github.com/juspay/hyperswitch/compare/v1.13.0...v1.13.1)

- - -


## 1.13.0 (2023-07-28)

### Features

- **dummy_connector:** Add 3DS Flow, Wallets and Pay Later for Dummy Connector ([#1781](https://github.com/juspay/hyperswitch/pull/1781)) ([`8186c77`](https://github.com/juspay/hyperswitch/commit/8186c778bddb8932b37e5cf4c7b3e2d507f73e89))
- **router:** Validate payment method type in payments request against given payment method data for non-card flows ([#1236](https://github.com/juspay/hyperswitch/pull/1236)) ([`7607b6b`](https://github.com/juspay/hyperswitch/commit/7607b6b67153fce1e965d7ef7e41c62380884d8f))

### Bug Fixes

- **Connector:** [Noon] Update ApplePay Payment Struct ([#1794](https://github.com/juspay/hyperswitch/pull/1794)) ([`b96687c`](https://github.com/juspay/hyperswitch/commit/b96687c3fa863af76afef68170ee2c59946b76fd))
- **router:** Add validation for all the connector auth type ([#1748](https://github.com/juspay/hyperswitch/pull/1748)) ([`1cda7ad`](https://github.com/juspay/hyperswitch/commit/1cda7ad5fccb64c1adefc24a47b79b8315f91a59))

### Documentation

- Add renewed links for readme ([#1796](https://github.com/juspay/hyperswitch/pull/1796)) ([`e06e62c`](https://github.com/juspay/hyperswitch/commit/e06e62cc75497cb245fa115bb718a29c31e577c5))

**Full Changelog:** [`v1.12.0...v1.13.0`](https://github.com/juspay/hyperswitch/compare/v1.12.0...v1.13.0)

- - -


## 1.12.0 (2023-07-27)

### Features

- **connector:** [Zen] Add Latam Payment Methods ([#1670](https://github.com/juspay/hyperswitch/pull/1670)) ([`4df67ad`](https://github.com/juspay/hyperswitch/commit/4df67adb9bb110f1c5f3fc094fe21bf4741cda46))
- **core:** Changed frm_config format type in merchant_connector_account and added frm_message in payments response ([#1543](https://github.com/juspay/hyperswitch/pull/1543)) ([`c284f41`](https://github.com/juspay/hyperswitch/commit/c284f41cc685b4a5093be12ec4b5e4b503de82b5))
- **errors:** Add `GenericDuplicateError` in`ApiErrorResponse` ([#1792](https://github.com/juspay/hyperswitch/pull/1792)) ([`7f94716`](https://github.com/juspay/hyperswitch/commit/7f947169feac9d15616cc2b1a2aacdfa80f219bf))
- **router:**
  - Add grouping and priority logic in connector utils to handle multiple errors in connector flows ([#1765](https://github.com/juspay/hyperswitch/pull/1765)) ([`e6a5e9f`](https://github.com/juspay/hyperswitch/commit/e6a5e9fa72d28c7b0031aa23817ae234e8f81da0))
  - Apply filters on payments ([#1744](https://github.com/juspay/hyperswitch/pull/1744)) ([`04c3de7`](https://github.com/juspay/hyperswitch/commit/04c3de73a51060ab567a4b53dce678020bcc7dfa))
- Api contract for gift cards ([#1634](https://github.com/juspay/hyperswitch/pull/1634)) ([`8369626`](https://github.com/juspay/hyperswitch/commit/836962677b955bbe761d6c18596cbb964d8e83ad))

### Bug Fixes

- **connector:**
  - [Powertranz] Fix response handling for https status code other than 200 ([#1775](https://github.com/juspay/hyperswitch/pull/1775)) ([`4805a94`](https://github.com/juspay/hyperswitch/commit/4805a94ab905da520edacdddab41e9e74bd3a956))
  - [Klarna] Handle error response with both error_messages and error_message fields ([#1783](https://github.com/juspay/hyperswitch/pull/1783)) ([`9cfdce0`](https://github.com/juspay/hyperswitch/commit/9cfdce0abe8a0c6ded458cdd4b07a8cb4098e504))
- **router:** Add manual retry flag in Re-direction url ([#1791](https://github.com/juspay/hyperswitch/pull/1791)) ([`20f6644`](https://github.com/juspay/hyperswitch/commit/20f664408ac1e3ee795ee26b128380185e8fc2f0))

### Refactors

- **core:** Use secrets for connector AuthType in connector integration ([#1441](https://github.com/juspay/hyperswitch/pull/1441)) ([`d068569`](https://github.com/juspay/hyperswitch/commit/d068569f4debe25ee94802b29b4765d473891547))

### Revert

- Feat(connector): [Adyen] Add pix support for adyen ([#1795](https://github.com/juspay/hyperswitch/pull/1795)) ([`38f14b9`](https://github.com/juspay/hyperswitch/commit/38f14b9f39370e89e0176d8e0255f8fcb624efca))

**Full Changelog:** [`v1.11.0...v1.12.0`](https://github.com/juspay/hyperswitch/compare/v1.11.0...v1.12.0)

- - -


## 1.11.0 (2023-07-26)

### Features

- **compatibility:** Add wallet mandate support setup intent and connector_metadata field ([#1767](https://github.com/juspay/hyperswitch/pull/1767)) ([`af9a458`](https://github.com/juspay/hyperswitch/commit/af9a4585b26b278ffb298d4e8de13479da447d5f))
- **connector:**
  - [Boku] Template generation ([#1760](https://github.com/juspay/hyperswitch/pull/1760)) ([`78c6cce`](https://github.com/juspay/hyperswitch/commit/78c6ccea2ef88dcd02d74d173021a1e57092e1b7))
  - [Stripe, Adyen, Checkout] Add reference ID support for retries ([#1735](https://github.com/juspay/hyperswitch/pull/1735)) ([`9ba8ec3`](https://github.com/juspay/hyperswitch/commit/9ba8ec348b1e377521386d751c2a924ad843ce8d))
  - [Adyen] Add pix support for adyen ([#1703](https://github.com/juspay/hyperswitch/pull/1703)) ([`33a1368`](https://github.com/juspay/hyperswitch/commit/33a1368e8a0961610d652f5a6834ba37b995582a))
- **db:** Implement `MerchantKeyStoreInterface` for `MockDb` ([#1772](https://github.com/juspay/hyperswitch/pull/1772)) ([`f3baf2f`](https://github.com/juspay/hyperswitch/commit/f3baf2ff3f0a50a5558316625ade647e7607d6c2))
- **macro:** Add config validation macro for connectors ([#1755](https://github.com/juspay/hyperswitch/pull/1755)) ([`37a0651`](https://github.com/juspay/hyperswitch/commit/37a06516603e3c8d3e7cf367530266c055a6cb0a))
- **router:** Add merchant_id check for manual_retry_allowed flag sent in payments response ([#1785](https://github.com/juspay/hyperswitch/pull/1785)) ([`435c939`](https://github.com/juspay/hyperswitch/commit/435c9395762428843699f001c0c8f80489c662ad))

### Bug Fixes

- **connector:**
  - [Bluesnap] Populate Error Reason and Update error handling ([#1787](https://github.com/juspay/hyperswitch/pull/1787)) ([`5c6bcb5`](https://github.com/juspay/hyperswitch/commit/5c6bcb594eca050c2abbd3cc622c7e2d527b31be))
  - [Tsys] Update endpoint and unit tests ([#1730](https://github.com/juspay/hyperswitch/pull/1730)) ([`8223f8b`](https://github.com/juspay/hyperswitch/commit/8223f8b29a3b236bf310986013aa0b0b1c9bd7d4))
- **redis_interface:** Add back Redis pool connect step ([#1789](https://github.com/juspay/hyperswitch/pull/1789)) ([`1f8e790`](https://github.com/juspay/hyperswitch/commit/1f8e790b14b049a540474882327545b4434665ee))

### Refactors

- **fix:** [Mollie] Add support for both HeaderKey and BodyKey AuthType ([#1761](https://github.com/juspay/hyperswitch/pull/1761)) ([`07c60f8`](https://github.com/juspay/hyperswitch/commit/07c60f8abf32fb500c6dcf974b8444de476fb210))
- **redis_interface:** Remove the `Drop` implementation on `RedisConnectionPool` ([#1786](https://github.com/juspay/hyperswitch/pull/1786)) ([`ac17b11`](https://github.com/juspay/hyperswitch/commit/ac17b11e09115947e7cf76d66d3ad35c59b47258))

### Testing

- **UI-tests:** Allow ignoring connector tests at runtime ([#1766](https://github.com/juspay/hyperswitch/pull/1766)) ([`884f284`](https://github.com/juspay/hyperswitch/commit/884f284263e243b3a8342ed1c728411fb438e4f9))
- **connector:** [Nexinets] Add UI test for Nexinets Payment methods ([#1784](https://github.com/juspay/hyperswitch/pull/1784)) ([`bf62a7c`](https://github.com/juspay/hyperswitch/commit/bf62a7c9ad8ea35c141e9fcf4edee02ff5856753))

**Full Changelog:** [`v1.10.2...v1.11.0`](https://github.com/juspay/hyperswitch/compare/v1.10.2...v1.11.0)

- - -


## 1.10.2 (2023-07-25)

### Bug Fixes

- **connector:** [Paypal] fix amount to its currency base unit ([#1780](https://github.com/juspay/hyperswitch/pull/1780)) ([`f40d144`](https://github.com/juspay/hyperswitch/commit/f40d1441787977b911f72abe3d9112e4c25817d0))

### Revert

- Connector_label in webhook url is reverted back to connector_name ([#1779](https://github.com/juspay/hyperswitch/pull/1779)) ([`a229c37`](https://github.com/juspay/hyperswitch/commit/a229c37a7cd71fbbd73b4aa1378d1d326cb3bbe8))

**Full Changelog:** [`v1.10.1...v1.10.2`](https://github.com/juspay/hyperswitch/compare/v1.10.1...v1.10.2)

- - -


## 1.10.1 (2023-07-25)

### Bug Fixes

- **config:** Detect duplicate config insert and throw appropriate error ([#1777](https://github.com/juspay/hyperswitch/pull/1777)) ([`1ab4226`](https://github.com/juspay/hyperswitch/commit/1ab4226c780e9205785f012fd1c48c7a4bafb48f))
- **connector:**
  - [Paypal] Fix payment status for PayPal cards ([#1749](https://github.com/juspay/hyperswitch/pull/1749)) ([`88b4b96`](https://github.com/juspay/hyperswitch/commit/88b4b9679d6de62bad7d52442be4565894a1d43b))
  - Apple pay not working because of payment_method_type[] field stripe ([#1759](https://github.com/juspay/hyperswitch/pull/1759)) ([`039a859`](https://github.com/juspay/hyperswitch/commit/039a85977b6479710625e2f7f0c0f9ca0b52571b))
- **core:** Address 500 when deleting payment method and add logs to postman collections ([#1695](https://github.com/juspay/hyperswitch/pull/1695)) ([`df3970f`](https://github.com/juspay/hyperswitch/commit/df3970f20a8d31a856d4e7323a6cbfbb5838a9b3))
- **router:**
  - Validate schedule time before scheduling API key expiry reminder ([#1776](https://github.com/juspay/hyperswitch/pull/1776)) ([`7b1dc78`](https://github.com/juspay/hyperswitch/commit/7b1dc78de5b4396c4ca66da27fa986287c144f22))
  - Restricted unknown customer_id to be pass in payment confirm and update call ([#1758](https://github.com/juspay/hyperswitch/pull/1758)) ([`32c7324`](https://github.com/juspay/hyperswitch/commit/32c73243c06db9e0e1210653bb79ff528d7e8dc5))

### Refactors

- **payments:** Dont update client secret on payment intent status update ([#1778](https://github.com/juspay/hyperswitch/pull/1778)) ([`b719725`](https://github.com/juspay/hyperswitch/commit/b719725864c99b655956ab906e26dead71490b75))

### Documentation

- **postman:** Added a note about how postman now requires you to fork a collection in order to send a request ([#1769](https://github.com/juspay/hyperswitch/pull/1769)) ([`1afc548`](https://github.com/juspay/hyperswitch/commit/1afc54837d5988eaf41f434474c30ec511681bbe))

### Miscellaneous Tasks

- **config:** [Paypal] Add configs for PayPal mandates for adyen ([#1774](https://github.com/juspay/hyperswitch/pull/1774)) ([`bad9b94`](https://github.com/juspay/hyperswitch/commit/bad9b9482398bb624cb34ae7021837f7af6e8e00))

**Full Changelog:** [`v1.10.0...v1.10.1`](https://github.com/juspay/hyperswitch/compare/v1.10.0...v1.10.1)

- - -


## 1.10.0 (2023-07-21)

### Features

- **connector:**
  - [Adyen] implement Online Banking Fpx for Adyen ([#1584](https://github.com/juspay/hyperswitch/pull/1584)) ([`2e492ee`](https://github.com/juspay/hyperswitch/commit/2e492ee6a9e767ef8a30446e3474f13c35afe607))
  - [Adyen] implement Online Banking Thailand for Adyen ([#1585](https://github.com/juspay/hyperswitch/pull/1585)) ([`0c3cf05`](https://github.com/juspay/hyperswitch/commit/0c3cf05ffc56ce60805a8ba7ee5b34b011261f67))
  - [Stripe] Add support for Blik ([#1565](https://github.com/juspay/hyperswitch/pull/1565)) ([`0589c57`](https://github.com/juspay/hyperswitch/commit/0589c572c48338fb8182dcd5de63e3fee574ced3))
  - [Adyen] implement Touch n Go for Adyen ([#1588](https://github.com/juspay/hyperswitch/pull/1588)) ([`8e45e73`](https://github.com/juspay/hyperswitch/commit/8e45e734c87981ce0a8a96f218bc1033dc63af76))
  - [Adyen] implement Atome for Adyen ([#1590](https://github.com/juspay/hyperswitch/pull/1590)) ([`3c5d725`](https://github.com/juspay/hyperswitch/commit/3c5d725cc204b83bca6d916293f6af6cf3648ff1))

### Bug Fixes

- **compatibility:** Map connector_metadata to core request ([#1753](https://github.com/juspay/hyperswitch/pull/1753)) ([`f340860`](https://github.com/juspay/hyperswitch/commit/f340860d793a353e91f2bc4ad197021d7e518aaf))
- **connector:**
  - [Authorizedotnet] Convert amount from cents to dollar before sending to connector ([#1756](https://github.com/juspay/hyperswitch/pull/1756)) ([`a685a9a`](https://github.com/juspay/hyperswitch/commit/a685a9aac5551768cd1afb4836ffae4385cd0fad))
  - [Adyen] Fix error message for fraud check from Adyen connector ([#1763](https://github.com/juspay/hyperswitch/pull/1763)) ([`78ce8f7`](https://github.com/juspay/hyperswitch/commit/78ce8f756357b89795fbb6351e897bfe6d1117c0))
- **router:** Add additional card info in payment response ([#1745](https://github.com/juspay/hyperswitch/pull/1745)) ([`a891708`](https://github.com/juspay/hyperswitch/commit/a891708f6780e3830b1e6ee92268ae70e6fc4860))
- **template:** Address add_connector.sh throwing errors when creating new connector template ([#1679](https://github.com/juspay/hyperswitch/pull/1679)) ([`3951561`](https://github.com/juspay/hyperswitch/commit/3951561752bf8f22e55b983788325c1e072e4168))
- Remove payout test cases from connector-template ([#1757](https://github.com/juspay/hyperswitch/pull/1757)) ([`d433a98`](https://github.com/juspay/hyperswitch/commit/d433a98d1fd93aef9566287e0340879f412e5c2b))

### Testing

- Fix failing unit tests ([#1743](https://github.com/juspay/hyperswitch/pull/1743)) ([`c4c9424`](https://github.com/juspay/hyperswitch/commit/c4c94241a942fd3620f818d70dc2cdeb97cb0e85))

**Full Changelog:** [`v1.9.0...v1.10.0`](https://github.com/juspay/hyperswitch/compare/v1.9.0...v1.10.0)

- - -


## 1.9.0 (2023-07-20)

### Features

- **connector:**
  - [Adyen] implement Momo for Adyen ([#1583](https://github.com/juspay/hyperswitch/pull/1583)) ([`96933f2`](https://github.com/juspay/hyperswitch/commit/96933f2636e39b96435cba8e59b96b8c59413f39))
  - [Adyen] Implement Alma BNPL and DANA Wallet ([#1566](https://github.com/juspay/hyperswitch/pull/1566)) ([`5dcf758`](https://github.com/juspay/hyperswitch/commit/5dcf758ac04716e194601c1571851f07a7d24fcc))
- **metrics:** Add pod information in metrics pipeline ([#1710](https://github.com/juspay/hyperswitch/pull/1710)) ([`cf145a3`](https://github.com/juspay/hyperswitch/commit/cf145a321c4c797f0efa44f846f19048ea69e7ec))
- Add payout service ([#1665](https://github.com/juspay/hyperswitch/pull/1665)) ([`763e2df`](https://github.com/juspay/hyperswitch/commit/763e2df3bdfb426214d94c56529d98f453452266))

### Bug Fixes

- **adyen_ui:** Ignore tests failing from connector side ([#1751](https://github.com/juspay/hyperswitch/pull/1751)) ([`e0f4507`](https://github.com/juspay/hyperswitch/commit/e0f4507b1009c481ecd8216ccd41f44fbc0ccb36))
- **connector:**
  - [PowerTranz] error message from response_code in absence of errors object & comment billing and shipping as it is optional ([#1738](https://github.com/juspay/hyperswitch/pull/1738)) ([`54f7ab7`](https://github.com/juspay/hyperswitch/commit/54f7ab7ae14fa593fa9749c0d67807f68247e899))
  - Update amount captured after webhook call and parse error responses from connector properly ([#1680](https://github.com/juspay/hyperswitch/pull/1680)) ([`cac9f50`](https://github.com/juspay/hyperswitch/commit/cac9f5049e8abee78c260c523e871754cfc2b22c))
  - Deserialization error due to latest_charge stripe ([#1740](https://github.com/juspay/hyperswitch/pull/1740)) ([`c53631e`](https://github.com/juspay/hyperswitch/commit/c53631ef55645e45cb0c3165e79d389e0100b4ac))
  - Stripe mandate failure and other ui tests failures ([#1742](https://github.com/juspay/hyperswitch/pull/1742)) ([`ea119eb`](https://github.com/juspay/hyperswitch/commit/ea119eb856cf47c5e28117ba9ecfce722aff541f))

### Testing

- **connector:**
  - [Authorizedotnet] Add UI test for Authorizedotnet Payment methods  ([#1736](https://github.com/juspay/hyperswitch/pull/1736)) ([`f44cc1e`](https://github.com/juspay/hyperswitch/commit/f44cc1e10705f167d332779a2dc0141566ac765e))
  - [Adyen] Add UI test for Adyen Payment methods ([#1648](https://github.com/juspay/hyperswitch/pull/1648)) ([`2e9b783`](https://github.com/juspay/hyperswitch/commit/2e9b78329a6bb6d400588578f7b83bc1201cc151))
  - [Noon] Add test for Noon Payment methods ([#1714](https://github.com/juspay/hyperswitch/pull/1714)) ([`f06e5dc`](https://github.com/juspay/hyperswitch/commit/f06e5dcd63affd9919d936884e055344bcd3e8ba))

**Full Changelog:** [`v1.8.0...v1.9.0`](https://github.com/juspay/hyperswitch/compare/v1.8.0...v1.9.0)

- - -


## 1.8.0 (2023-07-19)

### Features

- **connector:**
  - [Adyen] Implement Gcash for Adyen ([#1576](https://github.com/juspay/hyperswitch/pull/1576)) ([`df0ef15`](https://github.com/juspay/hyperswitch/commit/df0ef157c3a107f8b3d2bbf37ef9e19ea66425fc))
  - [Adyen] Implement Vipps in Wallets ([#1554](https://github.com/juspay/hyperswitch/pull/1554)) ([`e271ced`](https://github.com/juspay/hyperswitch/commit/e271ced69e64ac65d8e16a699531b12cbe4289dc))
- **merchant_account:** Add `is_recon_enabled` field in merchant_account ([#1713](https://github.com/juspay/hyperswitch/pull/1713)) ([`7549cd3`](https://github.com/juspay/hyperswitch/commit/7549cd3aa62fa2cb2d9e393bd1f3a0c49cbd6dda))

### Bug Fixes

- **connector:**
  - [PowerTranz] resolve pr comments and add comments ([#1726](https://github.com/juspay/hyperswitch/pull/1726)) ([`432a8e0`](https://github.com/juspay/hyperswitch/commit/432a8e02e98494bd20bcb8c2a1a425f9504b86c7))
  - [PowerTranz] fix rsync not implemented error ([#1734](https://github.com/juspay/hyperswitch/pull/1734)) ([`d52b564`](https://github.com/juspay/hyperswitch/commit/d52b564f09c63067b56684fa36d8940e45ccfccc))
  - [PowerTranz] removing optional field shipping address ([#1737](https://github.com/juspay/hyperswitch/pull/1737)) ([`63eac1f`](https://github.com/juspay/hyperswitch/commit/63eac1fdd6ca43f4a87a5008f53bbac5e5d03c37))
- **webhook:** Do not fail webhook verification if merchant_secret is not set by merchant ([#1732](https://github.com/juspay/hyperswitch/pull/1732)) ([`374f2c2`](https://github.com/juspay/hyperswitch/commit/374f2c28cd2b5ec47f3e67eb3fb925cdff5c208a))

### Testing

- **connector:** [Aci] Add UI test for Aci Payment Methods ([#1702](https://github.com/juspay/hyperswitch/pull/1702)) ([`fe7a5b0`](https://github.com/juspay/hyperswitch/commit/fe7a5b039c6221e8ff7f8841e6d5356446b3de20))

**Full Changelog:** [`v1.7.0...v1.8.0`](https://github.com/juspay/hyperswitch/compare/v1.7.0...v1.8.0)

- - -


## 1.7.0 (2023-07-18)

### Features

- **connector:**
  - [Adyen] Implement Twint in Wallets ([#1549](https://github.com/juspay/hyperswitch/pull/1549)) ([`d317021`](https://github.com/juspay/hyperswitch/commit/d317021bc55af8b45cb48b572d44a957d57e7d28))
  - [Stax] Add template code for Stax connector ([#1698](https://github.com/juspay/hyperswitch/pull/1698)) ([`f932d66`](https://github.com/juspay/hyperswitch/commit/f932d66c52a8b8ff78b90d1cd1b02ab068778ba0))
  - [Bluesnap] Remove wallet call  ([#1620](https://github.com/juspay/hyperswitch/pull/1620)) ([`ec35d55`](https://github.com/juspay/hyperswitch/commit/ec35d55da69ee3fef9048de14fc54b10abb32d18))
  - [Adyen] implement Kakao for Adyen ([#1558](https://github.com/juspay/hyperswitch/pull/1558)) ([`11ad9be`](https://github.com/juspay/hyperswitch/commit/11ad9beda81659da080aeb454cbea0476d0639dc))

### Bug Fixes

- **build:** Add a standalone Redis mode in docker-compose installation ([#1661](https://github.com/juspay/hyperswitch/pull/1661)) ([`ee1f6cc`](https://github.com/juspay/hyperswitch/commit/ee1f6ccb4cde3142d0a853dc1b04ac3792a4e68b))
- **router:** Add parsing for `connector_request_reference_id` env ([#1731](https://github.com/juspay/hyperswitch/pull/1731)) ([`110fbe9`](https://github.com/juspay/hyperswitch/commit/110fbe9fc546e51ad945da31f25f242273646ed0))

### Refactors

- **router:** Remove `WebhookApiErrorSwitch ` and implement error mapping using `ErrorSwitch` ([#1660](https://github.com/juspay/hyperswitch/pull/1660)) ([`a7c66dd`](https://github.com/juspay/hyperswitch/commit/a7c66ddea206ea1d22be6ddb1a503badf76fe2cf))

**Full Changelog:** [`v1.6.0...v1.7.0`](https://github.com/juspay/hyperswitch/compare/v1.6.0...v1.7.0)

- - -


## 1.6.0 (2023-07-17)

### Features

- **compatibility:**
  - [upi] add upi pm in compatibility layer, convert amount to base unit in iatapay ([#1711](https://github.com/juspay/hyperswitch/pull/1711)) ([`5213656`](https://github.com/juspay/hyperswitch/commit/5213656fac1cd1372374bfdcd90d41487e7aa387))
  - Add support for stripe compatible webhooks ([#1728](https://github.com/juspay/hyperswitch/pull/1728)) ([`87ae99f`](https://github.com/juspay/hyperswitch/commit/87ae99f7f2247f92078064169f998519cdfcf27b))
- **connector:**
  - [Adyen] Implement Bizum in Bank Redirects ([#1589](https://github.com/juspay/hyperswitch/pull/1589)) ([`c654d76`](https://github.com/juspay/hyperswitch/commit/c654d76660fcca18f54e270920b1d6976a01972b))
  - [Globepay] Add Refund and Refund Sync flow ([#1706](https://github.com/juspay/hyperswitch/pull/1706)) ([`c72a592`](https://github.com/juspay/hyperswitch/commit/c72a592e5e1d5c8ed16ae8fea89a7e3cfd365532))
  - [Mollie] Implement card 3ds ([#1421](https://github.com/juspay/hyperswitch/pull/1421)) ([`91f969a`](https://github.com/juspay/hyperswitch/commit/91f969a2908f4e7b0101a212567305888f51e236))
  - [PowerTranz] Add cards 3ds support for PowerTranz connector ([#1722](https://github.com/juspay/hyperswitch/pull/1722)) ([`95a45e4`](https://github.com/juspay/hyperswitch/commit/95a45e49786db4980fac8e347534048100e24039))
  - [Tsys] Add cards for Payments and Refunds flow ([#1716](https://github.com/juspay/hyperswitch/pull/1716)) ([`714cd27`](https://github.com/juspay/hyperswitch/commit/714cd275b32d16e24a8c1e5f181f97537947a3b9))
  - [Adyen] Implement Clearpay in BNPL ([#1546](https://github.com/juspay/hyperswitch/pull/1546)) ([`abed197`](https://github.com/juspay/hyperswitch/commit/abed197366035a03810b36eead590f189d83e6ac))
  - [Adyen] implement Gopay for Adyen ([#1557](https://github.com/juspay/hyperswitch/pull/1557)) ([`de2d9bd`](https://github.com/juspay/hyperswitch/commit/de2d9bd059ed82b34a6f0656492348693b985ec4))
- **mandates:** Recurring payment support for bank redirect and bank debit payment method for stripe ([#1119](https://github.com/juspay/hyperswitch/pull/1119)) ([`14c2d72`](https://github.com/juspay/hyperswitch/commit/14c2d72509c7fae648bbef620c2e3ef82aa9d8d6))
- **router:**
  - Add attempt_count field in attempt update record of payment_intent ([#1725](https://github.com/juspay/hyperswitch/pull/1725)) ([`95de3a5`](https://github.com/juspay/hyperswitch/commit/95de3a579d073060dd0e4eca382650042bfd6737))
  - Restricted customer update in payments-confirm and payments-update call via clientAuth ([#1659](https://github.com/juspay/hyperswitch/pull/1659)) ([`94a5eb3`](https://github.com/juspay/hyperswitch/commit/94a5eb35335afb4c38f4af62aef1a195f30ec448))

### Bug Fixes

- **ci:** Run UI tests only for 15mins max in case of build failure ([#1718](https://github.com/juspay/hyperswitch/pull/1718)) ([`16a2c46`](https://github.com/juspay/hyperswitch/commit/16a2c46affbd4319ee1106e08922e7f3094adfbe))
- **connector:**
  - [Adyen] Fix Klarna mandates for Adyen ([#1717](https://github.com/juspay/hyperswitch/pull/1717)) ([`c34a049`](https://github.com/juspay/hyperswitch/commit/c34a049506e18fa5f0c458676e54e54f95a1609e))
  - [Adyen] Add bizum in common enums ([#1719](https://github.com/juspay/hyperswitch/pull/1719)) ([`cbde4a6`](https://github.com/juspay/hyperswitch/commit/cbde4a6d7b65cfe11de51f7fd348e238f7ff9500))
  - [Multisafepay] Fix bug in Paypal payment decline and cancel ([#1647](https://github.com/juspay/hyperswitch/pull/1647)) ([`a77ab42`](https://github.com/juspay/hyperswitch/commit/a77ab42f4fde59a48d1e044295b0955152b99b58))
- **payments:** Populate mandate_data in the response body of the PaymentsCreate endpoint ([#1715](https://github.com/juspay/hyperswitch/pull/1715)) ([`fb149cb`](https://github.com/juspay/hyperswitch/commit/fb149cb0ff750fbaadf22d263be0f7bfe1574e37))
- **refunds:** Modify refund fields to process updating of refund_reason ([#1544](https://github.com/juspay/hyperswitch/pull/1544)) ([`9890570`](https://github.com/juspay/hyperswitch/commit/9890570274e344c474b2b0033033ae70e0314cc8))
- **router:** Convert ephemeral to client secret auth list payment_method_customer ([#1602](https://github.com/juspay/hyperswitch/pull/1602)) ([`5fbd1cc`](https://github.com/juspay/hyperswitch/commit/5fbd1cc3c787a64634aac640ced9e2dce59b036d))

### Refactors

- **pm_list:** Update required fields for a payment method ([#1720](https://github.com/juspay/hyperswitch/pull/1720)) ([`8dd9fcc`](https://github.com/juspay/hyperswitch/commit/8dd9fcc2c594f4aebd2f0418986836fce6e5c242))

### Revert

- Refactor(pm_list): Update required fields for a payment method ([#1724](https://github.com/juspay/hyperswitch/pull/1724)) ([`c6f7455`](https://github.com/juspay/hyperswitch/commit/c6f745540fa3096f8024ca29546a006395aa4bf2))

**Full Changelog:** [`v1.5.0...v1.6.0`](https://github.com/juspay/hyperswitch/compare/v1.5.0...v1.6.0)

- - -


## 1.5.0 (2023-07-14)

### Features

- **connector:**
  - [Tsys] Add template code for Tsys connector ([#1704](https://github.com/juspay/hyperswitch/pull/1704)) ([`7609895`](https://github.com/juspay/hyperswitch/commit/76098952105c101c88410c6aa78c2c56298f0aaa))
  - [Authorizedotnet] Add Wallet support ([#1223](https://github.com/juspay/hyperswitch/pull/1223)) ([`05540ea`](https://github.com/juspay/hyperswitch/commit/05540ea17e6fda4ae37b31c46956b3c93f94f903))
  - [Adyen] Add support for PayPal wallet mandates ([#1686](https://github.com/juspay/hyperswitch/pull/1686)) ([`82fd844`](https://github.com/juspay/hyperswitch/commit/82fd84462072a7616806b0e06dc8a6812312f439))
- **router:** Add expand attempts support in payments retrieve response ([#1678](https://github.com/juspay/hyperswitch/pull/1678)) ([`8572f1d`](https://github.com/juspay/hyperswitch/commit/8572f1da8eb57577b18537d3397f03448720ed3d))
- Filter out payment_methods which does not support mandates during list api call ([#1318](https://github.com/juspay/hyperswitch/pull/1318)) ([`07aef53`](https://github.com/juspay/hyperswitch/commit/07aef53a5cd4cd70f75415e883d0e07d85244a1e))
- Add `organization_id` to merchant account ([#1611](https://github.com/juspay/hyperswitch/pull/1611)) ([`7025b78`](https://github.com/juspay/hyperswitch/commit/7025b789b81221d45d7832460fab0c09b92aa9f9))

### Bug Fixes

- **api_keys:** Fix API key being created for non-existent merchant account ([#1712](https://github.com/juspay/hyperswitch/pull/1712)) ([`c9e20dc`](https://github.com/juspay/hyperswitch/commit/c9e20dcd30beb1de0b571dc61a0e843eda3f8ae0))
- **router:** Decrease payment method token time based on payment_intent creation time ([#1682](https://github.com/juspay/hyperswitch/pull/1682)) ([`ce1d205`](https://github.com/juspay/hyperswitch/commit/ce1d2052190623ff85b1af830fe3835300e4d025))
- **ui-test:** Run UI tests only on merge-queue ([#1709](https://github.com/juspay/hyperswitch/pull/1709)) ([`cb0ca0c`](https://github.com/juspay/hyperswitch/commit/cb0ca0cc2f9909921d574dbaa759744edb4cc275))
- Store and retrieve merchant secret from MCA table for webhooks source verification ([#1331](https://github.com/juspay/hyperswitch/pull/1331)) ([`a6645bd`](https://github.com/juspay/hyperswitch/commit/a6645bd3540f66ebfdfa352bce87700c3c67a069))

### Refactors

- **CI-push:** Move merge_group to CI-push ([#1696](https://github.com/juspay/hyperswitch/pull/1696)) ([`08cca88`](https://github.com/juspay/hyperswitch/commit/08cca881c200a3e9a24fa780c035c37f51816ca9))
- **payment_methods:** Remove legacy locker code  as it is not been used ([#1666](https://github.com/juspay/hyperswitch/pull/1666)) ([`8832dd6`](https://github.com/juspay/hyperswitch/commit/8832dd60b98e37a6a46452e9dc1381dd64c2720f))

### Testing

- **connector:**
  - [Multisafepay] Add ui test for card 3ds ([#1688](https://github.com/juspay/hyperswitch/pull/1688)) ([`9112417`](https://github.com/juspay/hyperswitch/commit/9112417caee51117c170af6096825c5b1b2bd0e0))
  - [stripe] Add ui test for affirm ([#1694](https://github.com/juspay/hyperswitch/pull/1694)) ([`8c5703d`](https://github.com/juspay/hyperswitch/commit/8c5703df545007d8b61679bd57d0a58986ec10ce))

### Miscellaneous Tasks

- Address Rust 1.71 clippy lints ([#1708](https://github.com/juspay/hyperswitch/pull/1708)) ([`2cf8ae7`](https://github.com/juspay/hyperswitch/commit/2cf8ae7817db0a74b744f41484db81e1c441ebf3))

**Full Changelog:** [`v1.4.0...v1.5.0`](https://github.com/juspay/hyperswitch/compare/v1.4.0...v1.5.0)

- - -


## 1.4.0 (2023-07-13)

### Features

- **connector:**
  - [Globepay] add authorize and psync flow  ([#1639](https://github.com/juspay/hyperswitch/pull/1639)) ([`c119bfd`](https://github.com/juspay/hyperswitch/commit/c119bfdd7e93d345c340cf1282f47ab297b2c4e2))
  - [PowerTranz] Add cards support for PowerTranz connector ([#1687](https://github.com/juspay/hyperswitch/pull/1687)) ([`07120bf`](https://github.com/juspay/hyperswitch/commit/07120bf422048255f93d7073c4dcd2f853667ffd))
- **payments:** Add client secret in redirect response  ([#1693](https://github.com/juspay/hyperswitch/pull/1693)) ([`f7d369a`](https://github.com/juspay/hyperswitch/commit/f7d369afa8b459a18a5ec0a36caebdb1a4fe72b4))
- **router:** Add connector_response_reference_id in payments response ([#1664](https://github.com/juspay/hyperswitch/pull/1664)) ([`a3ea5dc`](https://github.com/juspay/hyperswitch/commit/a3ea5dc09c7aef016bf4c5839317cfbbbe48cdb5))

### Bug Fixes

- **compatibility:**
  - Fix mismatched fields in the payments flow  ([#1640](https://github.com/juspay/hyperswitch/pull/1640)) ([`e0113b9`](https://github.com/juspay/hyperswitch/commit/e0113b98fd02d817a90f60fef177ee0faed02f68))
  - Fix AddressDetails in the customers flow ([#1654](https://github.com/juspay/hyperswitch/pull/1654)) ([`f48d6c4`](https://github.com/juspay/hyperswitch/commit/f48d6c4a2ba53a12b81eb491bd1cadc2b2be6a09))

### Refactors

- **enums:** Move enums from `storage_models` and `api_models` crates to `common_enums` crate ([#1265](https://github.com/juspay/hyperswitch/pull/1265)) ([`c0e1d4d`](https://github.com/juspay/hyperswitch/commit/c0e1d4d3b014ee4d75b3e96b1347e54e722d82ab))
- **payment_methods:** Fix db insert for payment method create ([#1651](https://github.com/juspay/hyperswitch/pull/1651)) ([`73f91a5`](https://github.com/juspay/hyperswitch/commit/73f91a5eee3046f5fcfbfaf1c772f53ea8bf6344))
- **storage:** Update crate name to diesel models ([#1685](https://github.com/juspay/hyperswitch/pull/1685)) ([`5a0e8be`](https://github.com/juspay/hyperswitch/commit/5a0e8be8c4a6b112e0f0e5475c876e57802100ab))

### Testing

- **connector:** [Trustpay] Add ui test for card 3ds  ([#1683](https://github.com/juspay/hyperswitch/pull/1683)) ([`3f756e5`](https://github.com/juspay/hyperswitch/commit/3f756e59c32aa667d7e244c1c7fe36394571b982))

**Full Changelog:** [`v1.3.0...v1.4.0`](https://github.com/juspay/hyperswitch/compare/v1.3.0...v1.4.0)

- - -


## 1.3.0 (2023-07-12)

### Features

- **payments:** Dont delete client secret on success status ([#1692](https://github.com/juspay/hyperswitch/pull/1692)) ([`5216d22`](https://github.com/juspay/hyperswitch/commit/5216d22efcd291f7e460d1461ef16cef69ad6bd9))
- Convert QrData into Qr data image source url ([#1674](https://github.com/juspay/hyperswitch/pull/1674)) ([`55ff761`](https://github.com/juspay/hyperswitch/commit/55ff761e9eca313327f67c1d271ea1672d12c339))

### Refactors

- Include binary name in `crates_to_filter` for logging ([#1689](https://github.com/juspay/hyperswitch/pull/1689)) ([`123b34c`](https://github.com/juspay/hyperswitch/commit/123b34c7dca543194b230bc9e46e14758f8bfb34))

**Full Changelog:** [`v1.2.0...v1.3.0`](https://github.com/juspay/hyperswitch/compare/v1.2.0...v1.3.0)

- - -


## 1.2.0 (2023-07-11)

### Features

- **connector:** [PowerTranz] Add template code for PowerTranz connector ([#1650](https://github.com/juspay/hyperswitch/pull/1650)) ([`f56f9d6`](https://github.com/juspay/hyperswitch/commit/f56f9d643451b9a7ff961b21fc6ec0eefac0ebdf))
- **payments:** Add client_secret auth for payments retrieve ([#1663](https://github.com/juspay/hyperswitch/pull/1663)) ([`b428298`](https://github.com/juspay/hyperswitch/commit/b428298030b3c04a249f175b51b7904ab96e2ce7))
- **pm_list:** Add required field info for crypto pay ([#1655](https://github.com/juspay/hyperswitch/pull/1655)) ([`6d4943d`](https://github.com/juspay/hyperswitch/commit/6d4943d825128250be4db54e88c3a67c01262636))
- **router:** Add connector_request_reference_id in router_data based on merchant config ([#1627](https://github.com/juspay/hyperswitch/pull/1627)) ([`865db94`](https://github.com/juspay/hyperswitch/commit/865db9411da88b11546830ba28d72cc73ab41c10))

### Bug Fixes

- **CI:** Fix msrv checks on github run on push to main ([#1645](https://github.com/juspay/hyperswitch/pull/1645)) ([`05ea08b`](https://github.com/juspay/hyperswitch/commit/05ea08bcc5c69e09462a4019830170dc0f67dfd9))
- **core:**
  - Fix wallet payments throwing `Invalid 'payment_method_type' provided` and UI test issues ([#1633](https://github.com/juspay/hyperswitch/pull/1633)) ([`307a470`](https://github.com/juspay/hyperswitch/commit/307a470f7d838dc53df07a004ab89045ee0048ff))
  - Add Payment_Method_data in Redirection Form  ([#1668](https://github.com/juspay/hyperswitch/pull/1668)) ([`b043ce6`](https://github.com/juspay/hyperswitch/commit/b043ce6130bf27f6279401ec98237aa91632480a))
- **locker:** Remove delete_locker_payment_method_by_lookup_key from payments_operation_core ([#1636](https://github.com/juspay/hyperswitch/pull/1636)) ([`b326c18`](https://github.com/juspay/hyperswitch/commit/b326c18f45703724b1c22c69debd15ada841bf2e))
- **middleware:** Include `x-request-id` header in `access-control-expose-headers` header value ([#1673](https://github.com/juspay/hyperswitch/pull/1673)) ([`b1ae981`](https://github.com/juspay/hyperswitch/commit/b1ae981f82697f788d64bed146fd989a6eca16fe))
- **router:**
  - Use `Connector` enum for `connector_name` field in `MerchantConnectorCreate` ([#1637](https://github.com/juspay/hyperswitch/pull/1637)) ([`e750a73`](https://github.com/juspay/hyperswitch/commit/e750a7332376a60843dde9e71adfa76ce48fd154))
  - Remove requires_customer_action status to payment confirm ([#1624](https://github.com/juspay/hyperswitch/pull/1624)) ([`69454ec`](https://github.com/juspay/hyperswitch/commit/69454ec55c1392aee7a5215f7dc0c834fd6613d2))
- Map not found error properly in db_not found ([#1671](https://github.com/juspay/hyperswitch/pull/1671)) ([`fbd40b5`](https://github.com/juspay/hyperswitch/commit/fbd40b5ac44b7410da9d4b139b15561e20bca616))

**Full Changelog:** [`v1.1.1...v1.2.0`](https://github.com/juspay/hyperswitch/compare/v1.1.1...v1.2.0)

- - -


## 1.1.0 (2023-07-07)

### Features

- **connector:**
  - [Globepay] Add template code for Globepay connector ([#1623](https://github.com/juspay/hyperswitch/pull/1623)) ([`06f92c2`](https://github.com/juspay/hyperswitch/commit/06f92c2c4c267e3d6ec914670684bb36b71ecd51))
  - [Payme] add Authorize, Sync, Capture, Refund, Refund Sync, Mandate & web hooks support for cards ([#1594](https://github.com/juspay/hyperswitch/pull/1594)) ([`093cc6a`](https://github.com/juspay/hyperswitch/commit/093cc6a71cb3060c06bc4e6238af8896b36308db))
- **router:** Get filters for payments ([#1600](https://github.com/juspay/hyperswitch/pull/1600)) ([`d5891ec`](https://github.com/juspay/hyperswitch/commit/d5891ecbd4a110e3885d6504194f7c7811a413d3))
- Add cache for api_key and mca tables ([#1212](https://github.com/juspay/hyperswitch/pull/1212)) ([`fc9057e`](https://github.com/juspay/hyperswitch/commit/fc9057ef2c601fd8a7deb5d10dc5678abd8e6f7b))

### Bug Fixes

- **router:** Desc payment list for pagination ([#1556](https://github.com/juspay/hyperswitch/pull/1556)) ([`f77fdb7`](https://github.com/juspay/hyperswitch/commit/f77fdb7a6ed354151d8a758a734382a4c3b2698e))

**Full Changelog:** [`v1.0.5...v1.1.0`](https://github.com/juspay/hyperswitch/compare/v1.0.5...v1.1.0)

- - -

## 1.0.5 (2023-07-06)

### Features

- **connector:** [Stripe] Add support for WeChat Pay and Qr code support in next action ([#1555](https://github.com/juspay/hyperswitch/pull/1555)) ([`a15a77d`](https://github.com/juspay/hyperswitch/commit/a15a77dea36fd13e92bd64014fc25014d51a3548))
- **test:** Add support to run UI tests in CI pipeline ([#1539](https://github.com/juspay/hyperswitch/pull/1539)) ([`21f5e20`](https://github.com/juspay/hyperswitch/commit/21f5e20929dfef9ffdd2f20fb0fd190c59e35316))

### Bug Fixes

- **connector:** [Rapyd] Add router_return_url in 3DS request ([#1621](https://github.com/juspay/hyperswitch/pull/1621)) ([`e913bfc`](https://github.com/juspay/hyperswitch/commit/e913bfc4958da613cd352eca9bc38b23ab7ac38e))

### Refactors

- **payments:** Error message of manual retry ([#1617](https://github.com/juspay/hyperswitch/pull/1617)) ([`fad4895`](https://github.com/juspay/hyperswitch/commit/fad4895f756811bb0af9ccbc69b9f6dfff3ab32f))

**Full Changelog:** [`v1.0.4...v1.0.5`](https://github.com/juspay/hyperswitch/compare/v1.0.4...v1.0.5)

- - -

## 1.0.4 (2023-07-05)

### Features

- **connector:** [DummyConnector] add new dummy connectors ([#1609](https://github.com/juspay/hyperswitch/pull/1609)) ([`cf7b672`](https://github.com/juspay/hyperswitch/commit/cf7b67286c5102f457595e287f4f9315046fe267))
- **payments:** Add connector_metadata, metadata and feature_metadata fields in payments, remove udf field ([#1595](https://github.com/juspay/hyperswitch/pull/1595)) ([`e713b62`](https://github.com/juspay/hyperswitch/commit/e713b62ae3444ef9a9a8984f9fd593936734dc41))
- **router:**
  - Modify attempt_id generation logic to accommodate payment_id as prefix ([#1596](https://github.com/juspay/hyperswitch/pull/1596)) ([`82e1bf0`](https://github.com/juspay/hyperswitch/commit/82e1bf0d168c60733775f933c838b6f9a6301cad))
  - Add card_info in payment_attempt table if not provided in request ([#1538](https://github.com/juspay/hyperswitch/pull/1538)) ([`5628985`](https://github.com/juspay/hyperswitch/commit/5628985c400500d031b0da2c7cef1b04118a096d))
- List payment_methods with the required fields in each method ([#1310](https://github.com/juspay/hyperswitch/pull/1310)) ([`6447b04`](https://github.com/juspay/hyperswitch/commit/6447b04574e941b9214239bf5b65b7c1a229dfd6))

### Bug Fixes

- **payment_methods:** Return an empty array when the merchant does not have any payment methods ([#1601](https://github.com/juspay/hyperswitch/pull/1601)) ([`04c60d7`](https://github.com/juspay/hyperswitch/commit/04c60d73cb34a3432fcb9fa24af95022b16048b2))

### Refactors

- **fix:** [Nuvei] fix currency conversion issue in nuvei cards ([#1605](https://github.com/juspay/hyperswitch/pull/1605)) ([`1b22638`](https://github.com/juspay/hyperswitch/commit/1b226389bd5c8c5dba211dc058c981d8d543f45a))
- **redis_interface:** Changed the in the get_options value from true to false ([#1606](https://github.com/juspay/hyperswitch/pull/1606)) ([`737aeb6`](https://github.com/juspay/hyperswitch/commit/737aeb6b0a083bdbcde169d4cfeb40ebc6f4378e))
- **router:** Add psync task to process tracker after building connector request in payments flow ([#1603](https://github.com/juspay/hyperswitch/pull/1603)) ([`e978e9d`](https://github.com/juspay/hyperswitch/commit/e978e9d66bcb8ea20837fa0e87aa0b0ffffac622))

### Miscellaneous Tasks

- **connector-template:** Update connector template code ([#1612](https://github.com/juspay/hyperswitch/pull/1612)) ([`8c90d0a`](https://github.com/juspay/hyperswitch/commit/8c90d0a78c99c6934a505324e07985eb31ac2f32))

**Full Changelog:** [`v1.0.3...v1.0.4`](https://github.com/juspay/hyperswitch/compare/v1.0.3...v1.0.4)

- - -

## 1.0.3 (2023-07-04)

### Features

- **compatibility:** Add straight through routing and udf mapping in setup intent ([#1536](https://github.com/juspay/hyperswitch/pull/1536)) ([`1e87f3d`](https://github.com/juspay/hyperswitch/commit/1e87f3d6732fea1b44e2caa17ececb10203d9798))
- **connector:**
  - [Adyen] implement Alipay HK for Adyen ([#1547](https://github.com/juspay/hyperswitch/pull/1547)) ([`2f9c289`](https://github.com/juspay/hyperswitch/commit/2f9c28938f95a58532604817b1ed370ef8285dd8))
  - [Mollie] Implement Przelewy24 and BancontactCard Bank Redirects for Mollie connector ([#1303](https://github.com/juspay/hyperswitch/pull/1303)) ([`f091be6`](https://github.com/juspay/hyperswitch/commit/f091be60cc628eff4a3537cd6f5d00402a08650d))
  - [Multisafepay] implement Googlepay for Multisafepay ([#1456](https://github.com/juspay/hyperswitch/pull/1456)) ([`2136326`](https://github.com/juspay/hyperswitch/commit/213632616642522df0983e62a69fb48d170f4e80))
  - [TrustPay] Add Google Pay support ([#1515](https://github.com/juspay/hyperswitch/pull/1515)) ([`47cd08a`](https://github.com/juspay/hyperswitch/commit/47cd08a0b07d457793d376b6cca3143011426f22))
  - [Airwallex] Implement Google Pay in Wallets ([#1316](https://github.com/juspay/hyperswitch/pull/1316)) ([`7489c87`](https://github.com/juspay/hyperswitch/commit/7489c870d9d85f169fb7fca469778fad5b2cc37a))
  - [Multisafepay] implement Paypal for Multisafepay ([#1459](https://github.com/juspay/hyperswitch/pull/1459)) ([`2c10e0b`](https://github.com/juspay/hyperswitch/commit/2c10e0b05c571a7c34c8f3f641b401bae68132a0))
- **db:** Implement `ConfigInterface` for `MockDb` ([#1586](https://github.com/juspay/hyperswitch/pull/1586)) ([`2ac1f2e`](https://github.com/juspay/hyperswitch/commit/2ac1f2e29ec08c457781a7456cb30a80a2bdd1f4))
- **email:** Implement process_tracker for scheduling email when api_key is about to expire ([#1233](https://github.com/juspay/hyperswitch/pull/1233)) ([`ee7cdef`](https://github.com/juspay/hyperswitch/commit/ee7cdef10754a72106271bf164e0acd751a8d35f))
- **payment_method:** [upi] add new payment method and use in iatapay ([#1528](https://github.com/juspay/hyperswitch/pull/1528)) ([`2d11bf5`](https://github.com/juspay/hyperswitch/commit/2d11bf5b3ac94b207978ef7a67d3ab70bd77a139))
- **payments:** Add field manual_retry_allowed in payments response ([#1298](https://github.com/juspay/hyperswitch/pull/1298)) ([`44b8da4`](https://github.com/juspay/hyperswitch/commit/44b8da430c5e5b0114e73b80c5a49d06beebf350))
- **router:**
  - Add requeue support for payments and fix duplicate entry error in process tracker for requeued payments ([#1567](https://github.com/juspay/hyperswitch/pull/1567)) ([`b967d23`](https://github.com/juspay/hyperswitch/commit/b967d232519b106d88d79da2d6baec550c9256df))
  - Add metrics for webhooks ([#1266](https://github.com/juspay/hyperswitch/pull/1266)) ([`d528132`](https://github.com/juspay/hyperswitch/commit/d528132932266aaa793bfe27fa6f40dcd56a8e6a)) by shashank.attarde@juspay.in
- Feat: add `merchant_name` field in the response body ([#1280](https://github.com/juspay/hyperswitch/pull/1280)) ([`dd4ba63`](https://github.com/juspay/hyperswitch/commit/dd4ba63cc4940b3e968a2a8eaf841de2ae14b3f8))
- Add `GenericNotFoundError` error response and `set_key_if_not_exists_with_expiry` Redis command ([#1526](https://github.com/juspay/hyperswitch/pull/1526)) ([`9a88a32`](https://github.com/juspay/hyperswitch/commit/9a88a32d5092cdacacc41bc8ec12ff56d4f53adf))

### Bug Fixes

- **disputes:** Update 4xx error for Files - Delete endpoint ([#1531](https://github.com/juspay/hyperswitch/pull/1531)) ([`eabe16c`](https://github.com/juspay/hyperswitch/commit/eabe16cc8516335b402fdecfd299d26c89cd8ce7))
- **payment_method:** Do not save card in locker in case of error from connector ([#1341](https://github.com/juspay/hyperswitch/pull/1341)) ([`9794079`](https://github.com/juspay/hyperswitch/commit/9794079c797dcb30edcd88e93e8448948321287c)) by karthikey.hegde@juspay.in
- Return nick name for each card while listing saved cards ([#1391](https://github.com/juspay/hyperswitch/pull/1391)) ([`4808af3`](https://github.com/juspay/hyperswitch/commit/4808af37503ed9cf506ac16c5d7cc68a79e30050))
- Add appropriate printable text for Result returned from delete_tokenized_data() ([#1369](https://github.com/juspay/hyperswitch/pull/1369)) ([`cebe993`](https://github.com/juspay/hyperswitch/commit/cebe993660c1afbbd0c442c0811f215286ccff8d))

### Refactors

- **connector:** [ACI] Use verbose names for `InstructionSource` variants ([#1575](https://github.com/juspay/hyperswitch/pull/1575)) ([`df01f8f`](https://github.com/juspay/hyperswitch/commit/df01f8f382ef68ff1798e5c8023f1aef83deeb2b))
- **payment_methods:** Added clone derivation for PaymentMethodId ([#1568](https://github.com/juspay/hyperswitch/pull/1568)) ([`6739b59`](https://github.com/juspay/hyperswitch/commit/6739b59bc8c94650e398901b402e977de28661e6))
- **payments_start:** Remove redundant call to fetch payment method data ([#1574](https://github.com/juspay/hyperswitch/pull/1574)) ([`6dd61b6`](https://github.com/juspay/hyperswitch/commit/6dd61b62ef322462e1a592e2dd3ef31683507f65))
- Add payment id and merchant id to logs ([#1548](https://github.com/juspay/hyperswitch/pull/1548)) ([`9a48c9e`](https://github.com/juspay/hyperswitch/commit/9a48c9ef723f1028bced71396a4f450af5703e82))

### Miscellaneous Tasks

- Update connector creds ([#1597](https://github.com/juspay/hyperswitch/pull/1597)) ([`d5b3f7c`](https://github.com/juspay/hyperswitch/commit/d5b3f7c0301b1cca809b37ce1288c939ee4a7277))

- - -

## 1.0.2 (2023-06-30)

### Features

- **connector:**
  - [Opayo] Add script generated template code ([#1295](https://github.com/juspay/hyperswitch/pull/1295)) ([`60e15dd`](https://github.com/juspay/hyperswitch/commit/60e15ddabbf7ca81ace088a08814c626215301eb))
  - [ACI] implement Card Mandates for ACI ([#1174](https://github.com/juspay/hyperswitch/pull/1174)) ([`15c2a70`](https://github.com/juspay/hyperswitch/commit/15c2a70b427df1c7ec719c2e738f83be1b6a5662))
  - [cryptopay] add new connector cryptopay, authorize, sync, webhook and testcases ([#1511](https://github.com/juspay/hyperswitch/pull/1511)) ([`7bb0aa5`](https://github.com/juspay/hyperswitch/commit/7bb0aa5ceb2e0d12b590602b9ad7c6803e1d5c43))
- **router:** Add filters for refunds ([#1501](https://github.com/juspay/hyperswitch/pull/1501)) ([`88860b9`](https://github.com/juspay/hyperswitch/commit/88860b9c0be0bc91bcdd6f89b60eb43a18b83b08))

### Testing

- **connector:** Add tests for Paypal, Adyen and Airwallex ([#1290](https://github.com/juspay/hyperswitch/pull/1290)) ([`cd4dbcb`](https://github.com/juspay/hyperswitch/commit/cd4dbcb3f6aba9a4b40f28a1ac5f0bb00a21029e))

**Full Changelog:** [`v1.0.1...v1.0.2`](https://github.com/juspay/hyperswitch/compare/v1.0.1...v1.0.2)

- - -

## 1.0.1 (2023-06-28)

### Features

- **connector:**
  - Add connector cashtocode ([#1429](https://github.com/juspay/hyperswitch/pull/1429)) ([`784847b`](https://github.com/juspay/hyperswitch/commit/784847b08ca00ee5b77abf6faaeb9673b57adec3))
  - [Adyen] Add support for Samsung Pay ([#1525](https://github.com/juspay/hyperswitch/pull/1525)) ([`33309da`](https://github.com/juspay/hyperswitch/commit/33309daf5ced2197c030d2c51b02a9d9d1878b9f))
  - [Noon] add error response handling in payments response ([#1494](https://github.com/juspay/hyperswitch/pull/1494)) ([`8254555`](https://github.com/juspay/hyperswitch/commit/82545555d79da654575decf5ed02aa6f12df6469))
  - [Stripe] Add support for refund webhooks ([#1488](https://github.com/juspay/hyperswitch/pull/1488)) ([`e6529b6`](https://github.com/juspay/hyperswitch/commit/e6529b6a63760fd78c26084f96aeeff7e6f844dc))
  - [Payme] Add template code for Payme connector ([#1486](https://github.com/juspay/hyperswitch/pull/1486)) ([`5305a7b`](https://github.com/juspay/hyperswitch/commit/5305a7b2f849fc29a786968ba02b9522d82164e4))
  - [Mollie] Implement Sepa Direct Debit ([#1301](https://github.com/juspay/hyperswitch/pull/1301)) ([`b4b6440`](https://github.com/juspay/hyperswitch/commit/b4b6440a9135b75ae76eff1c1bb8c013aa2dd7f3))
  - Add refund and dispute webhooks for Rapyd ([#1313](https://github.com/juspay/hyperswitch/pull/1313)) ([`db011f3`](https://github.com/juspay/hyperswitch/commit/db011f3d7690458c64c8bba75920b0646b502646))
- **db:** Implement `EphemeralKeyInterface` for `MockDb` ([#1285](https://github.com/juspay/hyperswitch/pull/1285)) ([`8c93904`](https://github.com/juspay/hyperswitch/commit/8c93904c3e34cb7543ce10e022fa5a7f5a10e56f))
- **router:**
  - Implement `PaymentMethodInterface` for `MockDB` ([#1535](https://github.com/juspay/hyperswitch/pull/1535)) ([`772fc84`](https://github.com/juspay/hyperswitch/commit/772fc8457749ceed121f6f7bd9244e4d8b66350e))
  - Add `connector_transaction_id` in payments response ([#1542](https://github.com/juspay/hyperswitch/pull/1542)) ([`1a8f5ff`](https://github.com/juspay/hyperswitch/commit/1a8f5ff2258a90f9cef5bcf5a1891804250f4560))

### Bug Fixes

- **connector:**
  - [Braintree] Map `SubmittedForSettlement` status to `Pending` instead of `Charged` ([#1508](https://github.com/juspay/hyperswitch/pull/1508)) ([`9cc14b8`](https://github.com/juspay/hyperswitch/commit/9cc14b80445ed6b036e7ebc3ea02371465f20f62))
  - [Cybersource] Throw proper unauthorised message ([#1529](https://github.com/juspay/hyperswitch/pull/1529)) ([`3e284b0`](https://github.com/juspay/hyperswitch/commit/3e284b04b1f02f190cd386f1ee6149bf7b25aa87))
  - [Bluesnap] add cardholder info in bluesnap payment request ([#1540](https://github.com/juspay/hyperswitch/pull/1540)) ([`0bc1e04`](https://github.com/juspay/hyperswitch/commit/0bc1e043fe2ff4e6514ef6c87fab2bb7c0911453))
- **payment_methods:** Return appropriate error when basilisk locker token expires ([#1517](https://github.com/juspay/hyperswitch/pull/1517)) ([`9969c93`](https://github.com/juspay/hyperswitch/commit/9969c930a9fc0e983f77e38da45710b87e1203d1))
- **routes:** Register handler for retrieve disput evidence endpoint ([#1516](https://github.com/juspay/hyperswitch/pull/1516)) ([`6bc4188`](https://github.com/juspay/hyperswitch/commit/6bc4188ff981f9539637752464d07e18fba4ba39))
- Invalidate all cache on invalidate cache route ([#1498](https://github.com/juspay/hyperswitch/pull/1498)) ([`2c6cc6a`](https://github.com/juspay/hyperswitch/commit/2c6cc6ab50b1cc83d14f8e164c5e780392288d5f))
- Add 3ds card_holder_info and 2 digit expiry year ([#1560](https://github.com/juspay/hyperswitch/pull/1560)) ([`5f83fae`](https://github.com/juspay/hyperswitch/commit/5f83fae3c4b84e0d512a536d936d17c4f44b23ef))
- Add config create route back ([#1559](https://github.com/juspay/hyperswitch/pull/1559)) ([`379d1d1`](https://github.com/juspay/hyperswitch/commit/379d1d1375783f2c35edbf4dda6bbb0eb9351a3c))

### Performance

- **logging:** Remove redundant heap allocation present in the logging framework ([#1487](https://github.com/juspay/hyperswitch/pull/1487)) ([`b1ed934`](https://github.com/juspay/hyperswitch/commit/b1ed93468cf8c54f2ae53420c0293a2e5a15fca4))

### Refactors

- **mandates:** Refactor mandates to check for misleading error codes in mandates ([#1377](https://github.com/juspay/hyperswitch/pull/1377)) ([`a899c97`](https://github.com/juspay/hyperswitch/commit/a899c9738941fd1a34841369c9a13b2ac49dda9c))

### Testing

- **connector:**
  - [Checkout] Add tests for 3DS and Gpay ([#1267](https://github.com/juspay/hyperswitch/pull/1267)) ([`218803a`](https://github.com/juspay/hyperswitch/commit/218803aaa75e4acdf145872056da76055424a595))
  - [Adyen] Add test for bank debits, bank redirects, and wallets ([#1260](https://github.com/juspay/hyperswitch/pull/1260)) ([`eddcc34`](https://github.com/juspay/hyperswitch/commit/eddcc3455b91569d60ecc955c0ba62d71dc8fefd))
  - [Bambora] Add tests for 3DS ([#1254](https://github.com/juspay/hyperswitch/pull/1254)) ([`295d41a`](https://github.com/juspay/hyperswitch/commit/295d41abba3ff02d7942534163ebc24ae57adf44))
  - [Mollie] Add tests for PayPal, Sofort, Ideal, Giropay and EPS ([#1246](https://github.com/juspay/hyperswitch/pull/1246)) ([`9ea9e55`](https://github.com/juspay/hyperswitch/commit/9ea9e5523b480d862d94cf22b92eb8533f0b8175))
  - Add tests for Globalpay and Bluesnap ([#1281](https://github.com/juspay/hyperswitch/pull/1281)) ([`c5ff6ed`](https://github.com/juspay/hyperswitch/commit/c5ff6ed45b6d053de1b5aa9db918a62887feb417))
  - [Shift4] Add tests for 3DS and Bank Redirect ([#1250](https://github.com/juspay/hyperswitch/pull/1250)) ([`041ecbb`](https://github.com/juspay/hyperswitch/commit/041ecbbcf39bbba5e2c274c7b6a485f3f096aa50))

### Miscellaneous Tasks

- **connector:** [Payme] disable payme connector in code ([#1561](https://github.com/juspay/hyperswitch/pull/1561)) ([`3cd4746`](https://github.com/juspay/hyperswitch/commit/3cd474604d04875a9e39ea0ee520dbb59b130867))

**Full Changelog:** [`v1.0.0...v1.0.1`](https://github.com/juspay/hyperswitch/compare/v1.0.0...v1.0.1)

- - -

## 1.0.0 (2023-06-23)

### Features

- **connector:** Enforce logging for connector requests ([#1467](https://github.com/juspay/hyperswitch/pull/1467)) ([`e575fde`](https://github.com/juspay/hyperswitch/commit/e575fde6dc22675af18e80b005872dec2f6cc22c))
- **router:** Add route to invalidate cache entry ([#1100](https://github.com/juspay/hyperswitch/pull/1100)) ([`21f2ccd`](https://github.com/juspay/hyperswitch/commit/21f2ccd47c3627c760ade1b5fe90c3c13a46210e))
- Fetch merchant key store only once per session ([#1400](https://github.com/juspay/hyperswitch/pull/1400)) ([`d321aa1`](https://github.com/juspay/hyperswitch/commit/d321aa1f7296932074ce86d6d0df97f312777bc7))
- Add default pm_filters ([#1493](https://github.com/juspay/hyperswitch/pull/1493)) ([`69e9e51`](https://github.com/juspay/hyperswitch/commit/69e9e518f40c4267c1d58b455b83088e431f767f))

### Bug Fixes

- **compatibility:** Add metadata object in both payment_intent and setup_intent request ([#1519](https://github.com/juspay/hyperswitch/pull/1519)) ([`6ec6272`](https://github.com/juspay/hyperswitch/commit/6ec6272f2acae6d5cb5e3120b2dbcc87ae2875ec))
- **configs:** Remove pix and twint from pm_filters for adyen ([#1509](https://github.com/juspay/hyperswitch/pull/1509)) ([`c1e8ad1`](https://github.com/juspay/hyperswitch/commit/c1e8ad194f45c2d08cb3975237ec4d266cf4ee83))
- **connector:**
  - [NMI] Fix Psync flow ([#1474](https://github.com/juspay/hyperswitch/pull/1474)) ([`2fdd14c`](https://github.com/juspay/hyperswitch/commit/2fdd14c38292653494c65560fff0aac6fbc6a726))
  - [DummyConnector] change dummy connector names ([#1328](https://github.com/juspay/hyperswitch/pull/1328)) ([`6645c4d`](https://github.com/juspay/hyperswitch/commit/6645c4d123399e2b6615c02932adf4571b8bcd91))
  - [ACI] fix cancel and refund request encoder ([#1507](https://github.com/juspay/hyperswitch/pull/1507)) ([`cf72dcd`](https://github.com/juspay/hyperswitch/commit/cf72dcdbb6d2164b83b22593f4ebd1be9c774b58))
  - Convert state of US and CA in ISO format for cybersource connector ([#1506](https://github.com/juspay/hyperswitch/pull/1506)) ([`4a047ce`](https://github.com/juspay/hyperswitch/commit/4a047ce133661d160c028d502b5f5eb96b7bdb12))
  - [Trustpay] handle errors fields as optional in TrustpayErrorResponse object ([#1514](https://github.com/juspay/hyperswitch/pull/1514)) ([`efe1ed9`](https://github.com/juspay/hyperswitch/commit/efe1ed9b770dc0924cf00f76ed02e8777bea4ed2))
  - [TrustPay] change the request encoding ([#1530](https://github.com/juspay/hyperswitch/pull/1530)) ([`692d370`](https://github.com/juspay/hyperswitch/commit/692d3704976aa80ea10dfc4cea808f8dba59959e))
  - Fix url_encode issue for paypal and payu ([#1534](https://github.com/juspay/hyperswitch/pull/1534)) ([`e296a49`](https://github.com/juspay/hyperswitch/commit/e296a49b623004784cece505ab08b172a5aa796c))
- **core:** `payment_method_type` not set in the payment attempt when making a recurring mandate payment ([#1415](https://github.com/juspay/hyperswitch/pull/1415)) ([`38b9e59`](https://github.com/juspay/hyperswitch/commit/38b9e59b7511b0486556f9899870d1c9c95c7518))
- **encryption:** Do not log encrypted binary data ([#1352](https://github.com/juspay/hyperswitch/pull/1352)) ([`b0c103a`](https://github.com/juspay/hyperswitch/commit/b0c103a19304cc21e9988675786c3c17dac9fb63))
- **errors:** Use `format!()` for `RefundNotPossibleError` ([#1518](https://github.com/juspay/hyperswitch/pull/1518)) ([`1da411e`](https://github.com/juspay/hyperswitch/commit/1da411e67a2e30e773beb87228cd2fb1fd4b1507))
- **payments:** Fix client secret parsing ([#1358](https://github.com/juspay/hyperswitch/pull/1358)) ([`2b71d4d`](https://github.com/juspay/hyperswitch/commit/2b71d4d8c40c3697e902398fc76bc1256d5b25ee))
- **process_tracker:** Log and ignore the duplicate entry error ([#1502](https://github.com/juspay/hyperswitch/pull/1502)) ([`424e77c`](https://github.com/juspay/hyperswitch/commit/424e77c912e3f9722660b424581aaf9b132fd3a6))
- **update_trackers:** Handle preprocessing steps status update ([#1496](https://github.com/juspay/hyperswitch/pull/1496)) ([`b452314`](https://github.com/juspay/hyperswitch/commit/b45231468db1e71a113ecc1f35841e80f82d8b3f))
- Add requires_customer_action status to payment confirm ([#1500](https://github.com/juspay/hyperswitch/pull/1500)) ([`6944415`](https://github.com/juspay/hyperswitch/commit/6944415da14cda3e9d5fbef62805d7b18d64eacf))
- Update adyen payment method supported countries and currencies in development.toml ([#1401](https://github.com/juspay/hyperswitch/pull/1401)) ([`5274f53`](https://github.com/juspay/hyperswitch/commit/5274f53dcc250804e59c1c13b2fe71daa36195e7))

### Refactors

- **core:** Rename `MandateTxnType` to `MandateTransactionType` ([#1322](https://github.com/juspay/hyperswitch/pull/1322)) ([`1069172`](https://github.com/juspay/hyperswitch/commit/10691728d2d6926672d12de124237d1842085cc7))
- **fix:** [Stripe] Fix bug in Stripe ([#1505](https://github.com/juspay/hyperswitch/pull/1505)) ([`957d5e0`](https://github.com/juspay/hyperswitch/commit/957d5e0f62ca43d1df3ee39b88ed6c7f6e92a099))
- **refunds:** Refactor refunds create to check for unintended 5xx ([#1332](https://github.com/juspay/hyperswitch/pull/1332)) ([`ff17b62`](https://github.com/juspay/hyperswitch/commit/ff17b62dc27092b6e04d19604e02e8f492c19efb))
- Add serde rename_all for refund enums ([#1520](https://github.com/juspay/hyperswitch/pull/1520)) ([`0c86243`](https://github.com/juspay/hyperswitch/commit/0c8624334c480a42bd5f06fced4f38ab66cdf07f))

### Build System / Dependencies

- **deps:** Bump openssl from 0.10.54 to 0.10.55 ([#1503](https://github.com/juspay/hyperswitch/pull/1503)) ([`c4f9029`](https://github.com/juspay/hyperswitch/commit/c4f9029c8ba3ea2570688e00e551ea979859d3be))

**Full Changelog:** [`v0.6.0...v1.0.0`](https://github.com/juspay/hyperswitch/compare/v0.6.0...v1.0.0)

- - -

## 0.6.0 (2023-06-20)

### Features

- **compatibility:**
  - Add receipt_ipaddress and user_agent in stripe compatibility ([#1417](https://github.com/juspay/hyperswitch/pull/1417)) ([`de2a6e8`](https://github.com/juspay/hyperswitch/commit/de2a6e86d767e77b7ab15b21832747531231453b))
  - Wallet support compatibility layer ([#1214](https://github.com/juspay/hyperswitch/pull/1214)) ([`3e64321`](https://github.com/juspay/hyperswitch/commit/3e64321bfd25cfeb6b02b70188c8e08b3cd4bfcc))
- **connector:**
  - [Noon] Add Card Payments, Capture, Void and Refund ([#1207](https://github.com/juspay/hyperswitch/pull/1207)) ([`2761036`](https://github.com/juspay/hyperswitch/commit/27610361b948c56f3422caa7c70beeb9e87bb69c))
  - [Noon] Add Card Mandates and Webhooks Support ([#1243](https://github.com/juspay/hyperswitch/pull/1243)) ([`ba8a17d`](https://github.com/juspay/hyperswitch/commit/ba8a17d66f12fce01fa3a2d50bd9a5591bf8ef2f))
  - [Noon] Add reference id in Order Struct ([#1371](https://github.com/juspay/hyperswitch/pull/1371)) ([`f0cd5ee`](https://github.com/juspay/hyperswitch/commit/f0cd5ee20d6f8a836f7b1f7117c2d0e43014eaba))
  - [Zen] add apple pay redirect flow support for zen connector ([#1383](https://github.com/juspay/hyperswitch/pull/1383)) ([`b3b16fc`](https://github.com/juspay/hyperswitch/commit/b3b16fcf95321f7ade05ed5b6678dcd851ba6ee5))
  - Mask pii information in connector request and response for stripe, bluesnap, checkout, zen ([#1435](https://github.com/juspay/hyperswitch/pull/1435)) ([`5535159`](https://github.com/juspay/hyperswitch/commit/5535159d5c2cc7278c9e189dcf3629efd67e6fb5))
  - Add request & response logs for top 4 connector ([#1427](https://github.com/juspay/hyperswitch/pull/1427)) ([`1e61f39`](https://github.com/juspay/hyperswitch/commit/1e61f396bd02ca66c7448776a5aab045dc06df10))
  - [Noon] Add GooglePay, ApplePay, PayPal Support ([#1450](https://github.com/juspay/hyperswitch/pull/1450)) ([`8ebcc1c`](https://github.com/juspay/hyperswitch/commit/8ebcc1ce39356307667e8c70be0ed5bdf034ed50))
  - [Zen] add google pay redirect flow support ([#1454](https://github.com/juspay/hyperswitch/pull/1454)) ([`3a225b2`](https://github.com/juspay/hyperswitch/commit/3a225b2118c52f7b28a40a87bbcd8b126b01eeef))
- **core:** Add signature to outgoing webhooks ([#1249](https://github.com/juspay/hyperswitch/pull/1249)) ([`3534cac`](https://github.com/juspay/hyperswitch/commit/3534caca68e18d222c685fa1ea50bc407ee3178e))
- **db:**
  - Implement `RefundInterface` for `MockDb` ([#1277](https://github.com/juspay/hyperswitch/pull/1277)) ([`10691c5`](https://github.com/juspay/hyperswitch/commit/10691c5fce630d60aade862080d25c62a5cddb44))
  - Implement `DisputeInterface` for `MockDb` ([#1345](https://github.com/juspay/hyperswitch/pull/1345)) ([`e5e39a7`](https://github.com/juspay/hyperswitch/commit/e5e39a74911057849748424dbefda7ac26bab45d))
  - Implement `LockerMockInterface` for `MockDb` ([#1347](https://github.com/juspay/hyperswitch/pull/1347)) ([`1322aa7`](https://github.com/juspay/hyperswitch/commit/1322aa757902662a1bd90cc3f09e887a7fdbf841))
  - Implement `MerchantConnectorAccountInterface` for `MockDb` ([#1248](https://github.com/juspay/hyperswitch/pull/1248)) ([`b002c97`](https://github.com/juspay/hyperswitch/commit/b002c97c9c11f7d725aa7ab5b29d49988baa6aea))
  - Implement `MandateInterface` for `MockDb` ([#1387](https://github.com/juspay/hyperswitch/pull/1387)) ([`2555c37`](https://github.com/juspay/hyperswitch/commit/2555c37adab4b0ab10f3e6d507e1b93b3eab1c67))
- **headers:** Add optional header masking feature to outbound request ([#1320](https://github.com/juspay/hyperswitch/pull/1320)) ([`fc6acd0`](https://github.com/juspay/hyperswitch/commit/fc6acd04cb28f02a4f52ec77d8ae003957183ff2))
- **kms:** Reduce redundant kms calls ([#1264](https://github.com/juspay/hyperswitch/pull/1264)) ([`71a17c6`](https://github.com/juspay/hyperswitch/commit/71a17c682e87a708adbea4f2d9f99a4a0172e76e))
- **logging:** Logging the request payload during `BeginRequest` ([#1247](https://github.com/juspay/hyperswitch/pull/1247)) ([`253eead`](https://github.com/juspay/hyperswitch/commit/253eead301bc919ff18af2ebe0064ca004d9852d))
- **metrics:**
  - Add flow-specific metrics ([#1259](https://github.com/juspay/hyperswitch/pull/1259)) ([`5e90a36`](https://github.com/juspay/hyperswitch/commit/5e90a369db32b125414c3674404dc34d134bf1da))
  - Add response metrics ([#1263](https://github.com/juspay/hyperswitch/pull/1263)) ([`4ebd26f`](https://github.com/juspay/hyperswitch/commit/4ebd26f27e43dddeae7498d81ed43516f3eb0e61))
- **order_details:** Adding order_details both inside and outside of metadata, in payments request, for backward compatibility ([#1344](https://github.com/juspay/hyperswitch/pull/1344)) ([`913b833`](https://github.com/juspay/hyperswitch/commit/913b833117e1adb02324d32857dedf050791ec3a))
- **payment:** Customer ip field inclusion ([#1370](https://github.com/juspay/hyperswitch/pull/1370)) ([`11a827a`](https://github.com/juspay/hyperswitch/commit/11a827a76d9efb81b70b4439a681eb17de73b94f))
- **response-log:**
  - Add logging to the response ([#1433](https://github.com/juspay/hyperswitch/pull/1433)) ([`96c5efe`](https://github.com/juspay/hyperswitch/commit/96c5efea2b0edc032d7199046650e7b00a276c5e))
  - Add logging to the response for stripe compatibility layer ([#1470](https://github.com/juspay/hyperswitch/pull/1470)) ([`96c71e1`](https://github.com/juspay/hyperswitch/commit/96c71e1b1bbf2b67a6e2c87478b98bcbb7cdb3ef))
- **router:**
  - Implement `CardsInfoInterface` for `MockDB` ([#1262](https://github.com/juspay/hyperswitch/pull/1262)) ([`cbff605`](https://github.com/juspay/hyperswitch/commit/cbff605f2af257f4f0ba45c1afe276ce902680ab))
  - Add mandate connector to payment data ([#1392](https://github.com/juspay/hyperswitch/pull/1392)) ([`7933e98`](https://github.com/juspay/hyperswitch/commit/7933e98c8cadfa27154b5a7ba5d7d12b33272ec6))
  - [Bluesnap] add kount frms session_id support for bluesnap connector ([#1403](https://github.com/juspay/hyperswitch/pull/1403)) ([`fbaecdc`](https://github.com/juspay/hyperswitch/commit/fbaecdc352e653f81bcc036ce0aabc222c91e92d))
  - Add caching for MerchantKeyStore ([#1409](https://github.com/juspay/hyperswitch/pull/1409)) ([`fda3fb4`](https://github.com/juspay/hyperswitch/commit/fda3fb4d2bc69297a8b8220e44798c4ca9dea9c2))
- Use subscriber client for subscription in pubsub ([#1297](https://github.com/juspay/hyperswitch/pull/1297)) ([`864d855`](https://github.com/juspay/hyperswitch/commit/864d85534fbc174d8e989493c3231497f5c79fe5))
- Encrypt PII fields before saving it in the database ([#1043](https://github.com/juspay/hyperswitch/pull/1043)) ([`fa392c4`](https://github.com/juspay/hyperswitch/commit/fa392c40a86b2589a55c3adf1de5b862a544dbe9))
- Add error type for empty connector list ([#1363](https://github.com/juspay/hyperswitch/pull/1363)) ([`b2da920`](https://github.com/juspay/hyperswitch/commit/b2da9202809089e6725405351f51d81837b08667))
- Add new error response for 403 ([#1330](https://github.com/juspay/hyperswitch/pull/1330)) ([`49d5ad7`](https://github.com/juspay/hyperswitch/commit/49d5ad7b3c24fc9c9847b473fda370398e3c7e38))
- Applepay through trustpay ([#1422](https://github.com/juspay/hyperswitch/pull/1422)) ([`8032e02`](https://github.com/juspay/hyperswitch/commit/8032e0290b0a0ee33a640740b3bd4567a939712c))

### Bug Fixes

- **api_models:** Fix bank namings ([#1315](https://github.com/juspay/hyperswitch/pull/1315)) ([`a8f2494`](https://github.com/juspay/hyperswitch/commit/a8f2494a87a7731deb104fe2eda548fbccdac895))
- **config:** Fix docker compose local setup ([#1372](https://github.com/juspay/hyperswitch/pull/1372)) ([`d21fcc7`](https://github.com/juspay/hyperswitch/commit/d21fcc7bfc3bdf672b9cfbc5a234a3f3d03771c8))
- **connector:**
  - [Authorizedotnet] Fix webhooks ([#1261](https://github.com/juspay/hyperswitch/pull/1261)) ([`776c833`](https://github.com/juspay/hyperswitch/commit/776c833de706dcd8e93786d1fa294769669303cd))
  - [Checkout] Fix error message in error handling ([#1221](https://github.com/juspay/hyperswitch/pull/1221)) ([`22b2fa3`](https://github.com/juspay/hyperswitch/commit/22b2fa30610ad5ca97cd53df187ccd994411f4e2))
  - [coinbase] remove non-mandatory fields ([#1252](https://github.com/juspay/hyperswitch/pull/1252)) ([`bfd7dad`](https://github.com/juspay/hyperswitch/commit/bfd7dad2f1d694bbdc0a18782732ff95ee7730f6))
  - [Rapyd] Fix payment response structure ([#1258](https://github.com/juspay/hyperswitch/pull/1258)) ([`3af3a3c`](https://github.com/juspay/hyperswitch/commit/3af3a3cb39641633aecc2d4fc120ece13a6ee72a))
  - [Adyen] Address Internal Server Error when calling PSync without redirection ([#1311](https://github.com/juspay/hyperswitch/pull/1311)) ([`b966525`](https://github.com/juspay/hyperswitch/commit/b96652507a6ab37f3c75aeb0cf715fd6454b9f32))
  - [opennode] webhook url fix ([#1364](https://github.com/juspay/hyperswitch/pull/1364)) ([`e484193`](https://github.com/juspay/hyperswitch/commit/e484193101ffdc693044fd43c130b12821256111))
  - [Zen] fix additional base url required for Zen apple pay checkout integration ([#1394](https://github.com/juspay/hyperswitch/pull/1394)) ([`7955007`](https://github.com/juspay/hyperswitch/commit/795500797d1061630b5ca493187a4e19d98d26c0))
  - [Bluesnap] Throw proper error message for redirection scenario ([#1367](https://github.com/juspay/hyperswitch/pull/1367)) ([`4a8de77`](https://github.com/juspay/hyperswitch/commit/4a8de7741d43da07e655bc7382927c68e8ac1eb5))
  - [coinbase][opennode][bitpay] handle error response ([#1406](https://github.com/juspay/hyperswitch/pull/1406)) ([`301c3dc`](https://github.com/juspay/hyperswitch/commit/301c3dc44bb740709a0c5c54ee95fc811c2897ed))
  - [Zen][ACI] Error handling and Mapping ([#1436](https://github.com/juspay/hyperswitch/pull/1436)) ([`8a4f4a4`](https://github.com/juspay/hyperswitch/commit/8a4f4a4c307d75dee8a1fac8f029091a7d49d432))
  - [Bluesnap] fix expiry year ([#1426](https://github.com/juspay/hyperswitch/pull/1426)) ([`92c8222`](https://github.com/juspay/hyperswitch/commit/92c822257e3780fce11f7f0df9a0db48ae1b86e0))
  - [Shift4]Add Refund webhooks ([#1307](https://github.com/juspay/hyperswitch/pull/1307)) ([`1691bea`](https://github.com/juspay/hyperswitch/commit/1691beacc3a1c604ce6a1f4cf236f5f7932ed7ae))
  - [Shift4] validate pretask for threeds cards ([#1428](https://github.com/juspay/hyperswitch/pull/1428)) ([`2c1dcff`](https://github.com/juspay/hyperswitch/commit/2c1dcff046407aa9053b96c549ab578988b5c618))
  - Fix trustpay error response for transaction status api ([#1445](https://github.com/juspay/hyperswitch/pull/1445)) ([`7db94a6`](https://github.com/juspay/hyperswitch/commit/7db94a620882d7b4d14ac97f35f6ced06ae529d7))
  - Fix for sending refund_amount in connectors refund request ([#1278](https://github.com/juspay/hyperswitch/pull/1278)) ([`016857f`](https://github.com/juspay/hyperswitch/commit/016857fff0681058f3321a7952c7bd917442293a))
  - Use reference as payment_id in trustpay ([#1444](https://github.com/juspay/hyperswitch/pull/1444)) ([`3645c49`](https://github.com/juspay/hyperswitch/commit/3645c49b3830e6dc9e23d91b3ac66213727dca9f))
  - Implement ConnectorErrorExt for error_stack::Result<T, ConnectorError> ([#1382](https://github.com/juspay/hyperswitch/pull/1382)) ([`3ef1d29`](https://github.com/juspay/hyperswitch/commit/3ef1d2935e32a8b581e4d2d7f328d970ade4b7f9))
  - [Adyen] fix charged status for Auto capture payment ([#1462](https://github.com/juspay/hyperswitch/pull/1462)) ([`6c818ef`](https://github.com/juspay/hyperswitch/commit/6c818ef3366e9f094d39523334199a9a3abb78e9))
  - [Adyen] fix unit test ([#1469](https://github.com/juspay/hyperswitch/pull/1469)) ([`6e581c6`](https://github.com/juspay/hyperswitch/commit/6e581c6060423af9984375ad4169a1fec94d4585))
  - [Airwallex] Fix refunds ([#1468](https://github.com/juspay/hyperswitch/pull/1468)) ([`1b2841b`](https://github.com/juspay/hyperswitch/commit/1b2841be5997083cd2e414fc698d3d39f9c24c04))
  - [Zen] Convert the amount to base denomination in order_details ([#1477](https://github.com/juspay/hyperswitch/pull/1477)) ([`7ca62d3`](https://github.com/juspay/hyperswitch/commit/7ca62d3c7c04997c7eed6e82ec02dc39ea046b2f))
  - [Shift4] Fix incorrect deserialization of webhook event type ([#1463](https://github.com/juspay/hyperswitch/pull/1463)) ([`b44f35d`](https://github.com/juspay/hyperswitch/commit/b44f35d4d9ddf4fcd725f0e0a5d51fa9eb7f7e3f))
  - [Trustpay] add missing failure status ([#1485](https://github.com/juspay/hyperswitch/pull/1485)) ([`ecf16b0`](https://github.com/juspay/hyperswitch/commit/ecf16b0c7437fefa3550db7275ca2f73d1499b72))
  - [Trustpay] add reason to all the error responses ([#1482](https://github.com/juspay/hyperswitch/pull/1482)) ([`1d216db`](https://github.com/juspay/hyperswitch/commit/1d216db5ceeac3dc61d672de89a921501dcaee45))
- **core:**
  - Remove `missing_required_field_error` being thrown in `should_add_task_to_process_tracker` function ([#1239](https://github.com/juspay/hyperswitch/pull/1239)) ([`3857d06`](https://github.com/juspay/hyperswitch/commit/3857d06627d4c1b85b2e5b9687d80298acf82c14))
  - Return an empty array when the customer does not have any payment methods ([#1431](https://github.com/juspay/hyperswitch/pull/1431)) ([`6563587`](https://github.com/juspay/hyperswitch/commit/6563587564a6de579888a751b8c21e832060d728))
  - Fix amount capturable in payments response ([#1437](https://github.com/juspay/hyperswitch/pull/1437)) ([`5bc1aab`](https://github.com/juspay/hyperswitch/commit/5bc1aaba5945bd829bc2dffcef59db074fa523a7))
  - Save payment_method_type when creating a record in the payment_method table ([#1378](https://github.com/juspay/hyperswitch/pull/1378)) ([`76cb15e`](https://github.com/juspay/hyperswitch/commit/76cb15e01de748f9328d57968d6ddee9831720aa))
  - Add validation for card expiry month, expiry year and card cvc ([#1416](https://github.com/juspay/hyperswitch/pull/1416)) ([`c40617a`](https://github.com/juspay/hyperswitch/commit/c40617aea66eb3c14ad47efbce28374cd28626e0))
- **currency:** Add RON and TRY currencies ([#1455](https://github.com/juspay/hyperswitch/pull/1455)) ([`495a98f`](https://github.com/juspay/hyperswitch/commit/495a98f0454787ae322f63f2adc3e3a6b6e0b515))
- **error:** Propagate MissingRequiredFields api_error ([#1244](https://github.com/juspay/hyperswitch/pull/1244)) ([`798881a`](https://github.com/juspay/hyperswitch/commit/798881ab5b0e7a095daad9e920a29c36961ec13d))
- **kms:** Add metrics to external_services kms ([#1237](https://github.com/juspay/hyperswitch/pull/1237)) ([`28f0d1f`](https://github.com/juspay/hyperswitch/commit/28f0d1f5351f0d3f6abd982ebe99bc15a74797c2))
- **list:** Add mandate type in payment_method_list ([#1238](https://github.com/juspay/hyperswitch/pull/1238)) ([`9341191`](https://github.com/juspay/hyperswitch/commit/9341191e39627b661b9d105d65a869e8348c81ed))
- **locker:** Remove unnecessary assertions for locker_id on BasiliskLocker when saving cards ([#1337](https://github.com/juspay/hyperswitch/pull/1337)) ([`23458bc`](https://github.com/juspay/hyperswitch/commit/23458bc42776e6440e76d324d37f36b65c393451))
- **logging:** Fix traces export through opentelemetry ([#1355](https://github.com/juspay/hyperswitch/pull/1355)) ([`b2b9dc0`](https://github.com/juspay/hyperswitch/commit/b2b9dc0b58d737ea114d078fe02271a10accaefa))
- **payments:** Do not delete client secret on payment failure ([#1226](https://github.com/juspay/hyperswitch/pull/1226)) ([`c1b631b`](https://github.com/juspay/hyperswitch/commit/c1b631bd1e0025452f2cf37345996ea789810839))
- **refund:** Change amount to refund_amount ([#1268](https://github.com/juspay/hyperswitch/pull/1268)) ([`24c3a42`](https://github.com/juspay/hyperswitch/commit/24c3a42898a37dccf3f99a9fcc259127606598dd))
- **router:**
  - Subscriber return type ([#1292](https://github.com/juspay/hyperswitch/pull/1292)) ([`55bb117`](https://github.com/juspay/hyperswitch/commit/55bb117e1ddc147d7309823dc593bd1a05fe69a9))
  - Hotfixes for stripe webhook event mapping and reference id retrieval ([#1368](https://github.com/juspay/hyperswitch/pull/1368)) ([`5c2232b`](https://github.com/juspay/hyperswitch/commit/5c2232b737f5430a68fdf6cba9aa5f4c1d6cf3e2))
  - [Trustpay] fix email & user-agent information as mandatory fields in trustpay card payment request ([#1414](https://github.com/juspay/hyperswitch/pull/1414)) ([`7ef011a`](https://github.com/juspay/hyperswitch/commit/7ef011ad737257fc83f7a43d16f1bf4ac54336ae))
  - [Trustpay] fix email & user-agent information as mandatory fields in trustpay card payment request ([#1418](https://github.com/juspay/hyperswitch/pull/1418)) ([`c596d12`](https://github.com/juspay/hyperswitch/commit/c596d121a846e6c0fa399b8f28ffe4ab6124651a))
  - Fix payment status updation for 2xx error responses ([#1457](https://github.com/juspay/hyperswitch/pull/1457)) ([`a7ac4af`](https://github.com/juspay/hyperswitch/commit/a7ac4af5d916ff1e7965be35f347ce0e13407747))
- **router/webhooks:**
  - Use api error response for returning errors from webhooks core ([#1305](https://github.com/juspay/hyperswitch/pull/1305)) ([`cd0cf40`](https://github.com/juspay/hyperswitch/commit/cd0cf40fe29358700f92c1520475934752bb4b30))
  - Correct webhook error mapping and make source verification optional for all connectors ([#1333](https://github.com/juspay/hyperswitch/pull/1333)) ([`7131509`](https://github.com/juspay/hyperswitch/commit/71315097dd01ee675b0e4df3087b930637de416c))
  - Map webhook event type not found errors to 422 ([#1340](https://github.com/juspay/hyperswitch/pull/1340)) ([`61bacd8`](https://github.com/juspay/hyperswitch/commit/61bacd8c9590a78a6d5067e378bfed6301d64d07))
- **session_token:** Log error only when it occurs ([#1136](https://github.com/juspay/hyperswitch/pull/1136)) ([`ebf3de4`](https://github.com/juspay/hyperswitch/commit/ebf3de41018f131f7501b17936e58c05276ead77))
- **stripe:** Fix logs on stripe connector integration ([#1448](https://github.com/juspay/hyperswitch/pull/1448)) ([`c42b436`](https://github.com/juspay/hyperswitch/commit/c42b436abe1ed980d9b861dd4ba56324c8361a5a))
- Remove multiple call to locker ([#1230](https://github.com/juspay/hyperswitch/pull/1230)) ([`b3c6b1f`](https://github.com/juspay/hyperswitch/commit/b3c6b1f0aacb9950d225779aa7de1ac49fe148d2))
- Populate meta_data in payment_intent ([#1240](https://github.com/juspay/hyperswitch/pull/1240)) ([`1ac3eb0`](https://github.com/juspay/hyperswitch/commit/1ac3eb0a36030412ef51ec2664e8af43c9c2fc54))
- Merchant webhook config should be looked up in config table instead of redis ([#1241](https://github.com/juspay/hyperswitch/pull/1241)) ([`48e5375`](https://github.com/juspay/hyperswitch/commit/48e537568debccdcd01c78eabce0b480a96beda2))
- Invalidation of in-memory cache ([#1270](https://github.com/juspay/hyperswitch/pull/1270)) ([`e78b3a6`](https://github.com/juspay/hyperswitch/commit/e78b3a65d45429357adf3534b6028798d1f68620))
- Customer id is not mandatory during confirm ([#1317](https://github.com/juspay/hyperswitch/pull/1317)) ([`1261791`](https://github.com/juspay/hyperswitch/commit/1261791d9f70794b3d6426ff35f4eb0fc1076be0))
- Certificate decode failed when creating the session token for applepay ([#1385](https://github.com/juspay/hyperswitch/pull/1385)) ([`8497c55`](https://github.com/juspay/hyperswitch/commit/8497c55283d548c04b3a01560b06d9594e7d634c))
- Update customer data if passed in payments ([#1402](https://github.com/juspay/hyperswitch/pull/1402)) ([`86f679a`](https://github.com/juspay/hyperswitch/commit/86f679abc1549b59239ece4a1123b60e40c26b96))
- Fix some fields not being updated during payments create, update and confirm ([#1451](https://github.com/juspay/hyperswitch/pull/1451)) ([`1764085`](https://github.com/juspay/hyperswitch/commit/17640858eabb5d5a56a17c9e0a52e5773a0c592f))

### Refactors

- **api_models:** Follow naming convention for wallets & paylater payment method data enums ([#1338](https://github.com/juspay/hyperswitch/pull/1338)) ([`6c0d136`](https://github.com/juspay/hyperswitch/commit/6c0d136cee106fc25fbcf63e4bbc01b28baa1519))
- **auth_type:** Updated auth type in `update tracker` and also changed the default flow to `non-3ds` from `3ds` ([#1424](https://github.com/juspay/hyperswitch/pull/1424)) ([`1616051`](https://github.com/juspay/hyperswitch/commit/1616051145c1e276fdd7d0f85cda76baaeaa0023))
- **compatibility:** Map connector to routing in payments request for backward compatibility ([#1339](https://github.com/juspay/hyperswitch/pull/1339)) ([`166688a`](https://github.com/juspay/hyperswitch/commit/166688a5906a2fcbb034c40a113452f6dc2e7160))
- **compatibility, connector:** Add holder name and change trust pay merchant_ref id to payment_id ([`d091549`](https://github.com/juspay/hyperswitch/commit/d091549576676c87f855e06678544704339d82e4))
- **configs:** Make kms module and KmsDecrypt pub ([#1274](https://github.com/juspay/hyperswitch/pull/1274)) ([`f0db993`](https://github.com/juspay/hyperswitch/commit/f0db9937c7b33858a1ff3e17eaecba094ca4c18c))
- **connector:**
  - Update error handling for Nexinets, Cybersource ([#1151](https://github.com/juspay/hyperswitch/pull/1151)) ([`2ede8ad`](https://github.com/juspay/hyperswitch/commit/2ede8ade8cff56443d8712518c64de7d952f4a0c))
  - [Zen] refactor connector_meta_data for zen connector applepay session data ([#1390](https://github.com/juspay/hyperswitch/pull/1390)) ([`0575b26`](https://github.com/juspay/hyperswitch/commit/0575b26b4fc229e92aef179146dfd561a9ee7f27))
- **connector_customer:** Incorrect mapping of connector customer ([#1275](https://github.com/juspay/hyperswitch/pull/1275)) ([`ebdfde7`](https://github.com/juspay/hyperswitch/commit/ebdfde75ecc1c39720396ad7c18062f5c108b8d3))
- **core:**
  - Generate response hash key if not specified in create merchant account request ([#1232](https://github.com/juspay/hyperswitch/pull/1232)) ([`7b74cab`](https://github.com/juspay/hyperswitch/commit/7b74cab385db68e510d2d513083a725a4f945ae3))
  - Add 'redirect_response' field to CompleteAuthorizeData ([#1222](https://github.com/juspay/hyperswitch/pull/1222)) ([`77e60c8`](https://github.com/juspay/hyperswitch/commit/77e60c82fa123ef780485a8507ce779f2f41e166))
  - Use HMAC-SHA512 to calculate payments response hash ([#1302](https://github.com/juspay/hyperswitch/pull/1302)) ([`7032ea8`](https://github.com/juspay/hyperswitch/commit/7032ea849416cb740c892360d21e436d2675fbe4))
  - Accept customer data in customer object ([#1447](https://github.com/juspay/hyperswitch/pull/1447)) ([`cff1ce6`](https://github.com/juspay/hyperswitch/commit/cff1ce61f0347665d18040486cfbbcd93139950b))
  - Move update trackers after build request ([#1472](https://github.com/juspay/hyperswitch/pull/1472)) ([`6114fb6`](https://github.com/juspay/hyperswitch/commit/6114fb634063a9a6d732af38e2a9e343d940a15e))
  - Update trackers for preprocessing steps ([#1481](https://github.com/juspay/hyperswitch/pull/1481)) ([`8fffc16`](https://github.com/juspay/hyperswitch/commit/8fffc161ea909fb29a81090f97ee9f811431d539))
- **disputes:** Resolve incorrect 5xx error mappings for disputes ([#1360](https://github.com/juspay/hyperswitch/pull/1360)) ([`c9b400e`](https://github.com/juspay/hyperswitch/commit/c9b400e186731b7de6073fece662fd0fcbbfc953))
- **errors:**
  - Remove RedisErrorExt ([#1389](https://github.com/juspay/hyperswitch/pull/1389)) ([`5d51505`](https://github.com/juspay/hyperswitch/commit/5d515050cf77705e3bf8c4b83f81ee51a8bff052))
  - Refactor `actix_web::ResponseError` for `ApiErrorResponse` ([#1362](https://github.com/juspay/hyperswitch/pull/1362)) ([`02a3ce7`](https://github.com/juspay/hyperswitch/commit/02a3ce74b84e86b0e17f8809c9b7651998a1c864))
- **fix:**
  - [Stripe] Fix bug in Stripe ([#1412](https://github.com/juspay/hyperswitch/pull/1412)) ([`e48202e`](https://github.com/juspay/hyperswitch/commit/e48202e0a06fa4d61a2637f57830ffa4aae1335d))
  - [Adyen] Fix bug in Adyen ([#1375](https://github.com/juspay/hyperswitch/pull/1375)) ([`d3a6906`](https://github.com/juspay/hyperswitch/commit/d3a69060b4db24fbbfc5c03934684dd8bfd45711))
- **mca:** Use separate struct for connector metadata ([#1465](https://github.com/juspay/hyperswitch/pull/1465)) ([`8d20578`](https://github.com/juspay/hyperswitch/commit/8d2057844ef4a29474d266d814c8ee01cc557961))
- **payments:**
  - Attempt to address unintended 5xx and 4xx in payments ([#1376](https://github.com/juspay/hyperswitch/pull/1376)) ([`cf64862`](https://github.com/juspay/hyperswitch/commit/cf64862daca0ad05b7af27646430d12bac71a5ee))
  - Add udf field and remove refactor metadata ([#1466](https://github.com/juspay/hyperswitch/pull/1466)) ([`6419953`](https://github.com/juspay/hyperswitch/commit/641995371db4deba13dc246179d726ed390b6b3e))
- **process_tracker:** Attempt to identify unintended 5xx in process_tracker ([#1359](https://github.com/juspay/hyperswitch/pull/1359)) ([`d8adf4c`](https://github.com/juspay/hyperswitch/commit/d8adf4c2b542a5cdd7888b956b085a69bd900920))
- **router:**
  - Router_parameters field inclusion ([#1251](https://github.com/juspay/hyperswitch/pull/1251)) ([`16cd325`](https://github.com/juspay/hyperswitch/commit/16cd32513bc6528e064058907a8c3c848fdba132))
  - Remove `pii-encryption-script` feature and use of timestamps for decryption ([#1350](https://github.com/juspay/hyperswitch/pull/1350)) ([`9f2832f`](https://github.com/juspay/hyperswitch/commit/9f2832f60078b98e6faae34b05b63d2dab6b7969))
  - Infer ip address for online mandates from request headers if absent ([#1419](https://github.com/juspay/hyperswitch/pull/1419)) ([`a1a009d`](https://github.com/juspay/hyperswitch/commit/a1a009d7966d2354d12bba86fbc59c1b853e14a1))
  - Send 200 response for 5xx status codes received from connector ([#1440](https://github.com/juspay/hyperswitch/pull/1440)) ([`1e5d2a2`](https://github.com/juspay/hyperswitch/commit/1e5d2a28f6592106a5924044fc8d6fc49ab20acf))
- **webhook:** Added the unknown field to the webhook_event_status of every connector ([#1343](https://github.com/juspay/hyperswitch/pull/1343)) ([`65d4a95`](https://github.com/juspay/hyperswitch/commit/65d4a95b59ee950ba67ce5b38688a650c5131149))
- Make NextAction as enum ([#1234](https://github.com/juspay/hyperswitch/pull/1234)) ([`a359b76`](https://github.com/juspay/hyperswitch/commit/a359b76d09ffc581d5808e3750dac7326c389876))
- Make bank names optional in payment method data ([#1483](https://github.com/juspay/hyperswitch/pull/1483)) ([`8198559`](https://github.com/juspay/hyperswitch/commit/8198559966313ab147161eb72c07a230ecebb70c))

### Testing

- **connector:**
  - [Stripe] Fix redirection UI tests ([#1215](https://github.com/juspay/hyperswitch/pull/1215)) ([`ea6bce6`](https://github.com/juspay/hyperswitch/commit/ea6bce663dcb084b5990834cb922eec5c626e897))
  - [Globalpay] Fix unit tests ([#1217](https://github.com/juspay/hyperswitch/pull/1217)) ([`71c0d4c`](https://github.com/juspay/hyperswitch/commit/71c0d4c500f7daca7b00f737d714f2d98cc91513))
- **postman-collection:** Add Github action to run postman collection ([#1272](https://github.com/juspay/hyperswitch/pull/1272)) ([`92c7767`](https://github.com/juspay/hyperswitch/commit/92c776714f63d02055fc46b5b750cee71328f5eb))
- **selenium:** Read config from `CONNECTOR_AUTH_FILE_PATH` environment variable and fix bugs in UI tests ([#1225](https://github.com/juspay/hyperswitch/pull/1225)) ([`d9a16ed`](https://github.com/juspay/hyperswitch/commit/d9a16ed5abdafa6d48bf30a6ba8c3783bed3dff5))

### Documentation

- **CONTRIBUTING:** Update commit guidelines ([#1351](https://github.com/juspay/hyperswitch/pull/1351)) ([`5d8895c`](https://github.com/juspay/hyperswitch/commit/5d8895c06412ff05d5abbfefaaa4933db853eb13))
- Add changelog to 0.5.15 ([#1216](https://github.com/juspay/hyperswitch/pull/1216)) ([`0be900d`](https://github.com/juspay/hyperswitch/commit/0be900d2388ad732a40b788223bd48aee9b3aa95))
- Add `ApplePayRedirectionData` to OpenAPI schema ([#1386](https://github.com/juspay/hyperswitch/pull/1386)) ([`d0d3254`](https://github.com/juspay/hyperswitch/commit/d0d32544c23481a1acd91182055a7a0afb78d723))

### Miscellaneous Tasks

- **common_utils:** Apply the new type pattern for phone numbers ([#1286](https://github.com/juspay/hyperswitch/pull/1286)) ([`98e73e2`](https://github.com/juspay/hyperswitch/commit/98e73e2e90b4c79d0cc6cf8682693c1e5aad50a3))
- **config:**
  - Add bank config for online_banking_poland, online_banking_slovakia ([#1220](https://github.com/juspay/hyperswitch/pull/1220)) ([`ee5466a`](https://github.com/juspay/hyperswitch/commit/ee5466a3b04f69c92dc5d04faca80d1f04275a9c))
  - Add bank config for przelewy24 ([#1460](https://github.com/juspay/hyperswitch/pull/1460)) ([`3ee97cd`](https://github.com/juspay/hyperswitch/commit/3ee97cda552e0745b8d75daad2e300288673a4d7))
- **migrations:** Shrink `merchant_id` column of `merchant_key_store` to 64 characters ([#1476](https://github.com/juspay/hyperswitch/pull/1476)) ([`0fdd6ec`](https://github.com/juspay/hyperswitch/commit/0fdd6ecd4ac4bc5e1fc11e5cf79292c99eae71c1))
- Address Rust 1.70 clippy lints ([#1334](https://github.com/juspay/hyperswitch/pull/1334)) ([`b681f78`](https://github.com/juspay/hyperswitch/commit/b681f78d964d02f80249751cc6fd12e3c85bc4d7))

### Build System / Dependencies

- **deps:**
  - Bump `diesel` from `2.0.3` to `2.1.0` ([#1287](https://github.com/juspay/hyperswitch/pull/1287)) ([`b9ec38a`](https://github.com/juspay/hyperswitch/commit/b9ec38a1b54abbaa90bbc967aa8cdd450f149947))
  - Update dependencies ([#1342](https://github.com/juspay/hyperswitch/pull/1342)) ([`bce01ce`](https://github.com/juspay/hyperswitch/commit/bce01ced11e3869699d454827dc659fc82941951))
- **docker:** Use `debian:bookworm-slim` as base image for builder and runner stages ([#1473](https://github.com/juspay/hyperswitch/pull/1473)) ([`5eb0333`](https://github.com/juspay/hyperswitch/commit/5eb033336321b5deb197f4416c8409abf99a8421))
- Unify `sandbox` and `production` cargo features as `release` ([#1356](https://github.com/juspay/hyperswitch/pull/1356)) ([`695d3cd`](https://github.com/juspay/hyperswitch/commit/695d3cdac27448806fcde8cbb9cdc6ba4e7cbe7e))

**Full Changelog:** [`v0.5.15...v0.6.0`](https://github.com/juspay/hyperswitch/compare/v0.5.15...v0.6.0)

- - -

## 0.5.15 (2023-05-19)

### Features

- **connector:**
  - [Bluesnap] Add support for ApplePay ([#1178](https://github.com/juspay/hyperswitch/pull/1178)) ([`919c03e`](https://github.com/juspay/hyperswitch/commit/919c03e679c4ebbb138509da52a18bface7ba319))
  - Add Interac as Payment Method Type ([#1205](https://github.com/juspay/hyperswitch/pull/1205)) ([`afceda5`](https://github.com/juspay/hyperswitch/commit/afceda55ad9741909e21a3c3956d78b5ba858746))
  - [Authorizedotnet] implement Capture flow and webhooks for Authorizedotnet ([#1171](https://github.com/juspay/hyperswitch/pull/1171)) ([`2d49ce5`](https://github.com/juspay/hyperswitch/commit/2d49ce56de5ed314aa099f3ce4aa569b3e22b561))
- **db:** Implement `AddressInterface` for `MockDb` ([#968](https://github.com/juspay/hyperswitch/pull/968)) ([`39405bb`](https://github.com/juspay/hyperswitch/commit/39405bb4788bf88d6c8c166281fffc238a589aaa))
- **documentation:** Add polymorphic `generate_schema` macro ([#1183](https://github.com/juspay/hyperswitch/pull/1183)) ([`53aa5ac`](https://github.com/juspay/hyperswitch/commit/53aa5ac92d0692b753624a4254040f8452def1d2))
- **email:** Integrate email service using AWS SES ([#1158](https://github.com/juspay/hyperswitch/pull/1158)) ([`07e0fcb`](https://github.com/juspay/hyperswitch/commit/07e0fcbe06107e8be532b4e9a1e1a1ef6efba68e))
- **frm_routing_algorithm:** Added frm_routing_algorithm to merchant_account table, to be consumed for frm selection ([#1161](https://github.com/juspay/hyperswitch/pull/1161)) ([`ea98145`](https://github.com/juspay/hyperswitch/commit/ea9814531880584435c122b3e32e9883e4518fd2))
- **payments:** Add support for manual retries in payments confirm call ([#1170](https://github.com/juspay/hyperswitch/pull/1170)) ([`1f52a66`](https://github.com/juspay/hyperswitch/commit/1f52a66452042deb0e3959e839a726f261cce880))
- **redis_interface:** Implement `MGET` command ([#1206](https://github.com/juspay/hyperswitch/pull/1206)) ([`93dcd98`](https://github.com/juspay/hyperswitch/commit/93dcd98640a31e41f0d66d2ece2396e536adefae))
- **router:**
  - Implement `ApiKeyInterface` for `MockDb` ([#1101](https://github.com/juspay/hyperswitch/pull/1101)) ([`95c7ca9`](https://github.com/juspay/hyperswitch/commit/95c7ca99d1b5009f4cc8664825c5e63a165006c7))
  - Add mandates list api ([#1143](https://github.com/juspay/hyperswitch/pull/1143)) ([`commit`](https://github.com/juspay/hyperswitch/commit/75ba3ff09f71d1dd295f9dad0060d2620d7b3764))
- **traces:** Add support for aws xray ([#1194](https://github.com/juspay/hyperswitch/pull/1194)) ([`8947e1c`](https://github.com/juspay/hyperswitch/commit/8947e1c9dba3585c3d998110b53747cbc1007bc2))
- ACH transfers ([#905](https://github.com/juspay/hyperswitch/pull/905)) ([`23bca66`](https://github.com/juspay/hyperswitch/commit/23bca66b810993895e4054cc4bf3fdcac6b2ed4c))
- SEPA and BACS bank transfers through stripe ([#930](https://github.com/juspay/hyperswitch/pull/930)) ([`cf00059`](https://github.com/juspay/hyperswitch/commit/cf000599ddaca2646efce0493a013c06fcdf34b8))

### Bug Fixes

- **connector:** [Checkout] Fix incoming webhook event mapping ([#1197](https://github.com/juspay/hyperswitch/pull/1197)) ([`912a108`](https://github.com/juspay/hyperswitch/commit/912a1084846b6dc8e843e852a3b664a4faaf9f00))
- **core:** Add ephemeral key to payment_create response when customer_id is mentioned ([#1133](https://github.com/juspay/hyperswitch/pull/1133)) ([`f394c4a`](https://github.com/juspay/hyperswitch/commit/f394c4abc071b314798943024ba22d653a6a056e))
- **mandate:** Throw DuplicateMandate Error if mandate insert fails ([#1201](https://github.com/juspay/hyperswitch/pull/1201)) ([`186bd72`](https://github.com/juspay/hyperswitch/commit/186bd729d672290e0f49eac0cebb3dcb8948f992))
- **merchant_connector_account:** Add validation for the `disabled` flag ([#1141](https://github.com/juspay/hyperswitch/pull/1141)) ([`600dc33`](https://github.com/juspay/hyperswitch/commit/600dc338673c593c3cbd3ad8dfebe17d4f5c0326))
- **router:**
  - Aggregate critical hotfixes for v0.5.10 ([#1162](https://github.com/juspay/hyperswitch/pull/1162)) ([`ed22b2a`](https://github.com/juspay/hyperswitch/commit/ed22b2af763425d4e71cccd8da158e5e95722fed))
  - Outgoing webhook api call ([#1193](https://github.com/juspay/hyperswitch/pull/1193)) ([`31a52d8`](https://github.com/juspay/hyperswitch/commit/31a52d8058dbee38cd77de20b7cae7c5d6fb23bf))
  - Add dummy connector url to proxy bypass ([#1186](https://github.com/juspay/hyperswitch/pull/1186)) ([`bc5497f`](https://github.com/juspay/hyperswitch/commit/bc5497f03ab7fde585e7c57815f55cf7b4b8d475))
  - Aggregate hotfixes for v0.5.10 ([#1204](https://github.com/juspay/hyperswitch/pull/1204)) ([`9cc1cee`](https://github.com/juspay/hyperswitch/commit/9cc1ceec6986f5696030d95e6899730807637cd9))
- **utils:** Fix bug in email validation ([#1180](https://github.com/juspay/hyperswitch/pull/1180)) ([`5e51b6b`](https://github.com/juspay/hyperswitch/commit/5e51b6b16db6830dd5051b43fbd7d43532d9f195))
- Fix(connector) : Added signifyd to routableconnectors for frm ([#1182](https://github.com/juspay/hyperswitch/pull/1182)) ([`2ce5d5f`](https://github.com/juspay/hyperswitch/commit/2ce5d5ffe4e37de29749fc97b13d2faaec8fcee0))
- Handle unique constraint violation error gracefully ([#1202](https://github.com/juspay/hyperswitch/pull/1202)) ([`b3fd174`](https://github.com/juspay/hyperswitch/commit/b3fd174d04cdd4b26328d36c4f886e6ef4df830d))

### Refactors

- **mandate:** Allow merchant to pass the mandate details and customer acceptance separately ([#1188](https://github.com/juspay/hyperswitch/pull/1188)) ([`6c41cdb`](https://github.com/juspay/hyperswitch/commit/6c41cdb1c942d3152c73a44b62dd9a02587f6bd8))
- Use `strum::EnumString` implementation for connector name conversions ([#1052](https://github.com/juspay/hyperswitch/pull/1052)) ([`2809425`](https://github.com/juspay/hyperswitch/commit/28094251546b6067a44df8ae906d9cd04f85e84e))

### Documentation

- Add changelog for v0.5.14 ([#1177](https://github.com/juspay/hyperswitch/pull/1177)) ([`236124d`](https://github.com/juspay/hyperswitch/commit/236124d1993c2a7d52e30441761a3558ad02c973))

### Miscellaneous Tasks

- **CODEOWNERS:** Add hyperswitch-maintainers as default owners for all files ([#1210](https://github.com/juspay/hyperswitch/pull/1210)) ([`985670d`](https://github.com/juspay/hyperswitch/commit/985670da9c90cbc904162d7863c9c508f5cf5e19))
- **git-cliff:** Simplify `git-cliff` config files ([#1213](https://github.com/juspay/hyperswitch/pull/1213)) ([`bd0069e`](https://github.com/juspay/hyperswitch/commit/bd0069e2a8bd3c3389c92590c688ce945cd7ebec))

### Revert

- **connector:** Fix stripe status to attempt status map ([#1179](https://github.com/juspay/hyperswitch/pull/1179)) ([`bd8868e`](https://github.com/juspay/hyperswitch/commit/bd8868efd00748cf64c46519c4ed7ba04ad06d5e))
- Fix(connector): Added signifyd to routableconnectors for frm ([#1203](https://github.com/juspay/hyperswitch/pull/1203)) ([`dbc5bc5`](https://github.com/juspay/hyperswitch/commit/dbc5bc538a218ae287e96c44de0223c26c1583f0))

- - -

## 0.5.14 (2023-05-16)

### Features

- **connector:**
  - [Stripe] implement Bancontact Bank Redirect for stripe ([#1169](https://github.com/juspay/hyperswitch/pull/1169)) ([`5b22e96`](https://github.com/juspay/hyperswitch/commit/5b22e967981b604be6070f5b373555756a5c62f7))
  - [Noon] Add script generated template code ([#1164](https://github.com/juspay/hyperswitch/pull/1164)) ([`bfaf75f`](https://github.com/juspay/hyperswitch/commit/bfaf75fca38e535ceb3ea4327e252d807fb61892))
  - [Adyen] implement BACS Direct Debits for Adyen ([#1159](https://github.com/juspay/hyperswitch/pull/1159)) ([`9f47f20`](https://github.com/juspay/hyperswitch/commit/9f47f2070216eb8c64db14eae555073a507cc634))
- **router:** Add retrieve dispute evidence API ([#1114](https://github.com/juspay/hyperswitch/pull/1114)) ([`354ee01`](https://github.com/juspay/hyperswitch/commit/354ee0137a968862e545d9b437ade27aa0b0f8f3))
- Add accounts in-memory cache ([#1086](https://github.com/juspay/hyperswitch/pull/1086)) ([`da4d721`](https://github.com/juspay/hyperswitch/commit/da4d721424d329af618a63034aabe2d9248eb041))

### Bug Fixes

- **connector:**
  - [Checkout] Change error handling condition for empty response ([#1168](https://github.com/juspay/hyperswitch/pull/1168)) ([`e3fcfdd`](https://github.com/juspay/hyperswitch/commit/e3fcfdd3377df298058b5e1f69f0e553c09ac603))
  - Change payment method handling in dummy connector ([#1175](https://github.com/juspay/hyperswitch/pull/1175)) ([`32a3722`](https://github.com/juspay/hyperswitch/commit/32a3722f073c3ea22220abfa62034e476ee8acef))

### Refactors

- **connector:** Update error handling for Paypal, Checkout, Mollie to include detailed messages ([#1150](https://github.com/juspay/hyperswitch/pull/1150)) ([`e044c2f`](https://github.com/juspay/hyperswitch/commit/e044c2fd9a4464e59ffc372b9333af6acbc9809a))

### Documentation

- **CHANGELOG:** Add changelog for 0.5.13 ([#1166](https://github.com/juspay/hyperswitch/pull/1166)) ([`94fe1af`](https://github.com/juspay/hyperswitch/commit/94fe1af1b0bce3b4ecaef8665909fc8f5cd4bbbb))

- - -

## 0.5.13 (2023-05-15)

### Features

- **config:** Add API route `set_config` ([#1144](https://github.com/juspay/hyperswitch/pull/1144)) ([`f31926b`](https://github.com/juspay/hyperswitch/commit/f31926b833557f18f93620d34765c90ac16fbeeb))
- **connector:**
  - Add payment, refund urls for dummy connector ([#1084](https://github.com/juspay/hyperswitch/pull/1084)) ([`fee0e9d`](https://github.com/juspay/hyperswitch/commit/fee0e9dadd2e20c5c75dcee50de0e53f4e5e6deb))
  - [ACI] Implement Trustly Bank Redirect ([#1130](https://github.com/juspay/hyperswitch/pull/1130)) ([`46b40ec`](https://github.com/juspay/hyperswitch/commit/46b40ecce540b61eced7156555c0fcdcec170405))
  - Add multiple dummy connectors and enable them ([#1147](https://github.com/juspay/hyperswitch/pull/1147)) ([`8a35f7c`](https://github.com/juspay/hyperswitch/commit/8a35f7c926f3cbd0d5cd3c2c9470575246985ca3))
  - [ACI] Implement Alipay and MB WAY Wallets ([#1140](https://github.com/juspay/hyperswitch/pull/1140)) ([`d7cfb4a`](https://github.com/juspay/hyperswitch/commit/d7cfb4a179083580a7e195fa07077af23a262ceb))
  - [Stripe] Implement Przelewy24 bank redirect ([#1111](https://github.com/juspay/hyperswitch/pull/1111)) ([`54ff02d`](https://github.com/juspay/hyperswitch/commit/54ff02d9ddb4cbe2f085f894c833b9800ce8d597))
- **error:**
  - Add feature-gated stacktrace to error received from API ([#1104](https://github.com/juspay/hyperswitch/pull/1104)) ([`bf2352b`](https://github.com/juspay/hyperswitch/commit/bf2352b14ae7d7343474424be0f0a4b0fee1b0f2))
  - Add `DateTimeParsingError` and `EmailParsingError` variants to `ParsingError` enum ([#1146](https://github.com/juspay/hyperswitch/pull/1146)) ([`7eed8e7`](https://github.com/juspay/hyperswitch/commit/7eed8e7f3e84a7ab4ce8bd4b7892a931211dbe3f))
- **payment_request:** Add field `amount` to `OrderDetails` and make `order_details` a `Vec` in `payments_create` request ([#964](https://github.com/juspay/hyperswitch/pull/964)) ([`60e8c73`](https://github.com/juspay/hyperswitch/commit/60e8c7317a2d1cc99f0179479891565f990df685))
- **router:**
  - Add payment, refund routes for dummy connector ([#1071](https://github.com/juspay/hyperswitch/pull/1071)) ([`822fc69`](https://github.com/juspay/hyperswitch/commit/822fc695a38560e6ea4ff13bc837d46214ee9249))
  - Add attach dispute evidence api ([#1070](https://github.com/juspay/hyperswitch/pull/1070)) ([`a5756aa`](https://github.com/juspay/hyperswitch/commit/a5756aaecf1b96ef4d04c57592b85f2a20da6639))

### Bug Fixes

- **connector:**
  - [Adyen] fix status mapping for Adyen authorize, capture, refund API ([#1149](https://github.com/juspay/hyperswitch/pull/1149)) ([`2932a5f`](https://github.com/juspay/hyperswitch/commit/2932a5f0ff5aa8dabd69fc683b5c688a20c405f9))
  - Fix Stripe status to attempt status map ([#1132](https://github.com/juspay/hyperswitch/pull/1132)) ([`8b85647`](https://github.com/juspay/hyperswitch/commit/8b85647a169d1d3ea59d2b472eabb99482f71eda))
- **mandate:** Allow card details to be provided in case of network transaction id ([#1138](https://github.com/juspay/hyperswitch/pull/1138)) ([`cc121d0`](https://github.com/juspay/hyperswitch/commit/cc121d0febcb397a989e512928d33a8cff2fbdee))

- - -

## 0.5.12 (2023-05-11)

### Features

- **Connector:** [ACI] Implement Przelewy24 Bank Redirect ([#1064](https://github.com/juspay/hyperswitch/pull/1064)) ([`cef8914`](https://github.com/juspay/hyperswitch/commit/cef8914372fa051f074e89fc76b76c6aee0d7bca))
- **connector:**
  - [Iatapay] Implement AccessTokenAuth, Authorize, PSync, Refund, RSync and testcases ([#1034](https://github.com/juspay/hyperswitch/pull/1034)) ([`a2527b5`](https://github.com/juspay/hyperswitch/commit/a2527b5b2af0a72422e1169f0827b6c55e21d673))
  - [bitpay] Add new crypto connector bitpay & testcases for all crypto connectors ([#919](https://github.com/juspay/hyperswitch/pull/919)) ([`f70f10a`](https://github.com/juspay/hyperswitch/commit/f70f10aac58cce805b150badf634271c0f98d478))
  - Add connector nmi with card, applepay and googlepay support ([#771](https://github.com/juspay/hyperswitch/pull/771)) ([`baf5fd9`](https://github.com/juspay/hyperswitch/commit/baf5fd91cf7fbb9f787e1ba137d1a3c597fe44ef))
  - [ACI] Implement Interac Online Bank Redirect ([#1108](https://github.com/juspay/hyperswitch/pull/1108)) ([`0177f1d`](https://github.com/juspay/hyperswitch/commit/0177f1d1b90bfa6bfb817bf282f3fb1f52eae7f6))
- **pm_list:** Add pm list support for bank_debits ([#1120](https://github.com/juspay/hyperswitch/pull/1120)) ([`dfc6be4`](https://github.com/juspay/hyperswitch/commit/dfc6be4e4f3333ae4639bf4b98c4ec834a66f460))

### Bug Fixes

- **connector:** Fix checkout error response type ([#1124](https://github.com/juspay/hyperswitch/pull/1124)) ([`5fd1614`](https://github.com/juspay/hyperswitch/commit/5fd16146dba52f65f7c5fe26f0a7526875e4e4e2))
- **connector_customer:** Create connector_customer on requirement basis ([#1097](https://github.com/juspay/hyperswitch/pull/1097)) ([`e833a1d`](https://github.com/juspay/hyperswitch/commit/e833a1ddeeae06cd58cb9d6fc760d8e3b0d82b6b))
- **google_pay:** Allow custom fields in `GpayTokenParameters` for google pay via stripe ([#1125](https://github.com/juspay/hyperswitch/pull/1125)) ([`f790099`](https://github.com/juspay/hyperswitch/commit/f790099368ed6ed73ecc729cb18b85e0c6b5f809))
- **mandate:** Only trigger mandate procedure on successful connector call ([#1122](https://github.com/juspay/hyperswitch/pull/1122)) ([`a904d2b`](https://github.com/juspay/hyperswitch/commit/a904d2b4d945c8ecaacae41bf44c6a2ce6ac632e))
- **payments:** Fix address_insert error propagation in get_address_for_payment_request function ([#1079](https://github.com/juspay/hyperswitch/pull/1079)) ([`da3b520`](https://github.com/juspay/hyperswitch/commit/da3b5201b4e30a6047bbf3069b2542482f8f9e51))
- **router:** Fix webhooks flow for checkout connector ([#1126](https://github.com/juspay/hyperswitch/pull/1126)) ([`7f3ceb4`](https://github.com/juspay/hyperswitch/commit/7f3ceb42fb95a117a39bc679ce2f7830bffbec54))

### Refactors

- **api_models:**
  - Remove unused mapping of attempt status to intent status ([#1127](https://github.com/juspay/hyperswitch/pull/1127)) ([`45ccc41`](https://github.com/juspay/hyperswitch/commit/45ccc410eacd425c6b68179ffa7b4258ab341e61))
  - Derive serialize on`PaymentsCaptureRequest` struct ([#1129](https://github.com/juspay/hyperswitch/pull/1129)) ([`e779ee7`](https://github.com/juspay/hyperswitch/commit/e779ee78a47e1b6d08c4df4afc3762c33db51eeb))
- **errors:** Add parsing error types for context info ([#911](https://github.com/juspay/hyperswitch/pull/911)) ([`0d46690`](https://github.com/juspay/hyperswitch/commit/0d466905024018e7ca5a7acc66ee98784337e7d3))

### Revert

- Refactor(merchant_account): add back `api_key` field for backward compatibility ([#761](https://github.com/juspay/hyperswitch/pull/761)) ([#1062](https://github.com/juspay/hyperswitch/pull/1062)) ([`f481abb`](https://github.com/juspay/hyperswitch/commit/f481abb8551f3ec5e495cf9916d9d8a5cecd62da))

- - -

## 0.5.11 (2023-05-10)

### Features

- **Connector:**
  - [Adyen]Implement ACH Direct Debits for Adyen ([#1033](https://github.com/juspay/hyperswitch/pull/1033)) ([`eee55bd`](https://github.com/juspay/hyperswitch/commit/eee55bdfbe67e5f4be7ed7e388f5ed93e70165ff))
  - [Stripe] Implemented Alipay Digital Wallet ([#1048](https://github.com/juspay/hyperswitch/pull/1048)) ([`7c7185b`](https://github.com/juspay/hyperswitch/commit/7c7185bc1a783efe81a994fef179a73313954d9d))
  - [Stripe] Implement Wechatpay Digital Wallet ([#1049](https://github.com/juspay/hyperswitch/pull/1049)) ([`93947ea`](https://github.com/juspay/hyperswitch/commit/93947eaf258ddb74315f4776b2faec87f42e6216))
- **cards:** Add credit card number validation ([#889](https://github.com/juspay/hyperswitch/pull/889)) ([`d6e71b9`](https://github.com/juspay/hyperswitch/commit/d6e71b959ddbdc99411fc7d669df61f373de4e32))
- **connector:**
  - Mandates for alternate payment methods via Adyen ([#1046](https://github.com/juspay/hyperswitch/pull/1046)) ([`4403634`](https://github.com/juspay/hyperswitch/commit/4403634dda41b1b7fbbe56ee6177722bcbe2e29b))
  - Add klarna, afterpay support in Nuvei ([#1081](https://github.com/juspay/hyperswitch/pull/1081)) ([`0bb0437`](https://github.com/juspay/hyperswitch/commit/0bb0437b7fca30b9a1d1567ab22afebeb7bce744))
  - Add dispute and refund webhooks for Airwallex ([#1021](https://github.com/juspay/hyperswitch/pull/1021)) ([`8c34114`](https://github.com/juspay/hyperswitch/commit/8c3411413847ac2dda3fef485d1e402a11376780))
  - Add bank redirect support for worldline ([#1060](https://github.com/juspay/hyperswitch/pull/1060)) ([`bc4ac52`](https://github.com/juspay/hyperswitch/commit/bc4ac529aa981150de6882d425bd274bc6272e30))
  - [Adyen] Implement SEPA Direct debits for Adyen ([#1055](https://github.com/juspay/hyperswitch/pull/1055)) ([`7f796a6`](https://github.com/juspay/hyperswitch/commit/7f796a6709e18cc92668e50a044408bad8aeee3d))
- **refunds:** Add connector field in refund response ([#1059](https://github.com/juspay/hyperswitch/pull/1059)) ([`3fe24b3`](https://github.com/juspay/hyperswitch/commit/3fe24b3255039d6a5dff59203ffcfd024ff0d60b))
- **router:**
  - Added retrieval flow for connector file uploads and added support for stripe connector ([#990](https://github.com/juspay/hyperswitch/pull/990)) ([`38aa9ea`](https://github.com/juspay/hyperswitch/commit/38aa9eab3f2453593e7b0c3fa63b37f7f2609514))
  - Add disputes block in payments retrieve response ([#1038](https://github.com/juspay/hyperswitch/pull/1038)) ([`1304d91`](https://github.com/juspay/hyperswitch/commit/1304d912e53cf223f8f15760e29b84faafe4f6ea))
- Allow payment cancels for more statuses ([#1027](https://github.com/juspay/hyperswitch/pull/1027)) ([`a2a6bab`](https://github.com/juspay/hyperswitch/commit/a2a6bab56cc70463d25232ce40ca4f115bee24e0))

### Bug Fixes

- **applepay:** Rename applepay_session_response to lowercase ([#1090](https://github.com/juspay/hyperswitch/pull/1090)) ([`736a236`](https://github.com/juspay/hyperswitch/commit/736a236651523b7f72ff95ad9223f4dda875301a))
- **router:** Fix recursion bug in straight through algorithm ([#1080](https://github.com/juspay/hyperswitch/pull/1080)) ([`aa610c4`](https://github.com/juspay/hyperswitch/commit/aa610c49f5a24e3e858515d9dfe0872d43251ee5))
- **tests:** Remove ui tests from ci pipeline ([#1082](https://github.com/juspay/hyperswitch/pull/1082)) ([`2ab7f83`](https://github.com/juspay/hyperswitch/commit/2ab7f83103d0907095e5b15a35f298ae60e6d180))
- Connector-customer-id missing bug fix ([#1085](https://github.com/juspay/hyperswitch/pull/1085)) ([`c5db5c3`](https://github.com/juspay/hyperswitch/commit/c5db5c37ec8f15e90d56aca59d14331fd8a2ea30))

### Refactors

- **router:** Add `id` field in `MerchantConnectorAccountNotFound` ([#1098](https://github.com/juspay/hyperswitch/pull/1098)) ([`5214e22`](https://github.com/juspay/hyperswitch/commit/5214e22f20c01e7dfb402ae619fdf2e7339d0fe7))

### Documentation

- **changelog:** Adding changelog for v0.5.10 ([#1078](https://github.com/juspay/hyperswitch/pull/1078)) ([`cb77b01`](https://github.com/juspay/hyperswitch/commit/cb77b012a2751f10395c3ff698aed4714a6b4223))

### Miscellaneous Tasks

- **CODEOWNERS:** Update CODEOWNERS ([#1076](https://github.com/juspay/hyperswitch/pull/1076)) ([`1456580`](https://github.com/juspay/hyperswitch/commit/1456580366c618300db4e0746db08a7466e04ea8))

- - -

## 0.5.10 (2023-05-08)

### Features

- **common_utils:**
  - Impl deref for email newtype ([#1073](https://github.com/juspay/hyperswitch/pull/1073)) ([`fa8683a`](https://github.com/juspay/hyperswitch/commit/fa8683a54b0056f4cc31d096765de373f8ae8a43))
  - Impl from for email newtype ([#1074](https://github.com/juspay/hyperswitch/pull/1074)) ([`7c6f0fd`](https://github.com/juspay/hyperswitch/commit/7c6f0fdec5c8f03863d26fc6dabf1fb3225e3d59))
- **connector:**
  - Add authorize, capture, void, psync, refund, rsync for Forte connector ([#955](https://github.com/juspay/hyperswitch/pull/955)) ([`f0464bc`](https://github.com/juspay/hyperswitch/commit/f0464bc4f584b52c4983df62a28befd60f67cca4))
  - Add dummy connector template code ([#970](https://github.com/juspay/hyperswitch/pull/970)) ([`e5cc0d9`](https://github.com/juspay/hyperswitch/commit/e5cc0d9d45d41c391720ceb3f6c18151ac5a00f2))
  - Add payment routes for dummy connector ([#980](https://github.com/juspay/hyperswitch/pull/980)) ([`4ece376`](https://github.com/juspay/hyperswitch/commit/4ece376b56549b53bd81c16fd9fdebbd0b9b1114))
  - [Bluesnap] add cards 3DS support ([#1057](https://github.com/juspay/hyperswitch/pull/1057)) ([`9c331e4`](https://github.com/juspay/hyperswitch/commit/9c331e411ba524ef41352c1c7c69635492fcec23))
  - Mandates for alternate payment methods via Stripe ([#1041](https://github.com/juspay/hyperswitch/pull/1041)) ([`64721b8`](https://github.com/juspay/hyperswitch/commit/64721b80ae0d276820404ff1208af91303cf1473))
- **errors:** Add reverse errorswitch trait for foreign errors ([#909](https://github.com/juspay/hyperswitch/pull/909)) ([`ab55d21`](https://github.com/juspay/hyperswitch/commit/ab55d21013a279568379b97821da98457a10754a))

### Bug Fixes

- **common_utils:** Manually implement diesel queryable for email newtype ([#1072](https://github.com/juspay/hyperswitch/pull/1072)) ([`3519649`](https://github.com/juspay/hyperswitch/commit/35196493c4509a6f9f1c202bf8b8a6aa7605346b))
- **connector:**
  - [worldline] fix worldline unit test ([#1054](https://github.com/juspay/hyperswitch/pull/1054)) ([`3131bc8`](https://github.com/juspay/hyperswitch/commit/3131bc84af008f05508aab9049f6ee492ca89460))
  - [ACI] Add amount currency conversion and update error codes ([#1065](https://github.com/juspay/hyperswitch/pull/1065)) ([`b760cba`](https://github.com/juspay/hyperswitch/commit/b760cba5460395487c63ea4363665b0d7e5a6118))
- **mandate:**
  - Make payment_method_data optional for mandate scenario ([#1032](https://github.com/juspay/hyperswitch/pull/1032)) ([`9cb3fa2`](https://github.com/juspay/hyperswitch/commit/9cb3fa216ce490d62f99525b23430809b4943dcb))
  - Fix payment_method_data becoming empty when mandate_id is not present ([#1077](https://github.com/juspay/hyperswitch/pull/1077)) ([`5c5c3ef`](https://github.com/juspay/hyperswitch/commit/5c5c3ef3831991ccfefd9b0561f5eac976ed2191))
- **redis:** Fix recreation on redis connection pool ([#1063](https://github.com/juspay/hyperswitch/pull/1063)) ([`982c27f`](https://github.com/juspay/hyperswitch/commit/982c27fce72074d2644c0a9f229b201b927c55da))
- Impl `Drop` for `RedisConnectionPool` ([#1051](https://github.com/juspay/hyperswitch/pull/1051)) ([`3d05e50`](https://github.com/juspay/hyperswitch/commit/3d05e50abcb92fe7e6c4472faafc03fb70920048))
- Throw PreconditionFailed error when routing_algorithm is not configured ([#1017](https://github.com/juspay/hyperswitch/pull/1017)) ([`8853702`](https://github.com/juspay/hyperswitch/commit/8853702f4b98c72655d6e36ed6acc13b7c261ad5))

### Refactors

- **compatibility:** Refactor stripe compatibility routes using `web::resource` ([#1022](https://github.com/juspay/hyperswitch/pull/1022)) ([`92ae2d9`](https://github.com/juspay/hyperswitch/commit/92ae2d92f18577d5cc88805340fa63c5e50dbc37))
- **router:**
  - Nest the straight through algorithm column in payment attempt ([#1040](https://github.com/juspay/hyperswitch/pull/1040)) ([`64fa21e`](https://github.com/juspay/hyperswitch/commit/64fa21eb4fb265e122f97aaae7445fabd571be23))
  - Add the `connector_label` field to `DuplicateMerchantConnectorAccount` error message ([#1044](https://github.com/juspay/hyperswitch/pull/1044)) ([`b3772f8`](https://github.com/juspay/hyperswitch/commit/b3772f8ef13a565730ec229b612c10ed68bb3c4b))
  - Include payment method type in connector choice for session flow ([#1036](https://github.com/juspay/hyperswitch/pull/1036)) ([`73b8988`](https://github.com/juspay/hyperswitch/commit/73b8988322e3d15f90b2c4ca776d135d23e97710))
- Use newtype pattern for email addresses ([#819](https://github.com/juspay/hyperswitch/pull/819)) ([`b8e2b1c`](https://github.com/juspay/hyperswitch/commit/b8e2b1c5f42dcd41a3d02e0d2422e1407b6a41de))

- - -

## 0.5.9 (2023-05-04)

### Features

- **api_models:** Derive `Serialize`, `Eq`, `PartialEq`, `strum::Display` on `RefundStatus` ([#989](https://github.com/juspay/hyperswitch/pull/989)) ([`22a5372`](https://github.com/juspay/hyperswitch/commit/22a5372481bbf854cffb8b683606cdf0644a5f54))
- **cards:** Validate card security code and expiration ([#874](https://github.com/juspay/hyperswitch/pull/874)) ([`0b7bc7b`](https://github.com/juspay/hyperswitch/commit/0b7bc7bcd23498485c831d1c78187c433b8bb3c7))
- **connector:**
  - [ACI] Add banking redirect support for EPS, Giropay, iDEAL, and Sofortueberweisung ([#890](https://github.com/juspay/hyperswitch/pull/890)) ([`c86f2c0`](https://github.com/juspay/hyperswitch/commit/c86f2c045e3cc614e5f68d84b5055a1b0e222f67))
  - Add dispute webhooks for Stripe ([#918](https://github.com/juspay/hyperswitch/pull/918)) ([`0df2244`](https://github.com/juspay/hyperswitch/commit/0df224479416533579dd6d96e7f0dd9c246b739c))
  - Add Cards(3ds & non3ds),bank_redirects ,wallets(Paypal,Applepay) and Mandates support to nexinets ([#898](https://github.com/juspay/hyperswitch/pull/898)) ([`eea05f5`](https://github.com/juspay/hyperswitch/commit/eea05f5c3196d68cf9cd306419ac55003cebf002))
- **pm_list:** Add available capture methods filter ([#999](https://github.com/juspay/hyperswitch/pull/999)) ([`36cc13d`](https://github.com/juspay/hyperswitch/commit/36cc13d44bb61b840195e1a24f1bebdb0115d13b))
- **router:** Added support for optional defend dispute api call and added evidence submission flow for checkout connector ([#979](https://github.com/juspay/hyperswitch/pull/979)) ([`4728d94`](https://github.com/juspay/hyperswitch/commit/4728d946e24c2c548e7cdc23c34238ff028f1076))
- PG Agnostic mandate using network_txns_id (Adyen, Authorizedotnet, Stripe) ([#855](https://github.com/juspay/hyperswitch/pull/855)) ([`ed99655`](https://github.com/juspay/hyperswitch/commit/ed99655ebc11d53f4b2ffcb8c0eb9ef6b56f32c4))
- Expire client secret after a merchant configurable intent fufliment time ([#956](https://github.com/juspay/hyperswitch/pull/956)) ([`03a9643`](https://github.com/juspay/hyperswitch/commit/03a96432a9d9874d2232d75206f7bc605f1170f3))

### Bug Fixes

- **refund_list:** Updated refund list response status code when no refunds found. ([#974](https://github.com/juspay/hyperswitch/pull/974)) ([`4e0489c`](https://github.com/juspay/hyperswitch/commit/4e0489cf1cb7c17e55cffabeb0067c380ba41ff4))
- **refund_sync:** Add validation for missing `connector_refund_id` ([#1013](https://github.com/juspay/hyperswitch/pull/1013)) ([`4397c8e`](https://github.com/juspay/hyperswitch/commit/4397c8e19977974510f7c24daa8c3ef7f2ab907b))
- **storage_models:** Fix incorrect field order in `MerchantConnectorAccount` ([#976](https://github.com/juspay/hyperswitch/pull/976)) ([`c9e8a9b`](https://github.com/juspay/hyperswitch/commit/c9e8a9b4b721612ff2c771f4849fbad0c18bb7f2))
- Fix internal server errors on merchant connector account creation ([#1026](https://github.com/juspay/hyperswitch/pull/1026)) ([`c31b4b4`](https://github.com/juspay/hyperswitch/commit/c31b4b41c22c9c622d75f0f8421ec67a416d5b70))
- Remove old data while deserialization error from cache ([#1010](https://github.com/juspay/hyperswitch/pull/1010)) ([`23b5647`](https://github.com/juspay/hyperswitch/commit/23b5647290a7baa12107abd88359507aa3c31444))
- Passing connector_name instead of ConnectorCallType ([#1050](https://github.com/juspay/hyperswitch/pull/1050)) ([`c888635`](https://github.com/juspay/hyperswitch/commit/c888635166be08e826f8a21f5c0c3262cc0918f9))

### Refactors

- **config:** Add independent toggles for enabling traces and metrics ([#1020](https://github.com/juspay/hyperswitch/pull/1020)) ([`af71828`](https://github.com/juspay/hyperswitch/commit/af71828e351918fe6a97b52969db4abd331f6e5b))
- **stripe:** Return all the missing fields in a request ([#935](https://github.com/juspay/hyperswitch/pull/935)) ([`e9fc34f`](https://github.com/juspay/hyperswitch/commit/e9fc34ff626c13ec117f4ec9b091a69892bddf4f))
- Use `CountryAlpha2` instead of `CountryCode` for country codes ([#904](https://github.com/juspay/hyperswitch/pull/904)) ([`2cff019`](https://github.com/juspay/hyperswitch/commit/2cff019a1be669e5b1cd44d5513463671f386f4c))

### Documentation

- **README:** Remove redundant "more" in FAQs ([#1031](https://github.com/juspay/hyperswitch/pull/1031)) ([`9cbda83`](https://github.com/juspay/hyperswitch/commit/9cbda838171331598018a640551495014bc364a2))

### Miscellaneous Tasks

- Add `git-cliff` configs for generating changelogs and release notes ([#1047](https://github.com/juspay/hyperswitch/pull/1047)) ([`68360d4`](https://github.com/juspay/hyperswitch/commit/68360d4d6a31d8d7361c83021ca3049780d6d0a3))

### Build System / Dependencies

- **deps:** Make AWS dependencies optional ([#1030](https://github.com/juspay/hyperswitch/pull/1030)) ([`a4f6f3f`](https://github.com/juspay/hyperswitch/commit/a4f6f3fdaa23f7bd849eb44971de8311f9363ac3))

- - -

## 0.5.8 (2023-04-25)

### Chores

*  fix error message for deserialization ([#885](https://github.com/juspay/orca/pull/885)) ([e4d0dd0a](https://github.com/juspay/orca/commit/e4d0dd0a3885151a8e28a0246e67523f90f53076))

### Continuous Integration

* **connector-sanity-tests:**  run tests on being queued for merge ([#960](https://github.com/juspay/orca/pull/960)) ([067dc709](https://github.com/juspay/orca/commit/067dc709360394b062c217ca3a27e011bfbac215))
* **manual-release:**  fix `EXTRA_FEATURES` not being passed correctly ([#912](https://github.com/juspay/orca/pull/912)) ([9c9c52f9](https://github.com/juspay/orca/commit/9c9c52f9af74ebc7e835a5750dd05967b39a0ade))

### Documentation Changes

* **dashboard:**  add button that links to dashboard ([#934](https://github.com/juspay/orca/pull/934)) ([96f9e806](https://github.com/juspay/orca/commit/96f9e8068bfae0dd8479f69d4add675f1aaad991))

### New Features

* **connector:**
  *  add 3ds for Bambora and Support Html 3ds response ([#817](https://github.com/juspay/orca/pull/817)) ([20bea23b](https://github.com/juspay/orca/commit/20bea23b75c30b27f5beda78ac2ffa8302c6e6a8))
  *  [Nuvei] add support for bank redirect Eps, Sofort, Giropay, Ideal ([#870](https://github.com/juspay/orca/pull/870)) ([c1a25b30](https://github.com/juspay/orca/commit/c1a25b30bd88ab4ad4f40866a16ba5651d711ee3))
  *  [Checkout] add GooglePay, ApplePay and Webhooks support  ([#875](https://github.com/juspay/orca/pull/875)) ([3fce1407](https://github.com/juspay/orca/commit/3fce1407039c060712465cf4a696f8ed23f3bffb))
* **router:**
  *  added dispute accept api, file module apis and dispute evidence submission api  ([#900](https://github.com/juspay/orca/pull/900)) ([bdf1e514](https://github.com/juspay/orca/commit/bdf1e5147e710876a62c7377471144175e6c823d))
  *  add new payment methods for Bank redirects, BNPL and wallet ([#864](https://github.com/juspay/orca/pull/864)) ([304081cb](https://github.com/juspay/orca/commit/304081cbadf86bbd5a20d69b96a79d6cd647024c))
* **compatibility:**  add refund retrieve endpoint which accepts gateway creds ([#958](https://github.com/juspay/orca/pull/958)) ([bcbf4c88](https://github.com/juspay/orca/commit/bcbf4c882c248d08d3d0733299c7220597d669e3))
* **Core:**  gracefully shutdown router/scheduler if Redis is unavailable ([#891](https://github.com/juspay/orca/pull/891)) ([13185999](https://github.com/juspay/orca/commit/13185999d5c03dfa9c1f9d72bff6b798c4b80be5))
* **core:**  [Stripe] add bank debits payment method to stripe ([#906](https://github.com/juspay/orca/pull/906)) ([f624eb52](https://github.com/juspay/orca/commit/f624eb52d61561c365cce21e58b08281d096d904))
*  support gpay and applepay session response for all connectors ([#839](https://github.com/juspay/orca/pull/839)) ([d23e14c5](https://github.com/juspay/orca/commit/d23e14c57a1defe46416130bda4845973b62a54d))
*  add relevant ids for payment calls & make json logs  ([#908](https://github.com/juspay/orca/pull/908)) ([93b69e74](https://github.com/juspay/orca/commit/93b69e74b40592b241c6ade1b51e2dd49b25a45d))
*  [Bluesnap] add GooglePay, ApplePay support (#985) (897250e)
*  [Zen] add Cards 3DS, Non-3DS, GooglePay, ApplePay and Webhooks support (#962) (71c39b)


### Bug Fixes

*  different parent payment method token for different payment me… ([#982](https://github.com/juspay/orca/pull/982)) ([2f378345](https://github.com/juspay/orca/commit/2f378345aab58113620c11a18455f118e136a0c1))
* **config:**  fix Tempo config for Tempo 2.0 ([#959](https://github.com/juspay/orca/pull/959)) ([811cd523](https://github.com/juspay/orca/commit/811cd523c20343761ee5b420d0fcab59be39c56d))
* **stripe:**  add setup intent sync for stripe ([#953](https://github.com/juspay/orca/pull/953)) ([ab7fc23a](https://github.com/juspay/orca/commit/ab7fc23a7b7a2453ac41466f428d9c0df504968b))
* **connector:**
  *  fix adyen unit test ([#957](https://github.com/juspay/orca/pull/957)) ([85c76290](https://github.com/juspay/orca/commit/85c7629061ebbe5c9e0393f138af9b8876c3643d))
  *  [coinbase] update cancel status on user cancelling the payment ([#922](https://github.com/juspay/orca/pull/922)) ([22cee8cd](https://github.com/juspay/orca/commit/22cee8cdd9567545cd61587a8158aca754d77e0a))
  *  fix adyen unit test ([#931](https://github.com/juspay/orca/pull/931)) ([afeb8319](https://github.com/juspay/orca/commit/afeb83194f0772e7550c5d4a6ed4ba16216d2a28))
* **connector-template:**  Address unused import and mismatched types in connector-template ([#910](https://github.com/juspay/orca/pull/910)) ([891683e0](https://github.com/juspay/orca/commit/891683e060d1fdda32405cfd06d737b2416acdcc))

### Other Changes

* **try_local_system:**  replace Postman collection links with development collection ([#937](https://github.com/juspay/orca/pull/937)) ([ccc0c3f9](https://github.com/juspay/orca/commit/ccc0c3f96021b25ce5de700cf584d688096a9bca))
* **pr-template:**  add API contract changes and update contributing docs with recent labels ([#936](https://github.com/juspay/orca/pull/936)) ([3e2a7eae](https://github.com/juspay/orca/commit/3e2a7eaed2e830b419964e486757c022a0ebca63))
* **errors:**  make StorageErrorExt generic on errors ([#928](https://github.com/juspay/orca/pull/928)) ([e161d92c](https://github.com/juspay/orca/commit/e161d92c58c85127c73fc150f88d1f58b2275da5))

### Refactors

* **db:**  remove `connector_transaction_id` from PaymentAttemptNew ([#949](https://github.com/juspay/orca/pull/949)) ([57327b82](https://github.com/juspay/orca/commit/57327b829776c58fa6c3569c5546c4706d2c66af))
* **api_keys:**  use `merchant_id` and `key_id` to query the table ([#939](https://github.com/juspay/orca/pull/939)) ([40898c0a](https://github.com/juspay/orca/commit/40898c0ac9199258fbc6e8e12950d4fa54ec3339))

- - -

## 0.5.7 (2023-04-18)

### New Features

* **connector:**
  *  [Shift4] add support for card 3DS payment (#828) (29999fe5)
  *  [Nuvei] add support for card mandates (#818) (298a0a49)
* **bank_redirects:**  modify api contract for sofort (#880) (fc2e4514)
  *  add template code for connector forte (#854) (7a581a6)
  *  add template code for connector nexinets (#852) (dee5f61)

### Bug Fixes

* **connector:**  [coinbase] make metadata as option parameter (#887) (f5728955)
*  Update events table after notifying merchant (#871) (013026)
* **stripe:**  remove cancel reason validation for stripe (#876) (fa44c1f6)

### Enhancement

* **payments:**  make TokenizationAction clonable (#895)

### Integration

*  Frm integration with hyperswitch (#857)

### Refactors

*  use lowercase names for run environment and config files (#801) (ffaa8da0)
*  derive `Serialize` and `Deserialize` to `Country` enum (#882) (456c16fb)
* **storage_models, errors:**  impl StorageErrorExt for error_stack::Result<T, errors::StorageError> (#886) (b4020294)
* **router:**  KMS decrypt secrets when kms feature is enabled  (#868) (8905e663)

- - -

## 0.5.6 2023-04-14

### Build System / Dependencies

* **deps:**  bump `fred` from `5.2.0` to `6.0.0` (#869) (01bc162d)

### Continuous Integration

* **manual_release:**  add `multiple_mca` feature in ci (#872) (aebb4dca)

### New Features

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

### Bug Fixes

* **merchant_account:**  change `primary_business_details` to vec in update (#877) (396d24fe)
*  redis deserialization issue in tokenization call (#878) (5e9d7d6b)
*  duplication check fix in basilisk-hs (#881) (b12762e7)

### Refactors

* **Tokenization:**  remove ConnectorCallType from tokenization call (#862) (0d047e08)
* **router_env:**  improve logging setup (#847) (1b94d25f)
* **refund_type:** Feat/add copy derive (#849) (ccf03273)

- - -

## 0.5.5 (2023-04-10)

### New Features

* **api_models:**  derive `strum::Display` for `RefundStatus` (#846) (4524d4f5)
*  allow (de)serializing countries to/from alpha-2, alpha-3 and numeric country codes (#836) (899767cf)
* **connector:**  add authorize, capture, void, psync, refund, rsync for PayPal connector (#747) (36049c13)

### Bug Fixes

*  Add locker sign keyid in env (#844) (70dff140)

### Other Changes

* **common_utils:**  put the async ext trait behind a feature (#835) (de29eb68)
*  update ci workflows for common_enums crate (#843) (45111337)

### Refactors

* **scheduler:**  remove scheduler options & adding graceful shutdown to producer (#840) (11df8436)
* **router:**  refactor amount in PaymentsCaptureData from Option<i64> to i64 (#821) (b8bcba4e)

- - -

## 0.5.4 (2023-04-04)

### New Features

* **request:**  add `RequestBuilder` method to attach default request headers (#826) (6f61f830)
* **middleware:**  add middleware to attach default response headers (#824) (6d7b11a0)
* **core:**  added multiple payment_attempt support for payment_intent (#439) (35d3e277)
* **router:**  added incoming dispute webhooks flow (#769) (a733eafb)

### Bug Fixes

* **cards_info:**  add extra columns to cards_info struct (#813) (442bed0f)
* **connector:**  [Mollie] remove unsupported implementation of Void flow from mollie connector (#808) (eee8304b)

### Other Changes

* **common_utils:**  put the signals module behind a feature flag (#814) (fb4ec431)
* **core:**  replace string with enum for country (#735) (e18bfb2a)
* **api_models:**  put the errors module behind a feature flag (#815) (f14f87a1)
* **storage_models:**  delete client secret when status is succeeded, failed, cancelled (#724) (a05059b7)
### Refactors

* **drainer, router:**  KMS decrypt database password when `kms` feature is enabled (#733) (9d6e4ee3)

- - -

## 0.5.3 (2023-03-29)

### Documentation Changes

* **rfc:**  add rfc template & first RFC (#806) (01a5e0a0)

### New Features

*  cards info api (#749) (b15b8f7b)
* **connector:**  [Nuvei] add webhook support (#795) (20b4372b)

### Bug Fixes

* **compatibility:**  add last_payment_error in stripe payment response (#803) (97b95f0e)

### Refactors

* **api_models:**  enhance accepted countries/currencies types (#807) (f9ef3135)
* **services:**   make AppState impl generic using AppStateInfo (#805) (642c3f3a)

- - -

## 0.5.2 (2023-03-24)

### Chores

*  prepare for building production Docker images (#794) (6ddc30eb)

### Bug Fixes

* **connector:**  [Airwallex] Change Session Token to Init Payment (#798) (a3c00339)

### Other Changes

* **router:**  change MAX_ID_LENGTH to 64 (#792) (346bd954)

### Refactors

*  extract kms module to `external_services` crate (#793) (029e3894)

- - -

## 0.5.1 (2023-03-21)

### Documentation Changes

* **try_local_system:**
  *  add Ubuntu on WSL2 setup instructions (#767) (1d2166cf)
  *  add API key creation step (#765) (4b268068)

### New Features

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

### Bug Fixes

*  process delete response from basilisk-v3 as plaintext instead of JWE (#791) (699ca4f)
* **storage:**  add serialization for primitivedatetime for diesel structs (#764) (f27732a6)

### Refactors

*  get connection pool based on olap/oltp features (#743) (a392fb16)

- - -

## 0.5.0 (2023-03-21)

### Build System / Dependencies

* **deps:**  update deps (#734) (16bc886c)

### Chores

* **merchant_account:**  remove `api_key` field (#713) (230fcdd4)
* **config:**  move connector base URLs under the `[connectors]` table (#723) (df8c8b5a)
*  address Rust 1.68 clippy lints (#728) (1ffabb40)

### Continuous Integration

* **release:**  specify `fetch-depth` for code checkout and use official Docker GitHub actions (#722) (c451368f)

### Documentation Changes

*  Update naming conventions and added examples (#709) (98415193)
* **openapi:**  document path parameters for API keys endpoints (#702) (9062dc80)

### New Features

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

### Bug Fixes

* **docker-compose:**  remove port for hyperswitch-server-init in docker-compose.yml (#763) (20b93276)
*  fixing docker compose setup & adding redisinsight (#748) (5c9bec9f)
* **kms:**  log KMS SDK errors using the `Debug` impl (#720) (468aa87f)
* **errors:**
  *  Replace PaymentMethod with PaymentModethodData in test.rs (#716) (763ee094)
  *  use `Debug` impl instead of `Display` for error types wrapping `error_stack::Report` (#714) (45484752)

### Other Changes

*  card_fingerprint not sent by basilisk_hs (#754) (5ae2f63f)

### Refactors

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

### Tests

* **masking:**  add suitable feature gates for basic tests (#745) (4859b6e4)

- - -

## 0.3.0 (2023-03-05)

### Chores
* **connectors:**  log connector request and response at debug level (#624) (6a487b19)

### Continuous Integration

* **workflow:** adding build only sandbox feature to reduce build time (#664) (d1c9305e)
* **workflow:** run cargo hack only for code changes (#663) (f931c427)

### Documentation Changes

* **openapi:**  document security schemes (#676) (c5fda7ac)

### New Features

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

### Bug Fixes

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

### Other Changes

* **stripe:**  send statement descriptor to stripe (#707) (641c4d6d)
*  use connector error handler for 500 error messages. (#696) (9fe20932)
*  populate failed status and add bank_redirect (#674)
* **refunds:**  skip validate refunds for card (#672) (5cdbef04)
* **router/webhooks:**  expose additional incoming request details to webhooks flow (#637) (1b3b7f5b)
* **braintree:**  create basic auth for braintree (#602) (c47619b5)

### Refactors

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

- - -

## 0.3.0 (2023-02-25)

### Build System / Dependencies

* **docker-compose:**  increase docker health check interval for hyperswitch-server (#534)

### Chores

* **release:**  port release bug fixes to main branch (#612) (a8d6ce83)

### Continuous Integration

*  run CI checks on merge queue events (#530) (c7b9e9c1)

### Documentation Changes

* **add_connector:**  fix typo (#584) (a4f3abf3)

### New Features

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

### Bug Fixes

* **connector:**  update Bluesnap in routable connectors  (#654) (64cb2ffc)
*  allow errors with status code 200 to pass (#601) (8a8767e9)
*  don't call connector if connector transaction id doesn't exist (#525) (326d6beb)
*  throw 500 error when redis goes down (#531) (aafb115a)
* **router:**
  *  allow setup future usage to be updated in payment update and confirm requests (#610) (#638) (6c128f82)
  *  feature gate openssl deps for basilisk feature (#536) (e4956820)
* **checkout:**  Error Response when wrong api key is passed (#596) (55b6d88a)
* **core:**  use guard for access token result (#522) (903b4521)

### Other Changes

* **router:**
  *  webhooks enhancement (#637) (#641) (3bc9feb0)
  *  api keys path params (#609) (effa7a00)

### Refactors

* **router:**
  *  update payments api contract to accept a list of connectors (#643) (8f1f626c)
  *  api-key routes refactoring (#600) (e6408276)
  *  appstate as trait in authentication (#588) (eaf98e66)
* **compatibility:**  add additional fields to stripe payment and refund response types (#618) (2ea09e34)
*  Throw 500 error on database connection error instead of panic (#527) (f1e3bf48)
*  send full payment object for payment sync (#526) (6c2a1fea)
* **middleware:**  change visibility to `pub` (#587) (4884a24d)

- - -

## 0.2.1 (2023-02-17)

### Fixes
- fix payment_status not updated when adding payment method ([#446])
- Decide connector only when the payment method is confirm ([10ea4919ba07d3198a6bbe3f3d4d817a23605924](https://github.com/juspay/hyperswitch/commit/10ea4919ba07d3198a6bbe3f3d4d817a23605924))
- Fix panics caused with empty diesel updates ([448595498114cd15158b4a78fc32d8e6dc1b67ee](https://github.com/juspay/hyperswitch/commit/448595498114cd15158b4a78fc32d8e6dc1b67ee))

- - -

## 0.2.0 (2023-01-23) - Initial Release

### Supported Connectors

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


### Supported Payment Methods

- Cards No 3DS
- Cards 3DS*
- [Apple Pay](https://www.apple.com/apple-pay/)*
- [Google Pay](https://pay.google.com)*
- [Klarna](https://www.klarna.com/)*
- [PayPal](https://www.paypal.com/)*

### Supported Payment Functionalities

- Payments (Authorize/Sync/Capture/Cancel)
- Refunds (Execute/Sync)
- Saved Cards
- Mandates (No 3DS)*
- Customers
- Merchants
- ConnectorAccounts

\* May not be supported on all connectors
