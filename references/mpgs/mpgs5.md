Retrieve Order
Request to retrieve the details of an order and all transactions associated with this order.

GET
https://ap-gateway.mastercard.com/api/rest/version/100/merchant/{merchantId}/order/{orderid}Authentication
This operation requires authentication via one of the following methods:


Certificate authentication.
Basic HTTP authentication as described at w3.org. Provide 'merchant.<your gateway merchant ID>' in the userid portion and your API password in the password portion.
Request
URL Parameters
{merchantId}
Alphanumeric + additional characters
REQUIRED
The unique identifier issued to you by your payment provider.


This identifier can be up to 12 characters in length.


Data may consist of the characters 0-9, a-z, A-Z, '-', '_'

Min length: 1 Max length: 40
{orderid}
String
REQUIRED
A unique identifier for this order to distinguish it from any other order you create.


Use this identifier when referring to this order in subsequent transactions and in retrieval operations. This value must be unique for every order you create using your merchant profile.


Data can consist of any characters

Min length: 1 Max length: 40
Fields
Show optional fields 
To view the optional fields, please toggle on the "Show optional fields" setting.

Response
Fields
Show conditional fields 
amount
Decimal
ALWAYS PROVIDED
The total amount for the order.  This is the net amount plus any merchant charge amounts.If you provide any sub-total amounts, then the sum of these amounts (order.itemAmount, order.taxAmount, order.shippingAndHandlingAmount, order.cashbackAmount, order.gratuityAmount, order.merchantCharge.amount and order.dutyAmount), minus the order.discountAmount must equal the net amount.

The value of this field in the response is zero if payer funds are not transferred.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
creationTime
DateTime
ALWAYS PROVIDED
Indicates the date and time the gateway considers the order to have been created.

An instant in time expressed in ISO8601 date + time format - "YYYY-MM-DDThh:mm:ss.SSSZ"

currency
Upper case alphabetic text
ALWAYS PROVIDED
The currency of the order expressed as an ISO 4217 alpha code, e.g. USD.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
id
String
ALWAYS PROVIDED
A unique identifier for this order to distinguish it from any other order you create.

Use this identifier when referring to this order in subsequent transactions and in retrieval operations. This value must be unique for every order created by your merchant profile.

Data can consist of any characters

Min length: 1 Max length: 40
lastUpdatedTime
DateTime
ALWAYS PROVIDED
Indicates the date and time the gateway considers the order to have last been updated.

An instant in time expressed in ISO8601 date + time format - "YYYY-MM-DDThh:mm:ss.SSSZ"

merchant
Alphanumeric + additional characters
ALWAYS PROVIDED
The unique identifier issued to you by your payment provider.

This identifier can be up to 12 characters in length.

Data may consist of the characters 0-9, a-z, A-Z, '-', '_'

Min length: 1 Max length: 40
merchantAmount
Decimal
ALWAYS PROVIDED
The total amount for the order in order.merchantCurrency units.

This is derived from the rate quote and order.amount for this order when Multi-Currency Pricing was used.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
merchantCurrency
Upper case alphabetic text
ALWAYS PROVIDED
The currency in which you priced your inventory for this order, expressed as an ISO 4217 alpha code, e.g. USD.

This value (along with merchantAmount) is applicable if you are doing Multi-Currency Pricing, as it gives you a consistent currency across all your orders that involve foreign exchange (FX).

If there is FX on this order, this is based on the rate quote you provided on the payment transactions, if not then this is the order.currency.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
result
Enumeration
ALWAYS PROVIDED
A system-generated high level overall result of the operation.

Value must be a member of the following list. The values are case sensitive.

FAILURE
The operation was declined or rejected by the gateway, acquirer or issuer

PENDING
The operation is currently in progress or pending processing

SUCCESS
The operation was successfully processed

UNKNOWN
The result of the operation is unknown

totalAuthorizedAmount
Decimal
ALWAYS PROVIDED
The amount that has been successfully authorized for this order including any amount adjustments made via incremental authorizations or partial reversals.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
totalCapturedAmount
Decimal
ALWAYS PROVIDED
The amount that has been successfully captured for this order.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
totalDisbursedAmount
Decimal
ALWAYS PROVIDED
The amount that has been successfully disbursed for this order.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
totalRefundedAmount
Decimal
ALWAYS PROVIDED
The amount that has been successfully refunded for this order.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
Errors
error
Information on possible error conditions that may occur while processing an operation using the API.

error.cause
Enumeration
Broadly categorizes the cause of the error.

For example, errors may occur due to invalid requests or internal system failures.

Value must be a member of the following list. The values are case sensitive.

INVALID_REQUEST
The request was rejected because it did not conform to the API protocol.

REQUEST_REJECTED
The request was rejected due to security reasons such as firewall rules, expired certificate, etc.

SERVER_BUSY
The server did not have enough resources to process the request at the moment.

SERVER_FAILED
There was an internal system failure.

error.explanation
String
Textual description of the error based on the cause.

This field is returned only if the cause is INVALID_REQUEST or SERVER_BUSY.

Data can consist of any characters

Min length: 1 Max length: 1000
error.field
String
Indicates the name of the field that failed validation.

This field is returned only if the cause is INVALID_REQUEST and a field level validation error was encountered.

Data can consist of any characters

Min length: 1 Max length: 100
error.supportCode
String
Indicates the code that helps the support team to quickly identify the exact cause of the error.

This field is returned only if the cause is SERVER_FAILED or REQUEST_REJECTED.

Data can consist of any characters

Min length: 1 Max length: 100
error.validationType
Enumeration
Indicates the type of field validation error.

This field is returned only if the cause is INVALID_REQUEST and a field level validation error was encountered.

Value must be a member of the following list. The values are case sensitive.

INVALID
The request contained a field with a value that did not pass validation.

MISSING
The request was missing a mandatory field.

UNSUPPORTED
The request contained a field that is unsupported.

result
Enumeration
A system-generated high level overall result of the operation.

Value must be a member of the following list. The values are case sensitive.

ERROR
The operation resulted in an error and hence cannot be processed.