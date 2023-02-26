# Guide to Integrate a Connector

## Introduction

This is a guide to contributing new connector to Router. This guide includes instructions on checking out the source code, integrating and testing the new connector, and finally contributing the new connector back to the project.

## Prerequisites

- Understanding of the Connector APIs which you wish to integrate with Router
- Setup of Router repository and running it on local
- Access to API credentials for testing the Connector API (you can quickly sign up for sandbox/uat credentials by visiting the website of the connector you wish to integrate)

In Router, there are Connectors and Payment Methods, examples of both are shown below from which the difference is apparent.

### What is a Connector ?

A connector is an integration to fulfill payments. Related use cases could be any of the below

- Payment processor (Stripe, Adyen, ChasePaymentTech etc.,)
- Fraud and Risk management platform (like Ravelin, Riskified etc.,)
- Payment network (Visa, Master)
- Payment authentication services (Cardinal etc.,)
  Router supports "Payment Processors" right now. Support will be extended to the other categories in the near future.

### What is a Payment Method ?

Each Connector (say, a Payment Processor) could support multiple payment methods

- **Cards :** Bancontact , Knet, Mada
- **Bank Transfers :** EPS , giropay, sofort
- **Bank Direct Debit :** Sepa direct debit
- **Wallets :** Apple Pay , Google Pay , Paypal

Cards and Bank Transfer payment methods are already included in Router. Hence, adding a new connector which offers payment_methods available in Router is easy and requires almost no breaking changes.
Adding a new payment method (say Wallets or Bank Direct Debit) might require some changes in core business logic of Router, which we are actively working upon.

## How to Integrate a Connector

Most of the code to be written is just another API integration. You have to write request and response types for API of the connector you wish to integrate and implement required traits.

For this tutorial we will be integrating card payment through the [Checkout.com connector](https://www.checkout.com/).
Go through the [Checkout.com API reference](https://api-reference.checkout.com/). It would also be helpful to try out the API's, using tools like on postman, or any other API testing tool before starting the integration.

Below is a step-by-step tutorial for integrating a new connector.

### **Generate the template**

```bash
cd scripts
sh add_connector.sh <connector-name>
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
pub struct CheckoutPaymentsRequest {
    pub source: Source,
    pub amount: i64,
    pub currency: String,
    #[serde(default = "generate_processing_channel_id")]
    pub processing_channel_id: Cow<'static, str>,
}

fn generate_processing_channel_id() -> Cow<'static, str> {
    "pc_e4mrdrifohhutfurvuawughfwu".into()
}
```

Since Router is connector agnostic, only minimal data is sent to connector and optional fields may be ignored.

Here processing_channel_id, is specific to checkout and implementations of such functions should be inside the checkout directory.
Let's define `Source`

```rust
#[derive(Debug, Serialize)]
pub struct CardSource {
    #[serde(rename = "type")]
    pub source_type: Option<String>,
    pub number: Option<String>,
    pub expiry_month: Option<String>,
    pub expiry_year: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Source {
    Card(CardSource),
    // TODO: Add other sources here.
}
```

`Source` is an enum type. Request types will need to derive `Serialize` and response types will need to derive `Deserialize`. For request types `From<RouterData>` needs to be implemented.

```rust
impl<'a> From<&types::RouterData<'a>> for CheckoutRequestType
```

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CheckoutPaymentStatus {
    Authorized,
    Pending,
    #[serde(rename = "Card Verified")]
    CardVerified,
    Declined,
}
```

The important part is mapping it to the Router status codes.

```rust
impl From<CheckoutPaymentStatus> for enums::AttemptStatus {
    fn from(item: CheckoutPaymentStatus) -> Self {
        match item {
            CheckoutPaymentStatus::Authorized => enums::AttemptStatus::Charged,
            CheckoutPaymentStatus::Declined => enums::AttemptStatus::Failure,
            CheckoutPaymentStatus::Pending => enums::AttemptStatus::Authorizing,
            CheckoutPaymentStatus::CardVerified => enums::AttemptStatus::Pending,
        }
    }
}
```

Note: `enum::AttemptStatus` is Router status.

Router status are given below

- **Charged :** The amount has been debited
- **PendingVBV :** Pending but verified by visa
- **Failure :** The payment Failed
- **Authorizing :** In the process of authorizing.

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

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckoutPaymentsResponse {
    id: String,
    amount: i64,
    status: CheckoutPaymentStatus,
}

impl<'a> From<types::ResponseRouterData<'a, CheckoutPaymentsResponse>> for types::RouterData<'a> {
    fn from(item: types::ResponseRouterData<'a, CheckoutPaymentsResponse>) -> Self {
        types::RouterData {
            connector_transaction_id: Some(item.response.id),
            amount_received: Some(item.response.amount),
            status: enums::Status::from(item.response.status),
            ..item.data
        }
    }
}
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

There are four types of tasks that are done by implementing traits:

- **Payment :** For making/initiating payments
- **PaymentSync :** For checking status of the payment
- **Refund :** For initiating refund
- **RefundSync :** For checking status of the Refund.

We create a struct with the connector name and have trait implementations for it.
The following trait implementations are mandatory

- **ConnectorCommon :** contains common description of the connector, like the base endpoint, content-type, error message, id.
- **Payment :** Trait Relationship, has impl block.
- **PaymentAuthorize :** Trait Relationship, has impl block.
- **ConnectorIntegration :** For every api endpoint contains the url, using request transform and response transform and headers.
- **Refund :** Trait Relationship, has empty body.
- **RefundExecute :** Trait Relationship, has empty body.
- **RefundSync :** Trait Relationship, has empty body.

And the below derive traits

- **Debug**
- **Clone**
- **Copy**

There is a trait bound to implement refunds, if you don't want to implement refunds you can mark them as `todo!()` but code panics when you initiate refunds then.

Don’t forget to add logs lines in appropriate places.
Refer to other connector code for trait implementations. Mostly the rust compiler will guide you to do it easily.
Feel free to connect with us in case of any queries and if you want to confirm the status mapping.

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
