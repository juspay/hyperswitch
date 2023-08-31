# Changelog

All notable changes to HyperSwitch will be documented here.

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
