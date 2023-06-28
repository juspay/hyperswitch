# Changelog

All notable changes to HyperSwitch will be documented here.

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

## 0.5.2 (2023-03-24)

### Chores

*  prepare for building production Docker images (#794) (6ddc30eb)

### Bug Fixes

* **connector:**  [Airwallex] Change Session Token to Init Payment (#798) (a3c00339)

### Other Changes

* **router:**  change MAX_ID_LENGTH to 64 (#792) (346bd954)

### Refactors

*  extract kms module to `external_services` crate (#793) (029e3894)

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

## 0.2.1 (2023-02-17)

### Fixes
- fix payment_status not updated when adding payment method ([#446])
- Decide connector only when the payment method is confirm ([10ea4919ba07d3198a6bbe3f3d4d817a23605924](https://github.com/juspay/hyperswitch/commit/10ea4919ba07d3198a6bbe3f3d4d817a23605924))
- Fix panics caused with empty diesel updates ([448595498114cd15158b4a78fc32d8e6dc1b67ee](https://github.com/juspay/hyperswitch/commit/448595498114cd15158b4a78fc32d8e6dc1b67ee))


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
