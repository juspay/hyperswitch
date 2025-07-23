Void
Request to void a previous transaction. A void will reverse a previous transaction. Typically voids will only be successful when processed not long after the original transaction.

PUT
https://ap-gateway.mastercard.com/api/rest/version/100/merchant/{merchantId}/order/{orderid}/transaction/{transactionid}
Authentication
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
{transactionid}
String
REQUIRED
Unique identifier for this transaction to distinguish it from any other transaction on the order.


An order can have transactions representing:
Movement of money. For example, payments and refunds.
Validations. For example, account verification or 3-D Secure authentication of the payer.
Undoing other transactions. For example, voiding a payment transaction.
Chargebacks.
Fees from your payment service provider.
Each transaction on the order must have a unique id that identifies that transaction. Some transactions also hold the transaction identifier of other transactions on the order. For example a void payment transaction references the original payment transaction that is being voided.

If you attempt an operation and it fails (eg you try to PAY on a card with no funds), then you need a new id for each retry.


Data can consist of any characters

Min length: 1 Max length: 40
Fields
Show optional fields 
apiOperation
String
= VOID
FIXED
Any sequence of zero or more unicode characters.

transaction
REQUIRED
Information about this transaction.

transaction.targetTransactionId
String
REQUIRED
The identifier for the transaction you wish to void.

That is the {transactionId} URL field for REST and the transaction.id field for NVP.

Data can consist of any characters

Min length: 1 Max length: 40
Response
Fields
Show conditional fields 
merchant
Alphanumeric + additional characters
ALWAYS PROVIDED
The unique identifier issued to you by your payment provider.

This identifier can be up to 12 characters in length.

Data may consist of the characters 0-9, a-z, A-Z, '-', '_'

Min length: 1 Max length: 40
order
ALWAYS PROVIDED
Information about the order associated with this transaction.

order.amount
Decimal
ALWAYS PROVIDED
The total amount for the order.  This is the net amount plus any merchant charge amounts.If you provide any sub-total amounts, then the sum of these amounts (order.itemAmount, order.taxAmount, order.shippingAndHandlingAmount, order.cashbackAmount, order.gratuityAmount, order.merchantCharge.amount and order.dutyAmount), minus the order.discountAmount must equal the net amount.

The value of this field in the response is zero if payer funds are not transferred.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
order.creationTime
DateTime
ALWAYS PROVIDED
Indicates the date and time the gateway considers the order to have been created.

An instant in time expressed in ISO8601 date + time format - "YYYY-MM-DDThh:mm:ss.SSSZ"

order.currency
Upper case alphabetic text
ALWAYS PROVIDED
The currency of the order expressed as an ISO 4217 alpha code, e.g. USD.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
order.id
String
ALWAYS PROVIDED
A unique identifier for this order to distinguish it from any other order you create.

Use this identifier when referring to this order in subsequent transactions and in retrieval operations. This value must be unique for every order created by your merchant profile.

Data can consist of any characters

Min length: 1 Max length: 40
order.lastUpdatedTime
DateTime
ALWAYS PROVIDED
Indicates the date and time the gateway considers the order to have last been updated.

An instant in time expressed in ISO8601 date + time format - "YYYY-MM-DDThh:mm:ss.SSSZ"

order.merchantAmount
Decimal
ALWAYS PROVIDED
The total amount for the order in order.merchantCurrency units.

This is derived from the rate quote and order.amount for this order when Multi-Currency Pricing was used.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
order.merchantCurrency
Upper case alphabetic text
ALWAYS PROVIDED
The currency in which you priced your inventory for this order, expressed as an ISO 4217 alpha code, e.g. USD.

This value (along with merchantAmount) is applicable if you are doing Multi-Currency Pricing, as it gives you a consistent currency across all your orders that involve foreign exchange (FX).

If there is FX on this order, this is based on the rate quote you provided on the payment transactions, if not then this is the order.currency.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
order.totalAuthorizedAmount
Decimal
ALWAYS PROVIDED
The amount that has been successfully authorized for this order including any amount adjustments made via incremental authorizations or partial reversals.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
order.totalCapturedAmount
Decimal
ALWAYS PROVIDED
The amount that has been successfully captured for this order.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
order.totalDisbursedAmount
Decimal
ALWAYS PROVIDED
The amount that has been successfully disbursed for this order.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
order.totalRefundedAmount
Decimal
ALWAYS PROVIDED
The amount that has been successfully refunded for this order.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
response
ALWAYS PROVIDED
response.gatewayCode
Enumeration
ALWAYS PROVIDED
Summary of the success or otherwise of the operation.

Value must be a member of the following list. The values are case sensitive.

ABORTED
Transaction aborted by payer

ACQUIRER_SYSTEM_ERROR
Acquirer system error occurred processing the transaction

APPROVED
Transaction Approved

APPROVED_AUTO
The transaction was automatically approved by the gateway. it was not submitted to the acquirer.

APPROVED_PENDING_SETTLEMENT
Transaction Approved - pending batch settlement

AUTHENTICATION_FAILED
Payer authentication failed

AUTHENTICATION_IN_PROGRESS
The operation determined that payer authentication is possible for the given card, but this has not been completed, and requires further action by the merchant to proceed.

BALANCE_AVAILABLE
A balance amount is available for the card, and the payer can redeem points.

BALANCE_UNKNOWN
A balance amount might be available for the card. Points redemption should be offered to the payer.

BLOCKED
Transaction blocked due to Risk or 3D Secure blocking rules

CANCELLED
Transaction cancelled by payer

DECLINED
The requested operation was not successful. For example, a payment was declined by issuer or payer authentication was not able to be successfully completed.

DECLINED_AVS
Transaction declined due to address verification

DECLINED_AVS_CSC
Transaction declined due to address verification and card security code

DECLINED_CSC
Transaction declined due to card security code

DECLINED_DO_NOT_CONTACT
Transaction declined - do not contact issuer

DECLINED_INVALID_PIN
Transaction declined due to invalid PIN

DECLINED_PAYMENT_PLAN
Transaction declined due to payment plan

DECLINED_PIN_REQUIRED
Transaction declined due to PIN required

DEFERRED_TRANSACTION_RECEIVED
Deferred transaction received and awaiting processing

DUPLICATE_BATCH
Transaction declined due to duplicate batch

EXCEEDED_RETRY_LIMIT
Transaction retry limit exceeded

EXPIRED_CARD
Transaction declined due to expired card

INSUFFICIENT_FUNDS
Transaction declined due to insufficient funds

INVALID_CSC
Invalid card security code

LOCK_FAILURE
Order locked - another transaction is in progress for this order

NOT_ENROLLED_3D_SECURE
Card holder is not enrolled in 3D Secure

NOT_SUPPORTED
Transaction type not supported

NO_BALANCE
A balance amount is not available for the card. The payer cannot redeem points.

PARTIALLY_APPROVED
The transaction was approved for a lesser amount than requested. The approved amount is returned in order.totalAuthorizedAmount.

PENDING
Transaction is pending

REFERRED
Transaction declined - refer to issuer

SUBMITTED
The transaction has successfully been created in the gateway. It is either awaiting submission to the acquirer or has been submitted to the acquirer but the gateway has not yet received a response about the success or otherwise of the payment.

SYSTEM_ERROR
Internal system error occurred processing the transaction

TIMED_OUT
The gateway has timed out the request to the acquirer because it did not receive a response. Points redemption should not be offered to the payer.

UNKNOWN
The transaction has been submitted to the acquirer but the gateway was not able to find out about the success or otherwise of the payment. If the gateway subsequently finds out about the success of the payment it will update the response code.

UNSPECIFIED_FAILURE
Transaction could not be processed

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

transaction
ALWAYS PROVIDED
Information about this transaction.

transaction.acquirer
ALWAYS PROVIDED
transaction.amount
Decimal
ALWAYS PROVIDED
The total amount for the transaction.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
transaction.currency
Upper case alphabetic text
ALWAYS PROVIDED
The currency of the transaction expressed as an ISO 4217 alpha code, e.g. USD.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
transaction.id
String
ALWAYS PROVIDED
Unique identifier for this transaction to distinguish it from any other transaction on the order.

An order can have transactions representing:
Movement of money. For example, payments and refunds.
Validations. For example, account verification or 3-D Secure authentication of the payer.
Undoing other transactions. For example, voiding a payment transaction.
Chargebacks.
Fees from your payment service provider.
Each transaction on the order must have a unique id that identifies that transaction. Some transactions also hold the transaction identifier of other transactions on the order. For example a void payment transaction references the original payment transaction that is being voided.

If you attempt an operation and it fails (eg you try to PAY on a card with no funds), then you need a new id for each retry.

Data can consist of any characters

Min length: 1 Max length: 40
transaction.type
Enumeration
ALWAYS PROVIDED
Indicates the type of action performed on the order.

Value must be a member of the following list. The values are case sensitive.

AUTHENTICATION
Authentication

AUTHORIZATION
Authorization

AUTHORIZATION_UPDATE
Authorization Update

CAPTURE
Capture

CHARGEBACK
Chargeback

DISBURSEMENT
Disbursement

FUNDING
The transaction transfers money to or from the merchant, without the involvement of a payer. For example, recording monthly merchant service fees from your payment service provider.

PAYMENT
Payment (Purchase)

REFUND
Refund

REFUND_REQUEST
Refund Request

VERIFICATION
Verification

VOID_AUTHORIZATION
Void Authorization

VOID_CAPTURE
Void Capture

VOID_PAYMENT
Void Payment

VOID_REFUND
Void Refund

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