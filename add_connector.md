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
  Router supports "Payment Processors" right now. Support will be extended to the other categories in the near future.

### What is a Payment Method ?

Each Connector (say, a Payment Processor) could support multiple payment methods

- **Cards :** Visa, Mastercard, Bancontact , Knet, Mada, Discover, UnionPay
**Bank Redirects :** Ideal, EPS , Giropay, Sofort, Bancontact, Bizum, Blik, Interac, Online Banking Czech Republic, Online Banking Finland, Online Banking Poland, Online Banking Slovakia, Online Banking UK, Prezelwy24, Trustly, Online Banking Fpx, Online Banking Thailand
- **Bank Transfers :** Multibanco, Sepa, Bacs, Ach, Permata, Bca, Bni, Bri Va, Danamon Va Bank, Pix, Pse
- **Bank Direct Debit :** Sepa direct debit, Ach Debit, Becs Bank Debit, Bacs Bank Debit
- **Wallets :** Apple Pay , Google Pay , Paypal , Ali pay ,Mb pay ,Samsung Pay, Wechat Pay, TouchNGo, Cashapp
- **Card Redirect :** Knet, Benefit, MomoAtm
- **PayLater :** Klarna, Affirm, Afterpay, Paybright, Walley, Alma, Atome
- **Crypto :**  Crypto Currency
- **Reward :** Classic
- **Voucher :** Boleto, Efecty, PagoEfectivo, RedCompra, RedPagos, Alfarmart, Indomaret, Oxxo, SevenEleven, Lawson, MiniStop, FamilyMart, Seicomart, PayEasy
- **GiftCard :** Givex, Pay Safe Card
- **Upi :** Upi Collect

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

Using an enum for a response struct in Rust is not recommended due to potential deserialization issues where the deserializer attempts to deserialize into all the enum variants. A preferable alternative is to employ a separate enum for the possible response variants and include it as a field within the response struct. To implement the redirection flow, assign the redirection link to the `redirection_data`.

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

There are four types of tasks that are done by implementing traits:

- **Payment :** For making/initiating payments
- **PaymentSync :** For checking status of the payment
- **Refund :** For initiating refund
- **RefundSync :** For checking status of the Refund.

We create a struct with the connector name and have trait implementations for it.
The following trait implementations are mandatory

- **ConnectorCommon :** contains common description of the connector, like the base endpoint, content-type, error message, id, currency unit.
- **ConnectorIntegration :** For every api endpoint contains the url, using request transform and response transform and headers.
- **Payment :** This trait includes several other traits and is meant to represent the functionality related to payments.
- **PaymentAuthorize :** This trait extends the `api::ConnectorIntegration `trait with specific types related to payment authorization. 
- **PaymentCapture :** This trait extends the `api::ConnectorIntegration `trait with specific types related to manual payment capture. 
- **PaymentSync :** This trait extends the `api::ConnectorIntegration `trait with specific types related to payment retrieve. 
- **Refund :** This trait includes several other traits and is meant to represent the functionality related to Refunds.
- **RefundExecute :** This trait extends the `api::ConnectorIntegration `trait with specific types related to refunds create.
- **RefundSync :** This trait extends the `api::ConnectorIntegration `trait with specific types related to refunds retrieve.


And the below derive traits

- **Debug**
- **Clone**
- **Copy**

There is a trait bound to implement refunds, if you don't want to implement refunds you can mark them as `todo!()` but code panics when you initiate refunds then.

Don’t forget to add logs lines in appropriate places.
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

### **Test the connector**

Try running the tests in `crates/router/tests/connectors/{{connector-name}}.rs`.
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
