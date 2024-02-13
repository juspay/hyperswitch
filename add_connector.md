# Guide to Integrate a Connector

## Introduction

This is a guide to contributing new connector to Router. This guide includes instructions on checking out the source code, integrating and testing the new connector, and finally contributing the new connector back to the project.

## Prerequisites

- Understanding of the Connector APIs which you wish to integrate with Router
- Setup of Router repository and running it on local
- Access to API credentials for testing the Connector API (you can quickly sign up for sandbox/uat credentials by visiting the website of the connector you wish to integrate)
-  Ensure that you have the nightly toolchain installed because the connector template script includes code formatting.

 Install it using `rustup`:

  ```bash
    rustup toolchain install nightly
   ```


In Router, there are Connectors and Payment Methods, examples of both are shown below from which the difference is apparent.

### What is a Connector ?

A connector is an integration to fulfill payments. Related use cases could be any of the below

- Payment processor (Stripe, Adyen, ChasePaymentTech etc.,)
- Fraud and Risk management platform (like Signifyd, Riskified etc.,)
- Payment network (Visa, Master)
- Payment authentication services (Cardinal etc.,)
Currently, the router is compatible with 'Payment Processors' and 'Fraud and Risk Management' platforms. Support for additional categories will be expanded in the near future.

### What is a Payment Method ?

Every Payment Processor has the capability to accommodate various payment methods. Refer to the [Hyperswitch Payment matrix](https://hyperswitch.io/pm-list) to discover the supported processors and payment methods.

The above mentioned payment methods are already included in Router. Hence, adding a new connector which offers payment_methods available in Router is easy and requires almost no breaking changes.
Adding a new payment method might require some changes in core business logic of Router, which we are actively working upon.

## How to Integrate a Connector

Most of the code to be written is just another API integration. You have to write request and response types for API of the connector you wish to integrate and implement required traits.

For this tutorial we will be integrating card payment through the [Checkout.com connector](https://www.checkout.com/).
Go through the [Checkout.com API reference](https://api-reference.checkout.com/). It would also be helpful to try out the API's, using tools like on postman, or any other API testing tool before starting the integration.

Below is a step-by-step tutorial for integrating a new connector.

### **Generate the template**

```bash
sh scripts/add_connector.sh <connector-name> <connector-base-url>
```

For this tutorial `<connector-name>` would be `checkout`.

The folder structure will be modified as below

```
crates/router/src/connector
├── checkout
│   └── transformers.rs
└── checkout.rs
crates/router/tests/connectors
└── checkout.rs
```

`crates/router/src/connector/checkout/transformers.rs` will contain connectors API Request and Response types, and conversion between the router and connector API types.
`crates/router/src/connector/checkout.rs` will contain the trait implementations for the connector.
`crates/router/tests/connectors/checkout.rs` will contain the basic tests for the payments flows.

There is boiler plate code with `todo!()` in the above mentioned files. Go through the rest of the guide and fill in code wherever necessary.

### **Implementing Request and Response types**

Adding new Connector is all about implementing the data transformation from Router's core to Connector's API request format.
The Connector module is implemented as a stateless module, so that you will not have to worry about persistence of data. Router core will automatically take care of data persistence.

Lets add code in `transformers.rs` file.
A little planning and designing is required for implementing the Requests and Responses of the connector, as it depends on the API spec of the connector.

For example, in case of checkout, the [request](https://api-reference.checkout.com/#tag/Payments) has a required parameter `currency` and few other required parameters in `source`. But the fields in “source” vary depending on the `source type`. An enum is needed to accommodate this in the Request type. You may need to add the serde tags to convert enum into json or url encoded based on your requirements. Here `serde(untagged)` is added to make the whole structure into the proper json acceptable to the connector.

Now let's implement Request type for checkout

```rust
#[derive(Debug, Serialize)]
pub struct CardSource {
    #[serde(rename = "type")]
    pub source_type: CheckoutSourceTypes,
    pub number: cards::CardNumber,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub cvv: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaymentSource {
    Card(CardSource),
    Wallets(WalletSource),
    ApplePayPredecrypt(Box<ApplePayPredecrypt>),
}

#[derive(Debug, Serialize)]
pub struct PaymentsRequest {
    pub source: PaymentSource,
    pub amount: i64,
    pub currency: String,
    pub processing_channel_id: Secret<String>,
    #[serde(rename = "3ds")]
    pub three_ds: CheckoutThreeDS,
    #[serde(flatten)]
    pub return_url: ReturnUrl,
    pub capture: bool,
    pub reference: String,
}
```

Since Router is connector agnostic, only minimal data is sent to connector and optional fields may be ignored.

Here processing_channel_id, is specific to checkout and implementations of such functions should be inside the checkout directory.
Let's define `PaymentSource`

`PaymentSource` is an enum type. Request types will need to derive `Serialize` and response types will need to derive `Deserialize`. For request types `From<RouterData>` needs to be implemented.

For request types that involve an amount, the implementation of `TryFrom<&ConnectorRouterData<&T>>` is required:

```rust
impl TryFrom<&CheckoutRouterData<&T>> for PaymentsRequest 
```
else 
```rust
impl TryFrom<T> for PaymentsRequest 
```

where `T` is a generic type which can be `types::PaymentsAuthorizeRouterData`, `types::PaymentsCaptureRouterData`, etc.

In this impl block we build the request type from RouterData which will almost always contain all the required information you need for payment processing.
`RouterData` contains all the information required for processing the payment.

An example implementation for checkout.com is given below.

```rust
impl<'a> From<&types::RouterData<'a>> for CheckoutPaymentsRequest {
    fn from(item: &types::RouterData) -> Self {

        let ccard = match item.payment_method_data {
            Some(api::PaymentMethod::Card(ref ccard)) => Some(ccard),
            Some(api::PaymentMethod::BankTransfer) | None => None,
        };

        let source_var = Source::Card(CardSource {
            source_type: Some("card".to_owned()),
            number: ccard.map(|x| x.card_number.clone()),
            expiry_month: ccard.map(|x| x.card_exp_month.clone()),
            expiry_year: ccard.map(|x| x.card_exp_year.clone()),
        });

        CheckoutPaymentsRequest {
            source: source_var,
            amount: item.amount,
            currency: item.currency.to_string(),
            processing_channel_id: generate_processing_channel_id(),
        }

    }
}
```

Request side is now complete.
Similar changes are now needed to handle response side.

While implementing the Response Type, the important Enum to be defined for every connector is `PaymentStatus`.

It stores the different status types that the connector can give in its response that is listed in its API spec. Below is the definition for checkout

```rust
#[derive(Default, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum CheckoutPaymentStatus {
    Authorized,
    #[default]
    Pending,
    #[serde(rename = "Card Verified")]
    CardVerified,
    Declined,
    Captured,
}
```

The important part is mapping it to the Router status codes.

```rust
impl ForeignFrom<(CheckoutPaymentStatus, Option<Balances>)> for enums::AttemptStatus {
    fn foreign_from(item: (CheckoutPaymentStatus, Option<Balances>)) -> Self {
        let (status, balances) = item;

        match status {
            CheckoutPaymentStatus::Authorized => {
                if let Some(Balances {
                    available_to_capture: 0,
                }) = balances
                {
                    Self::Charged
                } else {
                    Self::Authorized
                }
            }
            CheckoutPaymentStatus::Captured => Self::Charged,
            CheckoutPaymentStatus::Declined => Self::Failure,
            CheckoutPaymentStatus::Pending => Self::AuthenticationPending,
            CheckoutPaymentStatus::CardVerified => Self::Pending,
        }
    }
}
```
If you're converting ConnectorPaymentStatus to AttemptStatus without any additional conditions, you can employ the `impl From<ConnectorPaymentStatus> for enums::AttemptStatus`.

Note: A payment intent can have multiple payment attempts. `enums::AttemptStatus` represents the status of a payment attempt.

Some of the attempt status are given below

- **Charged :** The payment attempt has succeeded.
- **Pending :** Payment is in processing state.
- **Failure :** The payment attempt has failed.
- **Authorized :** Payment is authorized. Authorized payment can be voided, captured and partial captured.
- **AuthenticationPending :** Customer action is required.
- **Voided :** The payment was voided and never captured; the funds were returned to the customer.

It is highly recommended that the default status is Pending. Only explicit failure and explicit success from the connector shall be marked as success or failure respectively.

```rust
// Default should be Pending
impl Default for CheckoutPaymentStatus {
    fn default() -> Self {
        CheckoutPaymentStatus::Pending
    }
}
```

Below is rest of the response type implementation for checkout

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct PaymentsResponse {
    id: String,
    amount: Option<i32>,
    action_id: Option<String>,
    status: CheckoutPaymentStatus,
    #[serde(rename = "_links")]
    links: Links,
    balances: Option<Balances>,
    reference: Option<String>,
    response_code: Option<String>,
    response_summary: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ActionResponse {
    #[serde(rename = "id")]
    pub action_id: String,
    pub amount: i64,
    #[serde(rename = "type")]
    pub action_type: ActionType,
    pub approved: Option<bool>,
    pub reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PaymentsResponseEnum {
    ActionResponse(Vec<ActionResponse>),
    PaymentResponse(Box<PaymentsResponse>),
}

impl TryFrom<types::PaymentsResponseRouterData<PaymentsResponse>>
    for types::PaymentsAuthorizeRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::PaymentsResponseRouterData<PaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = item.response.links.redirect.map(|href| {
            services::RedirectForm::from((href.redirection_url, services::Method::Get))
        });
        let status = enums::AttemptStatus::foreign_from((
            item.response.status,
            item.data.request.capture_method,
        ));
        let error_response = if status == enums::AttemptStatus::Failure {
            Some(types::ErrorResponse {
                status_code: item.http_code,
                code: item
                    .response
                    .response_code
                    .unwrap_or_else(|| consts::NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .response_summary
                    .clone()
                    .unwrap_or_else(|| consts::NO_ERROR_MESSAGE.to_string()),
                reason: item.response.response_summary,
                attempt_status: None,
                connector_transaction_id: None,
            })
        } else {
            None
        };
        let payments_response_data = types::PaymentsResponseData::TransactionResponse {
            resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
            redirection_data,
            mandate_reference: None,
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(
                item.response.reference.unwrap_or(item.response.id),
            ),
        };
        Ok(Self {
            status,
            response: error_response.map_or_else(|| Ok(payments_response_data), Err),
            ..item.data
        })
    }
}
```

Using an enum for a response struct in Rust is not recommended due to potential deserialization issues where the deserializer attempts to deserialize into all the enum variants. A preferable alternative is to employ a separate enum for the possible response variants and include it as a field within the response struct.

Some recommended fields that needs to be set on connector request and response

- **connector_request_reference_id :** Most of the connectors anticipate merchants to include their own reference ID in payment requests. For instance, the merchant's reference ID in the checkout `PaymentRequest` is specified as `reference`.

```rust
  reference: item.router_data.connector_request_reference_id.clone(),
```
- **connector_response_reference_id :** Merchants might face ambiguity when deciding which ID to use in the connector dashboard for payment identification. It is essential to populate the connector_response_reference_id with the appropriate reference ID, allowing merchants to recognize the transaction. This field can be linked to either `merchant_reference` or `connector_transaction_id`, depending on the field that the connector dashboard search functionality supports.

```rust
  connector_response_reference_id: item.response.reference.or(Some(item.response.id))
```

- **resource_id :** The connector assigns an identifier to a payment attempt, referred to as `connector_transaction_id`. This identifier is represented as an enum variant for the `resource_id`.  If the connector does not provide a `connector_transaction_id`, the resource_id is set to `NoResponseId`.   

```rust
  resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
```
- **redirection_data :** For the implementation of a redirection flow (3D Secure, bank redirects, etc.), assign the redirection link to the `redirection_data`.

```rust 
  let redirection_data = item.response.links.redirect.map(|href| {
      services::RedirectForm::from((href.redirection_url, services::Method::Get))
  });
```


And finally the error type implementation

```rust
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct CheckoutErrorResponse {
    pub request_id: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub error_codes: Option<Vec<String>>,
}
```

Similarly for every API endpoint you can implement request and response types.

### **Implementing the traits**

The `mod.rs` file contains the trait implementations where we use the types in transformers.

We create a struct with the connector name and have trait implementations for it.
The following trait implementations are mandatory

**ConnectorCommon :** contains common description of the connector, like the base endpoint, content-type, error response handling, id, currency unit.

Within the `ConnectorCommon` trait, you'll find the following methods :

  -  `id` method corresponds directly to the connector name.
  ```rust
    fn id(&self) -> &'static str {
        "checkout"
    }
  ```
  - `get_currency_unit` method anticipates you to [specify the accepted currency unit](#set-the-currency-unit) for the connector.
  ```rust
    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }
  ```
  - `common_get_content_type` method requires you to provide the accepted content type for the connector API.
  ```rust
    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }
  ```
  - `get_auth_header` method accepts common HTTP Authorization headers that are accepted in all `ConnectorIntegration` flows.
  ```rust
    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth: checkout::CheckoutAuthType = auth_type
            .try_into()
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", auth.api_secret.peek()).into_masked(),
        )])
    }
  ```

  - `base_url` method is for fetching the base URL of connector's API. Base url needs to be consumed from configs.
  ```rust
    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.checkout.base_url.as_ref()
    }
  ```
  - `build_error_response` method is  common error response handling for a connector if it is same in all cases

  ```rust 
    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: checkout::ErrorResponse = if res.response.is_empty() {
            let (error_codes, error_type) = if res.status_code == 401 {
                (
                    Some(vec!["Invalid api key".to_string()]),
                    Some("invalid_api_key".to_string()),
                )
            } else {
                (None, None)
            };
            checkout::ErrorResponse {
                request_id: None,
                error_codes,
                error_type,
            }
        } else {
            res.response
                .parse_struct("ErrorResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?
        };

        router_env::logger::info!(error_response=?response);
        let errors_list = response.error_codes.clone().unwrap_or_default();
        let option_error_code_message = conn_utils::get_error_code_error_message_based_on_priority(
            self.clone(),
            errors_list
                .into_iter()
                .map(|errors| errors.into())
                .collect(),
        );
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: option_error_code_message
                .clone()
                .map(|error_code_message| error_code_message.error_code)
                .unwrap_or(consts::NO_ERROR_CODE.to_string()),
            message: option_error_code_message
                .map(|error_code_message| error_code_message.error_message)
                .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
            reason: response
                .error_codes
                .map(|errors| errors.join(" & "))
                .or(response.error_type),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
  ```

**ConnectorIntegration :** For every api endpoint contains the url, using request transform and response transform and headers.
Within the `ConnectorIntegration` trait, you'll find the following methods implemented(below mentioned is example for authorized flow):

- `get_url` method defines endpoint for authorize flow, base url is consumed from `ConnectorCommon` trait.

```rust
  fn get_url(
      &self,
      _req: &types::PaymentsAuthorizeRouterData,
      connectors: &settings::Connectors,
  ) -> CustomResult<String, errors::ConnectorError> {
      Ok(format!("{}{}", self.base_url(connectors), "payments"))
  }
```
- `get_headers` method accepts HTTP headers that are accepted for authorize flow. In this context, it is utilized from the `ConnectorCommonExt` trait, as the connector adheres to common headers across various flows.

```rust
  fn get_headers(
      &self,
      req: &types::PaymentsAuthorizeRouterData,
      connectors: &settings::Connectors,
  ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
      self.build_headers(req, connectors)
  }
```

- `get_request_body` method calls transformers where hyperswitch payment request data is transformed into connector payment request. For constructing the request body have a function `log_and_get_request_body` that allows generic argument which is the struct that is passed as the body for connector integration, and a function that can be use to encode it into String. We log the request in this function, as the struct will be intact and the masked values will be masked.

```rust
  fn get_request_body(
      &self,
      req: &types::PaymentsAuthorizeRouterData,
      _connectors: &settings::Connectors,
  ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
      let connector_router_data = checkout::CheckoutRouterData::try_from((
          &self.get_currency_unit(),
          req.request.currency,
          req.request.amount,
          req,
      ))?;
      let connector_req = checkout::PaymentsRequest::try_from(&connector_router_data)?;
      let checkout_req = types::RequestBody::log_and_get_request_body(
          &connector_req,
          utils::Encode::<checkout::PaymentsRequest>::encode_to_string_of_json,
      )
      .change_context(errors::ConnectorError::RequestEncodingFailed)?;
      Ok(Some(checkout_req))
  }
```

- `build_request` method assembles the API request by providing the method, URL, headers, and request body as parameters.
```rust
  fn build_request(
      &self,
      req: &types::RouterData<
          api::Authorize,
          types::PaymentsAuthorizeData,
          types::PaymentsResponseData,
      >,
      connectors: &settings::Connectors,
  ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
      Ok(Some(
          services::RequestBuilder::new()
              .method(services::Method::Post)
              .url(&types::PaymentsAuthorizeType::get_url(
                  self, req, connectors,
              )?)
              .attach_default_headers()
              .headers(types::PaymentsAuthorizeType::get_headers(
                  self, req, connectors,
              )?)
              .body(types::PaymentsAuthorizeType::get_request_body(
                  self, req, connectors,
              )?)
              .build(),
    ))
  }
```
- `handle_response` method calls transformers where connector response data is transformed into hyperswitch response.
```rust
  fn handle_response(
      &self,
      data: &types::PaymentsAuthorizeRouterData,
      res: types::Response,
  ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
      let response: checkout::PaymentsResponse = res
          .response
          .parse_struct("PaymentIntentResponse")
          .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
      types::RouterData::try_from(types::ResponseRouterData {
          response,
          data: data.clone(),
          http_code: res.status_code,
      })
      .change_context(errors::ConnectorError::ResponseHandlingFailed)
  }
```
- `get_error_response` method to manage error responses. As the handling of checkout errors remains consistent across various flows, we've incorporated it from the `build_error_response` method within the `ConnectorCommon` trait.
```rust
  fn get_error_response(
      &self,
      res: types::Response,
  ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
      self.build_error_response(res)
  }
```
**ConnectorCommonExt :** An enhanced trait for `ConnectorCommon` that enables functions with a generic type. This trait includes the `build_headers` method, responsible for constructing both the common headers and the Authorization headers (retrieved from the `get_auth_header` method), returning them as a vector.

```rust 
  where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
      &self,
      req: &types::RouterData<Flow, Request, Response>,
      _connectors: &settings::Connectors,
  ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
      let header = vec![(
          headers::CONTENT_TYPE.to_string(),
          self.get_content_type().to_string().into(),
      )];
      let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
      header.append(&mut api_key);
      Ok(header)
  }
}
```

**Payment :** This trait includes several other traits and is meant to represent the functionality related to payments.

**PaymentAuthorize :** This trait extends the `api::ConnectorIntegration `trait with specific types related to payment authorization. 

**PaymentCapture :** This trait extends the `api::ConnectorIntegration `trait with specific types related to manual payment capture. 

**PaymentSync :** This trait extends the `api::ConnectorIntegration `trait with specific types related to payment retrieve. 

**Refund :** This trait includes several other traits and is meant to represent the functionality related to Refunds.

**RefundExecute :** This trait extends the `api::ConnectorIntegration `trait with specific types related to refunds create.

**RefundSync :** This trait extends the `api::ConnectorIntegration `trait with specific types related to refunds retrieve.


And the below derive traits

- **Debug**
- **Clone**
- **Copy**

There is a trait bound to implement refunds, if you don't want to implement refunds you can mark them as `todo!()` but code panics when you initiate refunds then.

Refer to other connector code for trait implementations. Mostly the rust compiler will guide you to do it easily.
Feel free to connect with us in case of any queries and if you want to confirm the status mapping.

### **Set the currency Unit**
The `get_currency_unit` function, part of the ConnectorCommon trait, enables connectors to specify their accepted currency unit as either `Base` or `Minor`. For instance, Paypal designates its currency in the base unit (for example, USD), whereas Hyperswitch processes amounts in the minor unit (for example, cents). If a connector accepts amounts in the base unit, conversion is required, as illustrated.

``` rust
impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for PaypalRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        let amount = utils::get_amount_as_string(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}
```

**Note:** Since the amount is being converted in the aforementioned `try_from`, it is necessary to retrieve amounts from `ConnectorRouterData` in all other `try_from` instances.

### **Connector utility functions**

In the `connector/utils.rs` file, you'll discover utility functions that aid in constructing connector requests and responses. We highly recommend using these helper functions for retrieving payment request fields, such as `get_billing_country`, `get_browser_info`, and `get_expiry_date_as_yyyymm`, as well as for validations, including `is_three_ds`, `is_auto_capture`, and more.

```rust
  let json_wallet_data: CheckoutGooglePayData =wallet_data.get_wallet_token_as_json()?;
```

### **Connector configs for control center**

This section is explicitly for developers who are using the [Hyperswitch Control Center](https://github.com/juspay/hyperswitch-control-center). Below is a more detailed documentation that guides you through updating the connector configuration in the `development.toml` file in Hyperswitch and running the wasm-pack build command. Please replace placeholders such as `/absolute/path/to/` with the actual absolute paths.

1. Install wasm-pack: Run the following command to install wasm-pack:

```bash 
cargo install wasm-pack
```

2. Add connector configuration:

   Open the `development.toml` file located at `crates/connector_configs/toml/development.toml` in your Hyperswitch project.

   Locate the [stripe] section as an example and add the configuration for the `example_connector`. Here's an example:

   ```toml
   # crates/connector_configs/toml/development.toml

   # Other connector configurations...

   [stripe]
   [stripe.connector_auth.HeaderKey]
   api_key="Secret Key"

   # Add any other Stripe-specific configuration here

   [example_connector]
   # Your specific connector configuration for reference
   # ...

   ```

   provide the necessary configuration details for the `example_connector`. Don't forget to save the file.

3. Update Paths:

   Replace `/absolute/path/to/hyperswitch-control-center` with the absolute path to your Hyperswitch Control Center repository and `/absolute/path/to/hyperswitch` with the absolute path to your Hyperswitch repository.

4. Run `wasm-pack` Build:

   Execute the following command in your terminal:

   ```bash
   wasm-pack build --target web --out-dir /absolute/path/to/hyperswitch-control-center/public/hyperswitch/wasm --out-name euclid /absolute/path/to/hyperswitch/crates/euclid_wasm -- --features dummy_connector
   ```

   This command builds the WebAssembly files for the `dummy_connector` feature and places them in the specified directory.

Notes:

- Ensure that you replace placeholders like `/absolute/path/to/` with the actual absolute paths in your file system.
- Verify that your connector configurations in `development.toml` are correct and saved before running the `wasm-pack` command.
- Check for any error messages during the build process and resolve them accordingly.

By following these steps, you should be able to update the connector configuration and build the WebAssembly files successfully.

### **Test the connector**

The template code script generates a test file for the connector, containing 20 sanity tests. We anticipate that you will implement these tests when adding a new connector.

```rust
// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[serial_test::serial]
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}
```

Utility functions for tests are also available at `tests/connector/utils`. These functions enable you to write tests with ease.

```rust
    /// For initiating payments when `CaptureMethod` is set to `Manual`
    /// This doesn't complete the transaction, `PaymentsCapture` needs to be done manually
    async fn authorize_payment(
        &self,
        payment_data: Option<types::PaymentsAuthorizeData>,
        payment_info: Option<PaymentInfo>,
    ) -> Result<types::PaymentsAuthorizeRouterData, Report<ConnectorError>> {
        let integration = self.get_data().connector.get_connector_integration();
        let mut request = self.generate_data(
            types::PaymentsAuthorizeData {
                confirm: true,
                capture_method: Some(diesel_models::enums::CaptureMethod::Manual),
                ..(payment_data.unwrap_or(PaymentAuthorizeType::default().0))
            },
            payment_info,
        );
        let tx: oneshot::Sender<()> = oneshot::channel().0;
        let state = routes::AppState::with_storage(
            Settings::new().unwrap(),
            StorageImpl::PostgresqlTest,
            tx,
            Box::new(services::MockApiClient),
        )
        .await;
        integration.execute_pretasks(&mut request, &state).await?;
        Box::pin(call_connector(request, integration)).await
    }
```

Prior to executing tests in the shell, ensure that the API keys are configured in `crates/router/tests/connectors/sample_auth.toml` and set the environment variable `CONNECTOR_AUTH_FILE_PATH` using the export command. Avoid pushing code with exposed API keys.

```rust
  export CONNECTOR_AUTH_FILE_PATH="/hyperswitch/crates/router/tests/connectors/sample_auth.toml"
  cargo test --package router --test connectors -- checkout --test-threads=1  
```
All tests should pass and add appropriate tests for connector specific payment flows.

### **Build payment request and response from json schema**

Some connectors will provide [json schema](https://developer.worldpay.com/docs/access-worldpay/api/references/payments) for each request and response supported. We can directly convert that schema to rust code by using below script. On running the script a `temp.rs` file will be created in `src/connector/<connector-name>` folder

_Note: The code generated may not be production ready and might fail for some case, we have to clean up the code as per our standards._

```bash
brew install openapi-generator
export CONNECTOR_NAME="<CONNECTOR-NAME>" #Change it to appropriate connector name
export SCHEMA_PATH="<PATH-TO-JSON-SCHEMA-FILE>" #it can be json or yaml, Refer samples below
openapi-generator generate -g rust  -i ${SCHEMA_PATH} -o temp &&  cat temp/src/models/* > crates/router/src/connector/${CONNECTOR_NAME}/temp.rs && rm -rf temp && sed -i'' -r "s/^pub use.*//;s/^pub mod.*//;s/^\/.*//;s/^.\*.*//;s/crate::models:://g;" crates/router/src/connector/${CONNECTOR_NAME}/temp.rs && cargo +nightly fmt
```

JSON example

```json
{
  "openapi": "3.0.1",
  "paths": {},
  "info": {
    "title": "",
    "version": ""
  },
  "components": {
    "schemas": {
      "PaymentsResponse": {
        "type": "object",
        "properties": {
          "outcome": {
            "type": "string"
          }
        },
        "required": ["outcome"]
      }
    }
  }
}
```

YAML example

```yaml
---
openapi: 3.0.1
paths: {}
info:
  title: ""
  version: ""
components:
  schemas:
    PaymentsResponse:
      type: object
      properties:
        outcome:
          type: string
      required:
        - outcome
```
