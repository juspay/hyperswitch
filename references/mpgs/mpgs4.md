Refund
Request to refund previously captured funds to the payer. Typically, a Refund is linked to the Capture or Pay through the orderId - you provide the original orderId, a new transactionId, and the amount you wish to refund. You may provide other fields if you want to update their values; however, you must NOT provide sourceOfFunds.
In rare situations, you may want to refund the payer without associating the credit to a previous transaction (see Standalone Refund). In this case, you need to provide the sourceOfFunds and a new orderId.


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
accountFunding
OPTIONAL
Additional details for account funding transactions (order.purchaseType=ACCOUNT_FUNDING).

Account funding transactions are transactions that pull money from the sender's card account for the purpose of funding another account, the recipient's account. Depending on the type of account funding transaction you may be required to provide some or all the details in this parameter group.

accountFunding.purpose
Enumeration
OPTIONAL
Defines the purpose of the account funding payment.If not provided the value is defaulted to OTHER.

Value must be a member of the following list. The values are case sensitive.

CRYPTOCURRENCY_PURCHASE
The funds from this account funding transaction will exclusively be used to purchase cryptocurrency.

MERCHANT_SETTLEMENT
The funds from this account funding transaction will be used to settle the proceeds of processing card transactions.

OTHER
The funds from this account funding transaction will be used for any other purpose, e.g. transferring funds from a person to a person or transferring funds into a staged wallet. This is the default value.

PAYROLL
The funds from this account funding transaction will be used to pay salaries.

accountFunding.recipient
OPTIONAL
Details about the recipient who will subsequently receive the funds that you are debiting from the sender in this transaction.

accountFunding.recipient.account
OPTIONAL
Details about the account of recipient who will subsequently receive the funds that you are debiting from the sender in this transaction.

accountFunding.recipient.account.fundingMethod
Enumeration
OPTIONAL
If the recipient account type is an account with an associated card (accountFunding.recipient.account.identifierType=CARD_NUMBER) you must specify the funding method of the card.

If not provided the value is defaulted to UNKNOWN.

Value must be a member of the following list. The values are case sensitive.

CHARGE
The payer has a line of credit with the issuer which must be paid off monthly.

CREDIT
The payer has a revolving line of credit with the issuer.

DEBIT
Funds are immediately debited from the payer's account with the issuer.

UNKNOWN
The account funding method is not known. This is the default value.

accountFunding.recipient.account.identifier
String
OPTIONAL
The account identifier for the payment recipient's account.

For example, this may be a card number or bank account number. You must specify the type of identifier in field accountFunding.recipient.account.identifierType. In the response, the value will be masked. The masking format depends on the type of account identifier.

Data can consist of any characters

Min length: 1 Max length: 50
accountFunding.recipient.account.identifierType
Enumeration
OPTIONAL
Defines the type of the recipient's account identifier that you have provided in field accountFunding.recipient.account.identifier.

If not provided the value is defaulted to OTHER.

Value must be a member of the following list. The values are case sensitive.

BANK_ACCOUNT_BIC
The recipient's account identifier is a bank account number and Business Identifier Code (BIC).

BANK_ACCOUNT_IBAN
The recipient's account identifier is an International Bank Account Number (IBAN).

BANK_ACCOUNT_NATIONAL
The recipient's account identifier is a bank account number and a national bank identifier, for example, a routing number (RTN).

CARD_NUMBER
The recipient's account identifier is a card number.

EMAIL_ADDRESS
The recipient's account identifier is an email address.

OTHER
The recipient's account identifier type can not be classified using any of the other categories. This is the default value

PHONE_NUMBER
The recipient's account identifier is a phone number.

SOCIAL_NETWORK_PROFILE_ID
The recipient's account identifier is a social network profile ID.

STAGED_WALLET_USER_ID
The recipient's account identifier is a user ID for a staged digital wallet. For a staged wallet, when the payer makes a payment using the wallet, the funds are pulled from an account associated with the wallet (first stage) before they are credited to the recipient of the wallet payment (second stage).

STORED_VALUE_WALLET_USER_ID
The recipient's account identifier is a user ID for a stored value wallet. A stored value wallet requires the payer to preload the wallet with funds before they can use the wallet to make a payment.

accountFunding.recipient.address
OPTIONAL
Details of the recipient's address.

accountFunding.recipient.address.city
String
OPTIONAL
The city portion of the address.

Data can consist of any characters

Min length: 1 Max length: 100
accountFunding.recipient.address.country
Upper case alphabetic text
OPTIONAL
The 3 letter ISO standard alpha country code of the address.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
accountFunding.recipient.address.postCodeZip
String
OPTIONAL
The post code or zip code of the address.

Data can consist of any characters

Min length: 1 Max length: 10
accountFunding.recipient.address.stateProvinceCode
String
OPTIONAL
The second part of the ISO 3166-2 country subdivision code for the state or province of the address.

For an address in the United States provide the 2-letter ISO 3166-2 state code. For US military bases provide one of AE, AA, AP. For an address in Canada provide the 2-letter ISO 3166-2 province code.

Data can consist of any characters

Min length: 1 Max length: 3
accountFunding.recipient.address.street
String
OPTIONAL
The first line of the address.

Data can consist of any characters

Min length: 1 Max length: 100
accountFunding.recipient.address.street2
String
OPTIONAL
The second line of the address.

Data can consist of any characters

Min length: 1 Max length: 100
accountFunding.recipient.firstName
String
OPTIONAL
First name of the recipient.

Data can consist of any characters

Min length: 1 Max length: 50
accountFunding.recipient.identification
OPTIONAL
Identification of the recipient.

accountFunding.recipient.identification.country
Upper case alphabetic text
OPTIONAL
The ISO 3166 three-letter country code of the issuer of the identification.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
accountFunding.recipient.identification.type
Enumeration
OPTIONAL
The type of identification provided for the recipient.

Value must be a member of the following list. The values are case sensitive.

ALIEN_REGISTRATION_NUMBER
The recipient's identification type is an alien registration number issued by U.S. Citizenship and Immigration Services (USCIS) to immigrants who apply to live in the United States.

BUSINESS_TAX_ID
The recipient's identification type is a business tax id which is assigned to the business entity by the tax department.

COMPANY_REGISTRATION_NUMBER
The recipient's identification type is a company registration number which is issued to the company at the time of its incorporation.

CUSTOMER_IDENTIFICATION
The recipient's identification type is an unspecified form of customer identification through which the recipient can be identified and verified.

DATE_OF_BIRTH
The recipient's identification type is date of birth.

DRIVERS_LICENSE
The recipient's identification type is a driving license.

EMAIL
The recipient's identification type is an email address.

GOVERNMENT_ISSUED
The recipient's identification type is government issued.

INDIVIDUAL_TAX_ID
The recipient's identification type is an individual tax id which is assigned to the individual by the tax department.

LAW_ENFORCEMENT_IDENTIFICATION
The recipient's identification type is a law enforcement identification.

MILITARY_IDENTIFICATION
The recipient's identification type is a military identification which is issued by the department of defense and military.

NATIONAL_IDENTIFICATION_CARD
The recipient's identification type is a national identification card issued by the recipient's country.

OTHER
The recipient's identification type cannot be classified using any of the other categories. This is the default value.

PASSPORT
The recipient's identification type is a passport.

PHONE_NUMBER
The recipient's identification type is a phone number.

PROXY_IDENTIFICATION
The recipient's identification type is a proxy identification.

SOCIAL_SECURITY_NUMBER
The recipient's identification type is a social security number issued by Social Security Administration (SSA) to U.S. citizens, permanent residents and eligible non-immigrant workers in the United States.

TRAVEL_IDENTIFICATION
The recipient's identification type is a travel document other than a passport.

accountFunding.recipient.identification.value
String
OPTIONAL
The identification value/number for the type of identification provided in accountFunding.recipient.identification.type.

Data can consist of any characters

Min length: 1 Max length: 50
accountFunding.recipient.lastName
String
OPTIONAL
Last name of the recipient.

Data can consist of any characters

Min length: 1 Max length: 50
accountFunding.recipient.middleName
String
OPTIONAL
Middle name of the recipient.

Data can consist of any characters

Min length: 1 Max length: 50
accountFunding.senderIsRecipient
Boolean
OPTIONAL
Defines if the sender and recipient of the account funding payment are the same or not.

If not provided the value is defaulted to FALSE.

JSON boolean values 'true' or 'false'.

accountFunding.senderType
Enumeration
OPTIONAL
Defines if the sender is a person, a commercial organization, a non-profit organization or a government

Value must be a member of the following list. The values are case sensitive.

COMMERCIAL_ORGANIZATION
The sender is a commercial organization. Examples include account to account transfers initiated by a commercial organization for the purpose of transferring funds to one of their accounts, business to business payments, and disbursements for insurance claims, payroll, investment dividends, merchant rebates.

GOVERNMENT
The sender is a government or government agency. Examples include government agencies paying salaries, pensions, social benefits or tax credits.

NON_PROFIT_ORGANIZATION
The sender is a non-profit organization. Examples include non-profit organizations delivering emergency aid payments.

PERSON
The sender is a person. Examples include account to account transfers initiated by a person to their own account or a different person's account and adding funds to a staged wallet.

action
OPTIONAL
Actions that you want the gateway to perform.

action.refundAuthorization
Boolean
OPTIONAL
Use this field to indicate that you want the gateway to authorize the Refund with the issuer before submitting it to the acquirer.

Depending on your merchant profile configuration the gateway may or may not already attempt to authorize the Refund with the issuer before submitting it to the acquirer.

JSON boolean values 'true' or 'false'.

agreement
OPTIONAL
A commercial agreement you have with the payer that allows you to store and use their payment details for later payments.

For example, an agreement to a series of recurring payments (a mobile phone subscription), an agreement to take payment for a purchase by a series of installments (hire purchase), an agreement to make additional payments when required (account top up), or to fulfil a standard industry practice (no show penalty charge).

Do not provide this parameter group if you are storing the payment details for subsequent payer-initiated payments only.

See Credential on File, Cardholder, and Merchant Initiated Transactions for details.

agreement.amountVariability
Enumeration
OPTIONAL
Indicates if all the payments within the agreement use the same amount or if the amount differs between the payments.

The field must be provided for recurring payment agreements.

Value must be a member of the following list. The values are case sensitive.

FIXED
All payments in the recurring payment agreement have the same amount. Examples include magazine subscriptions or gym memberships.

VARIABLE
The amount for the payments within the recurring payment agreement differs between payments. Examples include usage-based charges like utility or phone bills.

agreement.customData
String
OPTIONAL
Additional information requested for the agreement which cannot be passed using other available data fields.

This field must not contain sensitive data.

Data can consist of any characters, but sensitive data will be rejected

Min length: 1 Max length: 2048
agreement.expiryDate
Date
OPTIONAL
Date at which your agreement with the payer to process payments expires.

Data must comply with ISO 8601 extended date format, yyyy-mm-dd

agreement.id
String
OPTIONAL
Your identifier for the agreement you have with the payer to process payments.

When you collect cards from your payers and store them for later use, you must provide an agreement ID when you use the stored values for:

Recurring payments: you have an agreement with the payer that authorizes you to automatically debit their account at agreed intervals for fixed or variable amounts. For example, gym membership, phone bills, or magazine subscriptions.
Installment payments: you have an agreement with the payer that authorizes you to process multiple payments over an agreed period of time for a single purchase. For example, the payer purchases an item for $1000 and pays for it in four monthly installments.
Unscheduled: you have an agreement with the payer that authorizes you to process future payments when required. For example, the payer authorizes you to process an account top-up transaction for a transit card when the account balance drops below a certain threshold.
Industry Practice: you have an agreement with the payer that authorizes you to initiate additional transactions to fulfil a standard business practice related to an original payment initiated by the payer. For example, a delayed charge for use of the hotel mini bar after the payer has checked out or a no show penalty charge when the payer fails to show for a booking.
When you first establish an agreement with the payer you should also specify the type of agreement in agreement.type.

Data can consist of any characters

Min length: 1 Max length: 100
agreement.maximumAmountPerPayment
Decimal
OPTIONAL
The maximum amount for a single payment in the series as agreed with the payer under your agreement with them.

The amount must be provided in the currency of the order.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
agreement.minimumAmountPerPayment
Decimal
OPTIONAL
The minimum amount for a single payment in the series as agreed with the payer under your agreement with them.

The amount must be provided in the currency of the order.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
agreement.minimumDaysBetweenPayments
Integer
OPTIONAL
The minimum number of days between payments agreed with the payer under your agreement with them.

JSON number data type, restricted to being positive or zero. In addition, the represented number may have no fractional part.

Min value: 1 Max value: 9999
agreement.numberOfPayments
Integer
OPTIONAL
The number of merchant-initiated payments within the recurring payment agreement.

JSON number data type, restricted to being positive or zero. In addition, the represented number may have no fractional part.

Min value: 1 Max value: 999
agreement.paymentFrequency
Enumeration
OPTIONAL
The frequency of the payments within the series as agreed with the payer under your agreement with them.

Value must be a member of the following list. The values are case sensitive.

AD_HOC
The agreement if for payments on an ah-hoc basis.

DAILY
The agreement if for a daily payment.

FORTNIGHTLY
The agreement if for a fortnightly payment.

MONTHLY
The agreement if for a monthly payment.

OTHER
The agreement is for payments according to a schedule other than the ones listed in the other enumeration values for this field.

QUARTERLY
The agreement if for a quarterly payment.

TWICE_YEARLY
The agreement if for a payment twice a year.

WEEKLY
The agreement if for a weekly payment.

YEARLY
The agreement if for a yearly payment.

agreement.retailer
OPTIONAL
For an installment agreement where the payer purchased goods and/or services from a retailer but entered an installment agreement to pay for this purchase with you, you must provide details about the retailer.

agreement.retailer.abbreviatedTradingName
String
OPTIONAL
Provide an abbreviation of the retailer's trading name that can be used by the issuer to indicate the retailer on the payer's statement.

Data can consist of any characters

Min length: 1 Max length: 10
agreement.retailer.merchantCategoryCode
String
OPTIONAL
A 4-digit code used to classify the retailer's business by the type of goods or services it offers.

Data can consist of any characters

Min length: 1 Max length: 4
agreement.retailer.tradingName
String
OPTIONAL
The retailer's trading name.

Data can consist of any characters

Min length: 1 Max length: 100
agreement.startDate
Date
OPTIONAL
This is the effective start date for the payment agreement.

Cannot be in the past.

Data must comply with ISO 8601 extended date format, yyyy-mm-dd

agreement.type
Enumeration
OPTIONAL
The type of commercial agreement that the payer has with you.

Specify the agreement type when you have provided a value for agreement.id and this payment is the first in a series of payments. The default value is OTHER.

The gateway will use the value you specify for subsequent payments in the series.

Value must be a member of the following list. The values are case sensitive.

INSTALLMENT
An agreement where the payer authorizes the payment for a single purchase to be split into a number of payments processed at agreed intervals. For example, pay for a purchase in six monthly installments.

OTHER
An agreement where you want to link related payments for any purpose other than processing recurring, installment, or unscheduled payments. For example, split tender payments.

RECURRING
An agreement where the payer authorizes you to process repeat payments for bills or invoices at agreed intervals (for example, weekly, monthly). The amount might be fixed or variable.

UNSCHEDULED
An agreement where the payer authorizes you to automatically deduct funds for a payment for an agreed purchase when required (unscheduled). For example, auto top-ups when the account value falls below a threshold.

airline
OPTIONAL
Airline industry specific data

airline.bookingReference
Alphanumeric
OPTIONAL
The record locator used to access a specific Passenger Name Record (PNR).

PNR is a record in the database of a booking system that contains the itinerary for a passenger, or a group of passengers traveling together.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 6 Max length: 15
airline.documentType
Enumeration
OPTIONAL
The type of charge associated with the transaction.

Document Type Code

Value must be a member of the following list. The values are case sensitive.

ADDITIONAL_COLLECTION
Additional Collection

AGENCY_EXCHANGE_ORDER
Agency Exchange Order

AGENCY_GROUP_TICKET
Agency Group Ticket

AGENCY_MISCELLANEOUS_CHARGE_ORDER
Agency Misc. Charge Order (MCO)

AGENCY_PASSENGER_TICKET
Agency Passenger Ticket

AGENCY_TOUR_ORDER_OR_VOUCHER
Agency Tour Order/Voucher

AIR_FREIGHT
SPD/Air Freight

ANIMAL_TRANSPORTATION_CHARGE
Animal Transportation Charge

CATALOGUE_MERCHANDISE_ORDERED
Catalogue Merchandise Ordered

CLUB_MEMBERSHIP_FEE
Club Membership Fee

COUPON_BOOK
Coupon Book

CREDIT_CLASS_SERVICE_ADJUSTMENT
Credit Class of Service Adjustment

CREDIT_DENIED_BOARDING
Credit Denied Boarding

CREDIT_EXCHANGE_REFUND
Credit Exchange Refund

CREDIT_LOST_TICKET_REFUND
Credit Lost Ticket Refund

CREDIT_MISCELLANEOUS_REFUND
Credit Misc. Refund

CREDIT_MULTIPLE_UNUSED_TICKETS
Credit Multiple Unused Tickets

CREDIT_OVERCHARGE_ADJUSTMENT
Credit Overcharge Adjustment

CREDIT_UNUSED_TRANSPORTATION
Credit Unused Transportation

DEBT_ADJUSTMENT_DUPLICATE_REFUND_OR_USE
Debt Adjustment Duplicate Refund/Use

DUTY_FREE_SALE
Duty Free Sale

EXCESS_BAGGAGE
Excess Baggage

EXCHANGE_ADJUSTMENT
Exchange Adjustment

EXCHANGE_ORDER
Exchange Order

FIREARMS_CASE
Firearms Case

FREQUENT_FLYER_FEE_OR_PURCHASE
Frequent Flyer Fee/Purchase

FREQUENT_FLYER_FULFILLMENT
Frequent Flyer Fulfillment

FREQUENT_FLYER_OVERNIGHT_DELIVERY_CHARGE
Frequent Flyer Overnight Delivery Charge

GROUP_TICKET
Group Ticket

IN_FLIGHT_ADJUSTMENT
In-flight Adjustment

IN_FLIGHT_CHARGES
In-flight Charges

IN_FLIGHT_DUTY_FREE_PURCHASE
In-flight Duty Free Purchase

IN_FLIGHT_MERCHANDISE_ORDERED
In-flight Merchandise Ordered

IN_FLIGHT_PHONE_CHARGES
In-flight Phone Charges

KENNEL_CHARGE
Kennel Charge

LOST_TICKET_APPLICATION
Lost Ticket Application

MISCELLANEOUS_CHARGE_ORDER_OR_PREPAID_TICKET_ADVICE
Misc. Charge Order (MCO) / Prepaid Ticket Auth.

MISCELLANEOUS_TAXES_FEES
Miscellaneous Tax(es) Fee(s)

PASSENGER_TICKET
Passenger Ticket

SELF_SERVICE_TICKETS
Self-Service Ticket(s)

SENIOR_CITIZEN_DISCOUNT_BOOKLETS
Senior Citizen Discount Booklets

SMALL_PACKAGE_DELIVERY
Small Package Delivery

SPECIAL_SERVICE_TICKET
Special Service Ticket

SUPPORTED_REFUND
Supported Refund

TICKET_BY_MAIL
Ticket by Mail

TOUR_DEPOSIT
Tour Deposit

TOUR_ORDER_VOUCHER
Tour Order Voucher

UNDERCHARGE_ADJUSTMENT
Undercharge Adjustment

UNSUPPORTED_REFUND
Unsupported Refund

UPGRADE_CHARGE
Upgrade Charge

VENDOR_REFUND_CREDIT
Vendor Refund Credit

VENDOR_SALE
Vendor Sale

airline.itinerary
OPTIONAL
Itinerary details

airline.itinerary.leg[n]
OPTIONAL
Travel leg details.

airline.itinerary.leg[n].carrierCode
Regex
OPTIONAL
The 2-character IATA airline code or 3 digit accounting code or both of the airline carrier for the trip leg.

Data must match regex

regex \w{2}|\d{3}|\w{2}/\d{3} message Carrier code must be 2 characters, 3 digits or a combination of both in the format: ZZ/999
airline.itinerary.leg[n].conjunctionTicketNumber
Alphanumeric
OPTIONAL
The ticket containing the coupon for this leg for an itinerary with more than four trip legs.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 11 Max length: 16
airline.itinerary.leg[n].couponNumber
Alphanumeric
OPTIONAL
The coupon number on the ticket for the trip leg.

Each trip leg requires a separate coupon. The coupon within the series is identified by the coupon number.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 1 Max length: 1
airline.itinerary.leg[n].departureAirport
Upper case alphabetic text
OPTIONAL
The 3 character IATA airport code of the departure airport for the trip leg.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
airline.itinerary.leg[n].departureDate
Date
OPTIONAL
Date of departure for the trip leg.

Data must comply with ISO 8601 extended date format, yyyy-mm-dd

airline.itinerary.leg[n].departureTax
Decimal
OPTIONAL
Tax payable on departure for the trip leg.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
airline.itinerary.leg[n].departureTime
Time
OPTIONAL
Departure time in local time for the departure airport for this trip leg.

Data must comply with ISO 8601 extended time formats, hh:mm[:ss]Z or hh:mm[:ss](+/-)hh[:mm]

airline.itinerary.leg[n].destinationAirport
Upper case alphabetic text
OPTIONAL
The 3 character IATA airport code for the destination airport for the trip leg.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
airline.itinerary.leg[n].destinationArrivalDate
Date
OPTIONAL
Arrival date in local time for the destination airport for this trip leg.

Data must comply with ISO 8601 extended date format, yyyy-mm-dd

airline.itinerary.leg[n].destinationArrivalTime
Time
OPTIONAL
Arrival time in local time for the destination airport for this trip leg.

Data must comply with ISO 8601 extended time formats, hh:mm[:ss]Z or hh:mm[:ss](+/-)hh[:mm]

airline.itinerary.leg[n].endorsementsRestrictions
Alphanumeric
OPTIONAL
Restrictions (e.g. non-refundable) or endorsements applicable to the trip leg.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 1 Max length: 20
airline.itinerary.leg[n].exchangeTicketNumber
Alphanumeric
OPTIONAL
New ticket number issued when a ticket is exchanged for the trip leg.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 11 Max length: 16
airline.itinerary.leg[n].fare
Decimal
OPTIONAL
Total fare payable for the trip leg.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
airline.itinerary.leg[n].fareBasis
Alphanumeric + additional characters
OPTIONAL
Code defining the rules forming the basis of the fare (type of fare, class entitlement, etc.)

Data may consist of the characters 0-9, a-z, A-Z, ' '

Min length: 1 Max length: 24
airline.itinerary.leg[n].fees
Decimal
OPTIONAL
Total fees payable for the trip leg.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
airline.itinerary.leg[n].flightNumber
Alphanumeric
OPTIONAL
The flight number for the trip leg.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 4 Max length: 6
airline.itinerary.leg[n].stopoverPermitted
Boolean
OPTIONAL
Indicates if a stopover is permitted for the trip leg.

JSON boolean values 'true' or 'false'.

airline.itinerary.leg[n].taxes
Decimal
OPTIONAL
Total taxes payable for the trip leg.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
airline.itinerary.leg[n].travelClass
Alphanumeric
OPTIONAL
The industry code indicating the class of service (e.g. Business, Coach) for the leg.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 1 Max length: 3
airline.itinerary.numberInParty
Digits
OPTIONAL
Number of passengers associated with this booking.

Data is a string that consists of the characters 0-9.

Min length: 1 Max length: 3
airline.itinerary.originCountry
Upper case alphabetic text
OPTIONAL
The 3 character ISO 3166-1 alpha-3 country code of the country of origin for the itinerary.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
airline.passenger[n]
OPTIONAL
Passenger details

airline.passenger[n].firstName
String
OPTIONAL
First name of the passenger to whom the ticket is being issued.

Data can consist of any characters

Min length: 1 Max length: 50
airline.passenger[n].frequentFlyerNumber
String
OPTIONAL
Frequent Flyer or Loyalty Program number for this passenger.

Data can consist of any characters

Min length: 1 Max length: 20
airline.passenger[n].lastName
String
OPTIONAL
Last name of the passenger to whom the ticket is being issued.

Data can consist of any characters

Min length: 1 Max length: 20
airline.passenger[n].middleName
String
OPTIONAL
Middle name of the passenger to whom the ticket is being issued.

Data can consist of any characters

Min length: 1 Max length: 50
airline.passenger[n].specificInformation
Alphanumeric
OPTIONAL
Passenger specific information recorded on the ticket.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 1 Max length: 59
airline.passenger[n].title
String
OPTIONAL
Title of the passenger to whom the ticket is being issued.

Data can consist of any characters

Min length: 1 Max length: 20
airline.planNumber
Alphanumeric
OPTIONAL
Plan number supplied by the airline for this booking.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 2 Max length: 2
airline.ticket
OPTIONAL
Ticket details

airline.ticket.conjunctionTicketIndicator
Boolean
OPTIONAL
Indicates if a conjunction ticket with additional coupons was issued.

Conjunction ticket refers to two or more tickets concurrently issued to a passenger and which together constitute a single contract of carriage.

JSON boolean values 'true' or 'false'.

airline.ticket.eTicket
Boolean
OPTIONAL
Indicates if an electronic ticket was issued.

JSON boolean values 'true' or 'false'.

airline.ticket.exchangedTicketNumber
Alphanumeric
OPTIONAL
The original ticket number when this is a transaction for an exchanged ticket.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 11 Max length: 16
airline.ticket.issue
OPTIONAL
Ticket issue information.

airline.ticket.issue.address
String
OPTIONAL
The address where the ticket was issued.

Data can consist of any characters

Min length: 1 Max length: 16
airline.ticket.issue.carrierCode
Regex
OPTIONAL
The 2-character IATA airline code or 3 digit accounting code or both of the airline carrier issuing the ticket.

Data must match regex

regex \w{2}|\d{3}|\w{2}/\d{3} message Carrier code must be 2 characters, 3 digits or a combination of both in the format: ZZ/999
airline.ticket.issue.carrierName
Alphanumeric
OPTIONAL
Name of airline carrier issuing the ticket.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 1 Max length: 25
airline.ticket.issue.city
String
OPTIONAL
The city/town where the ticket was issued.

Data can consist of any characters

Min length: 1 Max length: 100
airline.ticket.issue.country
Upper case alphabetic text
OPTIONAL
The 3 character ISO 3166-1 alpha-3 country code of the country where the ticket was issued.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
airline.ticket.issue.date
Date
OPTIONAL
The date the ticket was issued.

Data must comply with ISO 8601 extended date format, yyyy-mm-dd

airline.ticket.issue.travelAgentCode
Alphanumeric
OPTIONAL
Industry code of the travel agent issuing the ticket.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 8 Max length: 9
airline.ticket.issue.travelAgentName
Alphanumeric + additional characters
OPTIONAL
Name of the travel agent issuing the ticket.

Data may consist of the characters 0-9, a-z, A-Z, ' ', '.', ',', '-', ''', '&', '/', '\', '(', ')'

Min length: 1 Max length: 30
airline.ticket.restricted
Boolean
OPTIONAL
Indicates if the issued ticket is refundable.

JSON boolean values 'true' or 'false'.

airline.ticket.taxOrFee[n]
OPTIONAL
Breakdown of the ticket taxes, airport taxes, charges and fees for an airline ticket purchase.

The total of the amounts in this group should equal the sum of the airline.ticket.totalFees and airline.ticket.totalTaxes fields.

airline.ticket.taxOrFee[n].amount
Decimal
OPTIONAL
The tax, charge or fee amount payable.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
airline.ticket.taxOrFee[n].type
Alphanumeric
OPTIONAL
The tax, charge or fee type code as assigned by IATA.

For example, the IATA tax/ charge/ fee type for Passenger Movement Charge (PMC) in Australia is TT1.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 3 Max length: 3
airline.ticket.ticketNumber
Alphanumeric
OPTIONAL
The airline ticket number associated with the transaction.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 11 Max length: 16
airline.ticket.totalFare
Decimal
OPTIONAL
Total fare for all trip legs on the ticket.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
airline.ticket.totalFees
Decimal
OPTIONAL
Total fee for all trip legs on the ticket.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
airline.ticket.totalTaxes
Decimal
OPTIONAL
Total taxes for all trip legs on the ticket.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
airline.transactionType
Enumeration
OPTIONAL
The type of transaction performed against this airline booking.

Transaction Type

Value must be a member of the following list. The values are case sensitive.

EXCHANGE_TICKET
Exchange Ticket

MISCELLANEOUS_CHARGE
Miscellaneous Charge

REFUND
Refund

TOUR_ORDER
Tour Order

apiOperation
String
= REFUND
FIXED
Any sequence of zero or more unicode characters.

billing
OPTIONAL
Information on the billing address including the contact details of the payer.

billing.address
OPTIONAL
The payer's billing address.

This data may be used to qualify for better interchange rates on corporate purchase card transactions.

billing.address.city
String
OPTIONAL
The city portion of the address.

Data can consist of any characters

Min length: 1 Max length: 100
billing.address.company
String
OPTIONAL
The name of the company associated with this address.

Data can consist of any characters

Min length: 1 Max length: 100
billing.address.country
Upper case alphabetic text
OPTIONAL
The 3 letter ISO standard alpha country code of the address.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
billing.address.postcodeZip
Alphanumeric + additional characters
OPTIONAL
The post code or zip code of the address.

Data may consist of the characters 0-9, a-z, A-Z, ' ', '-'

Min length: 1 Max length: 10
billing.address.stateProvince
String
OPTIONAL
The state or province of the address.

Data can consist of any characters

Min length: 1 Max length: 20
billing.address.stateProvinceCode
String
OPTIONAL
The three character ISO 3166-2 country subdivision code for the state or province of the address.

Providing this field might improve your payer experience for 3-D Secure payer authentication.

Data can consist of any characters

Min length: 1 Max length: 3
billing.address.street
String
OPTIONAL
The first line of the address.

For example, this may be the street name and number, or the Post Office Box details.

Note: The transaction response will contain a concatenation of street and street2 data. If the concatenated value is more than the maximum field length, street2 data will be truncated.

Data can consist of any characters

Min length: 1 Max length: 100
billing.address.street2
String
OPTIONAL
The second line of the address (if provided).

Note: This field will be empty in the transaction response, as street2 data will be concatenated into the street field.

Data can consist of any characters

Min length: 1 Max length: 100
correlationId
String
OPTIONAL
A transient identifier for the request, that can be used to match the response to the request.

The value provided is not validated, does not persist in the gateway, and is returned as provided in the response to the request.

Data can consist of any characters

Min length: 1 Max length: 100
cruise
OPTIONAL
Cruise industry data.

cruise.bookingReference
String
OPTIONAL
The cruise booking reference.

Data can consist of any characters

Min length: 1 Max length: 12
cruise.company
OPTIONAL
Information about the cruise line.

cruise.company.contact
OPTIONAL
Contact details of the cruise line.

cruise.company.contact.companyPhone
Telephone Number
OPTIONAL
The cruise line registered office telephone number in ITU-T E123 format.

Data consists of '+', country code (1, 2 or 3 digits), 'space', and national number (which may embed single space characters for readability)

Mandatory country code: true Max total digits: 15
cruise.company.contact.customerServicePhone
Telephone Number
OPTIONAL
The customer service phone number in ITU-T E123 format.

Data consists of '+', country code (1, 2 or 3 digits), 'space', and national number (which may embed single space characters for readability)

Mandatory country code: true Max total digits: 15
cruise.departureDate
Date
OPTIONAL
The cruise departure/ sail date.

This field is required when cruise industry data is provided.

The value entered must be equal to or earlier than cruise.returnDate.

Data must comply with ISO 8601 extended date format, yyyy-mm-dd

cruise.departurePort
OPTIONAL
A departurePort is the port where the passenger(s) boarded the cruise ship when the cruise trip started

cruise.departurePort.address
OPTIONAL
Address of the cruise line.

cruise.departurePort.address.city
String
OPTIONAL
The city portion of the address.

Data can consist of any characters

Min length: 1 Max length: 100
cruise.departurePort.address.country
Upper case alphabetic text
OPTIONAL
The 3 letter ISO standard alpha country code of the address.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
cruise.departurePort.address.postCodeZip
Alphanumeric + additional characters
OPTIONAL
The post code or zip code of the address.

Data may consist of the characters 0-9, a-z, A-Z, ' ', '-'

Min length: 1 Max length: 10
cruise.departurePort.address.stateProvinceCode
String
OPTIONAL
The state or province code of the address.

The value must match the second part of the ISO 3166-2 code. For an address in the United States provide the 2-letter ISO 3166-2 state code. For US military bases provide one of AE, AA, AP. For an address in Canada provide the 2-letter ISO 3166-2 province code.

Data can consist of any characters

Min length: 1 Max length: 3
cruise.departurePort.address.street
String
OPTIONAL
The first line of the address.

Data can consist of any characters

Min length: 1 Max length: 100
cruise.departurePort.address.street2
String
OPTIONAL
The second line of the address (if provided).

Data can consist of any characters

Min length: 1 Max length: 100
cruise.passenger[n]
OPTIONAL
Cruise passenger details.

cruise.passenger[n].firstName
String
OPTIONAL
The first name of the passenger.

Data can consist of any characters

Min length: 1 Max length: 50
cruise.passenger[n].folioNumber
String
OPTIONAL
The folio number assigned to the passenger.

Data can consist of any characters

Min length: 1 Max length: 30
cruise.passenger[n].lastName
String
OPTIONAL
The last name of the passenger.

Data can consist of any characters

Min length: 1 Max length: 50
cruise.passenger[n].middleName
String
OPTIONAL
The middle name of the passenger.

Data can consist of any characters

Min length: 1 Max length: 50
cruise.passenger[n].title
String
OPTIONAL
The title of the passenger.

Data can consist of any characters

Min length: 1 Max length: 50
cruise.returnDate
Date
OPTIONAL
The cruise return/ sail end date.

This field is required when cruise.departureDate is provided and the value must be equal to or later than cruise.departureDate.

Data must comply with ISO 8601 extended date format, yyyy-mm-dd

cruise.shipName
String
OPTIONAL
The name of the cruise ship.

Data can consist of any characters

Min length: 1 Max length: 50
cruise.travelAgentCode
Alphanumeric
OPTIONAL
The industry code of the travel agent booking the cruise.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 8 Max length: 9
cruise.travelAgentName
String
OPTIONAL
The name of the travel agent booking the cruise.

Data can consist of any characters

Min length: 1 Max length: 30
cruise.travelPackageItems
Comma separated enumeration
OPTIONAL
A comma separated list of the travel items that are included as part of a cruise travel package.

If the value CRUISE_ONLY is provided then other items are not permitted in the list.

Value must be one or more comma separated members of the following list. The values are case sensitive.

CAR_RENTAL
Car rental is included in the travel package.

CRUISE_ONLY
No additional items are included in the cruise travel package.

FLIGHT
Flights are included in the travel package.

currencyConversion
OPTIONAL
Information specific to the use of dynamic currency conversion (DCC).

If you requested a rate quote via the gateway, provide the requestId as returned in the PAYMENT_OPTIONS_INQUIRY response. For rate quote requests performed outside the gateway, you must at least provide payer amount, payer currency, provider and payer exchange rate.

You can only provide DCC information on the initial transaction for an order.

If the initial transaction for an order is a payer authentication transaction with DCC information and the subsequent authorization or pay transaction contains different DCC information, that authorization or pay transaction will be rejected.

If DCC information is provided on subsequent capture or refund for an order, it will be ignored.

currencyConversion.exchangeRateTime
DateTime
OPTIONAL
The timestamp of when the conversion rate is effective.

The timestamp may need to be displayed to the payer on the merchant site to satisfy regulatory requirements.

An instant in time expressed in ISO8601 date + time format - "YYYY-MM-DDThh:mm:ss.SSSZ"

currencyConversion.marginPercentage
Decimal
OPTIONAL
The foreign exchange markup applied as a percentage to the transaction amount for providing the conversion service.

The margin percentage may need to be displayed to the payer on the merchant site to satisfy regulatory requirements.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 8
currencyConversion.payerAmount
Decimal
OPTIONAL
The total amount of the transaction in the payer's currency.

You must include this field if the payer accepted the DCC offer you presented to them.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
currencyConversion.payerCurrency
Upper case alphabetic text
OPTIONAL
The currency of the DCC rate quote provided by your DCC Service Provider.

The currency must be expressed as an ISO 4217 alpha code, e.g. USD and must be different to that provided for transaction currency. You must include this field if the payer accepted the DCC offer you presented to them.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
currencyConversion.payerExchangeRate
Decimal
OPTIONAL
The exchange rate used to convert the transaction amount into the payer's currency.

The payer exchange rate includes the foreign exchange markup (marginPercentage). The payer exchange rate is displayed to the payer on the merchant site.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 19
currencyConversion.provider
Enumeration
OPTIONAL
This identifies the name of the provider of the DCC quote.

Value must be a member of the following list. The values are case sensitive.

FEXCO
currencyConversion.providerReceipt
String
OPTIONAL
The quote provider's unique reference to the rate quote.

Data can consist of any characters

Min length: 1 Max length: 100
currencyConversion.requestId
String
OPTIONAL
The unique identifier for your DCC quote request as returned in the PAYMENT_OPTIONS_INQUIRY response.

Data can consist of any characters

Min length: 1 Max length: 100
currencyConversion.uptake
Enumeration
OPTIONAL
Indicates how DCC applies to the order.

If not provided, this value defaults to NOT_REQUIRED.

Value must be a member of the following list. The values are case sensitive.

ACCEPTED
The payer accepted the DCC offer and pays in their own currency. The conditions of the rate quote are applied in the processing of this transaction.

DECLINED
The payer declined the DCC offer and pays in your transaction currency.

NOT_AVAILABLE
A rate quote was requested, but no DCC offer was provided. For rate quotes via the gateway the PAYMENT_OPTION_INQUIRY response contains a currencyConversion.gatewayCode other than QUOTE_PROVIDED.

NOT_REQUIRED
DCC is not required for this transaction.

customer
OPTIONAL
Information about the customer, including their contact details.

customer.email
Email
OPTIONAL
The email address of the customer.

The field format restriction ensures that the email address is longer than 3 characters and adheres to a generous subset of valid RFC 2822 email addresses.

Ensures that the email address is longer than 3 characters and adheres to a generous subset of valid RFC 2822 email addresses

customer.firstName
String
OPTIONAL
The payer's first name.

Data can consist of any characters

Min length: 1 Max length: 50
customer.identification
OPTIONAL
Identification of the payer.

This information is used to identify the sender in account funding transactions.

customer.identification.country
Upper case alphabetic text
OPTIONAL
The ISO 3166 three-letter country code of the issuer of the identification.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
customer.identification.type
Enumeration
OPTIONAL
The type of identification provided for the customer.

Value must be a member of the following list. The values are case sensitive.

ALIEN_REGISTRATION_NUMBER
The customer's identification type is an alien registration number issued by U.S. Citizenship and Immigration Services (USCIS) to immigrants who apply to live in the United States.

BUSINESS_TAX_ID
The customer's identification type is a business tax id which is assigned to the business entity by the tax department.

COMPANY_REGISTRATION_NUMBER
The customer's identification type is company registration number which is issued to the company at the time of its incorporation.

CUSTOMER_IDENTIFICATION
The customer's identification type is an unspecified form of customer identification through which the customer can identified and verified.

DATE_OF_BIRTH
The customer's identification type is date of birth.

DRIVERS_LICENSE
The customer's identification type is a driving license.

EMAIL
The customer's identification type is an email address.

GOVERNMENT_ISSUED
The customer's identification type is government issued.

INDIVIDUAL_TAX_ID
The customer's identification type is an individual tax id which is assigned to the individual by the tax department.

LAW_ENFORCEMENT_IDENTIFICATION
The customer's identification type is a law enforcement identification.

MILITARY_IDENTIFICATION
The customer's identification type is a military identification which is issued by the department of defense and military.

NATIONAL_IDENTIFICATION_CARD
The customer's identification type is a national identification card issued by the customer's country.

OTHER
The customer's identification type cannot be classified using any of the other categories. This is the default value.

PASSPORT
The customer's identification type is a passport.

PHONE_NUMBER
The customer's identification type is a phone number.

PROXY_IDENTIFICATION
The customer's identification type is proxy identification.

SOCIAL_SECURITY_NUMBER
The customer's identification type is a social security number issued by Social Security Administration (SSA) to U.S. citizens, permanent residents and eligible non-immigrant workers in United States.

TRAVEL_IDENTIFICATION
The customer's identification type is travel document other than a passport.

customer.identification.value
String
OPTIONAL
The identification value/number for the type of identification provided in customer.identification.type.

Data can consist of any characters

Min length: 1 Max length: 50
customer.lastName
String
OPTIONAL
The payer's last or surname.

Data can consist of any characters

Min length: 1 Max length: 50
customer.middleName
String
OPTIONAL
The payer's middle name.

Data can consist of any characters

Min length: 1 Max length: 50
customer.mobilePhone
Telephone Number
OPTIONAL
The payer's mobile phone or cell phone number in ITU-T E123 format, for example +1 607 1234 5678

The number consists of:
'+'
country code (1, 2 or 3 digits)
'space'
national number ( which may embed single spaces characters for readability).

Data consists of '+', country code (1, 2 or 3 digits), 'space', and national number (which may embed single space characters for readability)

Mandatory country code: true Max total digits: 15
customer.phone
Telephone Number
OPTIONAL
The payer's phone number in ITU-T E123 format, for example +1 607 1234 456

The number consists of:
'+'
country code (1, 2 or 3 digits)
'space'
national number ( which may embed single spaces characters for readability).

Data consists of '+', country code (1, 2 or 3 digits), 'space', and national number (which may embed single space characters for readability)

Mandatory country code: true Max total digits: 15
customer.taxRegistrationId
String
OPTIONAL
The tax registration identifier of the customer.

Data can consist of any characters

Min length: 1 Max length: 30
device
OPTIONAL
Information about the device used by the payer for this transaction.

device.ani
String
OPTIONAL
The telephone number captured by ANI (Automatic Number Identification) when the customer calls to place the order.

Data can consist of any characters

Min length: 1 Max length: 10
device.aniCallType
String
OPTIONAL
The 2 digit ANI information identifier provided by the telephone company to indicate the call type, for example, cellular (61-63), toll free (24,25), etc.

Data can consist of any characters

Min length: 1 Max length: 2
device.browser
String
OPTIONAL
The User-Agent header of the browser the customer used to place the order.

For example, MOZILLA/4.0 (COMPATIBLE; MSIE 5.0; WINDOWS 95)

Data can consist of any characters

Min length: 1 Max length: 2048
device.fingerprint
String
OPTIONAL
Information collected about a remote computing device for the purpose of providing a unique identifier for the device.

For example, session ID, blackbox ID.

Data can consist of any characters

Min length: 1 Max length: 4000
device.hostname
String
OPTIONAL
The name of the server to which the customer is connected.

Data can consist of any characters

Min length: 1 Max length: 60
device.ipAddress
String
OPTIONAL
The IP address of the device used by the payer, in IPv4 nnn.nnn.nnn.nnn format.

You can provide the IP address in IPv6 format as defined in RFC4291.
IPv6 address will only be used in EMV 3DS authentication. Supplied IPv6 address will not be used for any other purposes.

Data can consist of any characters

Min length: 7 Max length: 45
initiator.userId
String
OPTIONAL
The person who initiated this transaction.

This field is automatically populated by the gateway if the transaction was created via Merchant Administration (gatewayEntryPoint=MERCHANT_ADMINISTRATION) or Merchant Manager (MERCHANT_MANAGER). In this case this is the name that the person used to log in to Merchant Administration or Merchant Manager respectively.

Data can consist of any characters

Min length: 1 Max length: 256
order
OPTIONAL
Information about the order associated with this transaction.

order.acceptPartialAmount
Boolean
OPTIONAL
Indicates whether you will accept a payment less than order.amount, e.g. when using a gift card.

If not set or set to FALSE, and the full amount is not available, the transaction will be rejected.
Unless you have been advised by your payment service provider that the gateway supports partial approvals for your acquirer, you can ignore this field.
If the gateway supports partial approvals for your acquirer you must set this field to TRUE else the transaction is rejected by the gateway.

JSON boolean values 'true' or 'false'.

order.custom
String
OPTIONAL
Information about this order that is of interest to you.

For example order.custom.X, where 'X' is defined by you and must be less than 100 characters from the set A-Z, a-z, 0-9. For example, order.custom.salesRegion. You can specify up to 50 such fields. They are not sent to acquirers.

Data can consist of any characters

Min length: 1 Max length: 250
order.customerNote
String
OPTIONAL
A note from the payer about this order.

Data can consist of any characters

Min length: 1 Max length: 250
order.description
String
OPTIONAL
Short textual description of the contents of the order.

Data can consist of any characters

Min length: 1 Max length: 127
order.industryPracticePaymentReason
Enumeration
OPTIONAL
This field is used to classify merchant initiated payments which are submitted in the context of certain industry practices.

Use this field to indicate the reason for that industry practice payment.

A merchant initiated industry practice transaction must also contain the 'scheme transaction Id' from the associated cardholder initiated transaction.

You can provide the referenceOrderId of the relevant cardholder initiated transaction and the gateway will include the 'scheme transaction Id' on the industry practice transaction. For example, when you submit a 'delayed charge', you should provide the referenceOrderId of the cardholder-initiated transaction that resulted in the delayed charge.

Alternatively, you can provide the 'scheme transaction Id' of the cardholder initiated transaction in the industry practice transaction using the field transaction.acquirer.traceId.

You must have obtained the payer's consent prior to submitting industry practice transactions.

Value must be a member of the following list. The values are case sensitive.

DELAYED_CHARGE
An additional payment processed in accordance with your terms and conditions after the original payment has been processed. For example, hotel mini bar charge after the payer has checked out or damage to a rental car.

NO_SHOW_PENALTY
A penalty charged in accordance with your charge cancellation policy the payer cancels or fails to show for the booking.

PARTIAL_SHIPMENT
A shipment where merchant decides to ship the goods from the same order in multiple shipments due to various reasons like goods availability, involvement of multiple suppliers for goods etc.

order.marketplace
OPTIONAL
Use this parameter group to provide additional information if you are a marketplace.You are considered a marketplace if you operate an electronic commerce website or mobile application that brings together payers and retailers and you are selling the goods or services on behalf of the retailer.In this case, the card schemes may require you to register with them as a marketplace and assign you a Marketplace ID.

You should provide this identifier to your payment service provider so that the gateway can automatically include it in all transaction messages to your acquirer.

order.marketplace.retailerLocation
Enumeration
OPTIONAL
Provide information about the location of the retailers for goods or services included in this order.Where a retailer is located in a country different from your country, they are considered a foreign retailer, otherwise they are considered a domestic retailer.

Value must be a member of the following list. The values are case sensitive.

DOMESTIC_ONLY
The order only contains items from domestic retailers.

FOREIGN_AND_DOMESTIC
The order contains items from both foreign and domestic retailers.

FOREIGN_ONLY
The order only contains items from foreign retailers.

order.merchantCategoryCode
Digits
OPTIONAL
A 4-digit code used to classify your business by the type of goods or services it offers.This is also known as the Merchant Category Code (MCC).

You only need to provide the MCC if you want to override the default value configured for your acquirer link.The value you provide must match one of those configured by your payment service provider.

Data is a string that consists of the characters 0-9.

Min length: 4 Max length: 4
order.merchantPartnerIdCode
Digits
OPTIONAL
The code that represents a partnership agreement, between you and the issuer.

Data is a string that consists of the characters 0-9.

Min length: 1 Max length: 6
order.notificationUrl
Url
OPTIONAL
The URL to which the gateway will send Webhook notifications when an order is created or updated.

To receive notifications at this URL, you must enable Webhook notifications in Merchant Administration. Ensure the URL is HTTPS

Ensure that this is a valid URL according to RFC 1738.

order.owningEntity
String
OPTIONAL
Your identifier for the part of your organization that is responsible for the order.

You might provide this data when you want to track the accountability for the order. For example, store number, sales region, branch, or profit center

Data can consist of any characters

Min length: 1 Max length: 40
order.purchaseType
Enumeration
OPTIONAL
Indicates the purchase of specific types of goods or services.

If the transaction pulls money from an account for the purpose of crediting another account you must set purchase type to ACCOUNT_FUNDING.


Value must be a member of the following list. The values are case sensitive.

ACCOUNT_FUNDING
The transaction pulls money from an account for the purpose of crediting another account. You may be required to provide additional details about the account funding transaction in the accountFunding parameter group.

order.reference
String
OPTIONAL
An optional identifier for the order.

For example, a shopping cart number, an order number, or an invoice number.

Data can consist of any characters

Min length: 1 Max length: 40
order.statementDescriptor
OPTIONAL
Contact information provided by you for printing on payer's account statements.

order.statementDescriptor.address
OPTIONAL
Descriptor address of the merchant.

order.statementDescriptor.address.city
String
OPTIONAL
The city portion of the address.

Data can consist of any characters

Min length: 1 Max length: 100
order.statementDescriptor.address.company
String
OPTIONAL
The name of the company associated with this address.

Data can consist of any characters

Min length: 1 Max length: 100
order.statementDescriptor.address.country
Upper case alphabetic text
OPTIONAL
The 3 letter ISO standard alpha country code of the address.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
order.statementDescriptor.address.postcodeZip
Alphanumeric + additional characters
OPTIONAL
The post code or zip code of the address.

Data may consist of the characters 0-9, a-z, A-Z, ' ', '-'

Min length: 1 Max length: 10
order.statementDescriptor.address.stateProvince
String
OPTIONAL
The state or province code of the address.

For an address in the United States provide the 2-letter ISO 3166-2 state code. For US military bases provide one of AE, AA, AP.

For an address in Canada provide the 2-letter ISO 3166-2 province code.

Data can consist of any characters

Min length: 1 Max length: 20
order.statementDescriptor.address.street
String
OPTIONAL
The first line of the address.

For example, this may be the street name and number, or the Post Office Box details.

Data can consist of any characters

Min length: 1 Max length: 100
order.statementDescriptor.address.street2
String
OPTIONAL
The second line of the address (if provided).

Data can consist of any characters

Min length: 1 Max length: 100
order.statementDescriptor.name
String
OPTIONAL
Descriptor name of the merchant.

Data can consist of any characters

Min length: 1 Max length: 100
order.statementDescriptor.phone
String
OPTIONAL
Descriptor phone number of the merchant's business.

Data can consist of any characters

Min length: 1 Max length: 20
order.statementDescriptor.websiteUrl
Url
OPTIONAL
The URL of the merchant's descriptor website.

Ensure that this is a valid URL according to RFC 1738.

order.subMerchant
OPTIONAL
Provide these parameters if you are a payment aggregator or facilitator and process payments on behalf of other merchants.

These merchants are referred to as your sub-merchants. The sub-merchant's details you provide may be displayed on the payer's cardholder statement. Note that your acquirer may require you to register with the card scheme(s) before allowing you to submit sub-merchant details with a transaction. This data must be on the initial transaction of an order, subsequent transactions with sub-merchant will be rejected.

order.subMerchant.address
OPTIONAL
The sub-merchant's address.

order.subMerchant.address.city
String
OPTIONAL
The city portion of the address.

Data can consist of any characters

Min length: 1 Max length: 100
order.subMerchant.address.company
String
OPTIONAL
The name of the company associated with this address.

Data can consist of any characters

Min length: 1 Max length: 100
order.subMerchant.address.country
Upper case alphabetic text
OPTIONAL
The 3 letter ISO standard alpha country code of the address.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
order.subMerchant.address.postcodeZip
Alphanumeric + additional characters
OPTIONAL
The post code or zip code of the address.

Data may consist of the characters 0-9, a-z, A-Z, ' ', '-'

Min length: 1 Max length: 10
order.subMerchant.address.stateProvince
String
OPTIONAL
The state or province code of the address.

For an address in the United States provide the 2-letter ISO 3166-2 state code. For US military bases provide one of AE, AA, AP.

For an address in Canada provide the 2-letter ISO 3166-2 province code.

Data can consist of any characters

Min length: 1 Max length: 20
order.subMerchant.address.street
String
OPTIONAL
The first line of the address.

For example, this may be the street name and number, or the Post Office Box details.

Data can consist of any characters

Min length: 1 Max length: 100
order.subMerchant.address.street2
String
OPTIONAL
The second line of the address (if provided).

Data can consist of any characters

Min length: 1 Max length: 100
order.subMerchant.bankIndustryCode
Digits
OPTIONAL
Code used by acquirer to describe the business or industry the sub-merchant operates in.

Data is a string that consists of the characters 0-9.

Min length: 4 Max length: 4
order.subMerchant.disputeContactPhone
Telephone Number
OPTIONAL
Only provide this field if you have received a notification from the scheme that either you or the sub-merchant has a high number of disputes.

In this case, provide a phone number that payers can use to contact the sub-merchant in case of a dispute. Where applicable, the issuer may display this phone number on the cardholder statement. The phone number must be provided in ITU-T E123 format.

Data consists of '+', country code (1, 2 or 3 digits), 'space', and national number (which may embed single space characters for readability)

Mandatory country code: true Max total digits: 15
order.subMerchant.email
Email
OPTIONAL
The sub-merchant's email address.

Ensures that the email address is longer than 3 characters and adheres to a generous subset of valid RFC 2822 email addresses

order.subMerchant.governmentCountryCode
Upper case alphabetic text
OPTIONAL
Only provide this field if the sub merchant is a government owned or controlled merchant.

A sub merchant is considered a government owned or controlled entity (government controlled merchant) if 50% or more of the sub merchant is owned by the government. Provide the ISO 3166 three-letter country code of the government country where this differs from the sub merchant's physical location country.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
order.subMerchant.identifier
String
REQUIRED
Your identifier for the sub-merchant.

You can use this identifier in searches and reports in the gateway.

Data can consist of any characters

Min length: 1 Max length: 100
order.subMerchant.marketplaceId
String
OPTIONAL
If the sub merchant is a marketplace, provide the marketplace ID assigned to them by Visa.

A sub merchant is considered a marketplace if they operate a platform (online commerce website or mobile application) where retailers can sell goods and services.

Data can consist of any characters

Min length: 1 Max length: 11
order.subMerchant.phone
String
OPTIONAL
The sub-merchant's phone number

Data can consist of any characters

Min length: 1 Max length: 20
order.subMerchant.registeredName
String
OPTIONAL
The legal name of the sub-merchant.

Data can consist of any characters

Min length: 1 Max length: 100
order.subMerchant.tradingName
String
REQUIRED
The trading name of the sub merchant, also known as doing business as (DBA), operating as or trading as.

For MasterCard transactions the name must not exceed 21 characters. For American Express transactions the name must not exceed 27 characters (or 36 characters including the aggregator name). The trading name may be displayed on the payer's cardholder statement. Therefore if you need to shorten it, use an abbreviation that will be meaningful to the payer when displayed on their statement.

Data can consist of any characters

Min length: 1 Max length: 100
order.subMerchant.websiteUrl
Url
OPTIONAL
The URL of the sub-merchant's website.

Ensure that this is a valid URL according to RFC 1738.

order.walletIndicator
String
OPTIONAL
The wallet indicator as returned by the wallet provider.

Data can consist of any characters

Min length: 3 Max length: 3
order.walletProvider
Enumeration
OPTIONAL
Details about the source of the payment details used for digital payment methods.

Provide this value when you process payments for:
• Device payment methods such as Apple Pay, Android Pay, Samsung Pay, or Google Pay.
• Digital wallets such as Masterpass, Visa Checkout or Amex Express Checkout.

Value must be a member of the following list. The values are case sensitive.

AMEX_EXPRESS_CHECKOUT
Amex Express Checkout wallet provider.

APPLE_PAY
Apple Pay mobile wallet provider.

CHASE_PAY
Chase Pay wallet provider.

GOOGLE_PAY
Google Pay mobile wallet provider.

MASTERPASS_ONLINE
MasterPass Online wallet provider.

SAMSUNG_PAY
Samsung Pay mobile wallet provider.

SECURE_REMOTE_COMMERCE
Secure Remote Commerce (SRC) wallet provider.

VISA_CHECKOUT
Visa Checkout wallet provider.

partnerSolutionId
String
OPTIONAL
If, when integrating with the gateway, you are using a solution (e.g. a shopping cart or e-commerce solution) provided, supported or certified by your payment service provider, enter the solution ID issued by your payment service provider here.

If your payment service provider has not provided you with a solution ID, you should ignore this field.

Data can consist of any characters

Min length: 1 Max length: 40
paymentPlan
OPTIONAL
Information about the payment plan selected by the cardholder.

Payment Plan is a payment option available to cardholders who wish to repay the purchase amount in a number of monthly installments with or without a deferral period.

paymentPlan.externalPlanId
Alphanumeric
OPTIONAL
A unique identifier for an installment plan chosen by the payer.

This is the plan id provided by an external provider such as VISA.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 1 Max length: 10
posTerminal
OPTIONAL
Information about the device used to initiate the transaction at the Point-of-Sale (POS).

posTerminal.address
OPTIONAL
The address where the POS is located.

For the posTerminal.address.country field - EMV: 9F1A.

posTerminal.address.city
String
OPTIONAL
The city portion of the address.

Data can consist of any characters

Min length: 1 Max length: 100
posTerminal.address.company
String
OPTIONAL
The name of the company associated with this address.

Data can consist of any characters

Min length: 1 Max length: 100
posTerminal.address.country
Upper case alphabetic text
OPTIONAL
The 3 letter ISO standard alpha country code of the address.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
posTerminal.address.postcodeZip
Alphanumeric + additional characters
OPTIONAL
The post code or zip code of the address.

Data may consist of the characters 0-9, a-z, A-Z, ' ', '-'

Min length: 1 Max length: 10
posTerminal.address.stateProvince
String
OPTIONAL
The state or province of the address.

Data can consist of any characters

Min length: 1 Max length: 20
posTerminal.address.street
String
OPTIONAL
The first line of the address.

For example, this may be the street name and number, or the Post Office Box details.

Data can consist of any characters

Min length: 1 Max length: 100
posTerminal.address.street2
String
OPTIONAL
The second line of the address (if provided).

Data can consist of any characters

Min length: 1 Max length: 100
posTerminal.attended
Enumeration
OPTIONAL
Specifies whether the terminal is attended by the merchant.

You only need to provide this field for card present transactions.

You must provide a value for this field for chip transactions with UK acquirers.

This field corresponds to EMV tag 9F35

Value must be a member of the following list. The values are case sensitive.

ATTENDED
Attended terminal.

SEMI_ATTENDED
Where a card or proximity payment device is present; and the cardholder is present; and the cardholder completes the transaction and, if required, an individual representing the merchant or acquirer assist the cardholder to complete the transaction.

UNATTENDED
Unattended terminal.

UNKNOWN_OR_UNSPECIFIED
Unknown or unspecified.

posTerminal.cardPresenceCapability
Enumeration
OPTIONAL
Indicates the capabilities of the terminal to support card present, card not present or both.

Value must be a member of the following list. The values are case sensitive.

CARD_NOT_PRESENT
Card not present.

CARD_PRESENT
Card present.

CARD_PRESENT_AND_CARD_NOT_PRESENT
Card present and card not present.

posTerminal.cardholderActivated
Enumeration
OPTIONAL
Indicates the type of cardholder-activated terminal (CAT) used by the payer for the transaction.

A CAT is typically an unattended terminal. For example a terminal used to purchase transit tickets, a terminal use to pay parking fees, toll fees, or automated dispensing machines.

There are seven types (levels) of CAT devices. Each level has specific card scheme requirements.

If you do not provide a value for this field for a Card Present payment the gateway defaults the value to NOT_CARDHOLDER_ACTIVATED.

This field corresponds to EMV tag 9F35

Value must be a member of the following list. The values are case sensitive.

AUTOMATED_DISPENSING_MACHINE_WITH_PIN
CAT level 1 terminal.

ELECTRONIC_COMMERCE
CAT level 6 terminal.

IN_FLIGHT_COMMERCE
CAT level 4 terminal.

LIMITED_AMOUNT_TERMINAL
CAT level 3 terminal.

MPOS_ACCEPTANCE_DEVICE
CAT level 9 terminal.

NOT_CARDHOLDER_ACTIVATED
Terminal is not activated by the cardholder.

SELF_SERVICE_TERMINAL
CAT level 2 terminal.

TRANSPONDER_TRANSACTION
CAT level 7 terminal.

posTerminal.inputCapability
Enumeration
OPTIONAL
Indicates the type of input the terminal is capable of receiving.

For example, chip, magnetic stripe read, key entry or contactless.

This field corresponds to EMV tag 9F33

Value must be a member of the following list. The values are case sensitive.

BARCODE
The terminal supports data input using a barcode reader.

CHIP
Chip read only.

CHIP_AND_KEY_ENTRY_AND_MAGNETIC_STRIPE
MSR, chip and key entry.

CHIP_AND_KEY_ENTRY_AND_MAGNETIC_STRIPE_AND_RFID
The terminal supports chip read, key entry, magnetic stripe read, and RFID read.

CHIP_AND_MAGNETIC_STRIPE
MSR and chip.

CONTACTLESS_CHIP
Contactless chip.

CONTACTLESS_MAGNETIC_STRIPE
Contactless MSR.

CONTACTLESS_OR_MAGNETIC_STRIPE
The terminal supports both contactless interaction with a chip and magnetic stripe read.

KEY_ENTRY
Key entry only.

KEY_ENTRY_AND_MAGNETIC_STRIPE
MSR and key entry.

MAGNETIC_STRIPE
Magnetic strip read (MSR) only.

UNKNOWN
VOICE_AUDIO_RESPONSE
posTerminal.lane
String
OPTIONAL
The name that you use to uniquely identify the location of the Point Of Sale instance used to initiate the transaction.

Examples could be S43_L12 (Lane 12 in Shop 43) or Kiosk_76. This field can be used for your search or reporting needs, and might be used by fraud management systems.

This field corresponds to EMV tag 9F1C

Data can consist of any characters

Min length: 1 Max length: 8
posTerminal.location
Enumeration
OPTIONAL
Indicates the physical location of the terminal in relation to your business premises.

If you do not provide a value for this field for a mobile wallet payment the gateway defaults the value to PAYER_TERMINAL_OFF_PREMISES.

Value must be a member of the following list. The values are case sensitive.

MERCHANT_TERMINAL_OFF_PREMISES
A terminal under the merchant's control but not on the merchant's premises was used.

MERCHANT_TERMINAL_ON_PREMISES
A terminal under the merchant's control on the merchant's premises was used.

NO_TERMINAL_VOICE_OR_AUDIO_RESPONSE
A voice or an audio response system was used, not a physical terminal.

PAYER_TERMINAL_OFF_PREMISES
A terminal under the payer's control and off the merchant's premises was used. For example, a mobile device or personal computer.

PAYER_TERMINAL_ON_PREMISES
A terminal under the payer's control on the merchant's premises was used. For example, a mobile device or personal computer.

posTerminal.mobile
OPTIONAL
Information about mobile POS (mPOS) device.

posTerminal.mobile.cardInputDevice
Enumeration
OPTIONAL
The card reader used by an mPOS device.

Value must be a member of the following list. The values are case sensitive.

BUILT_IN
Off-the-shelf mobile phone or tablet with only a built-in contactless reader.

INTEGRATED_DONGLE
Dedicated mobile terminal with an integrated card reader.

SEPARATE_DONGLE
Off-the shelf device or dedicated mobile terminal, with a separate card reader.

posTerminal.onlineReasonCode
Enumeration
OPTIONAL
Indicates the reason for sending a transaction online to the acquirer rather than completing it locally at the terminal.

The online reason code is mandatory for chip and chip fallback transactions (including reversals) for all online transactions.

Where more than one reason applies, then the order of priority used for the enumeration list applies.

Value must be a member of the following list. The values are case sensitive.

CHIP_APPLICATION_DATA_FILE_ERROR
The application data file on the chip was unable to process. The terminal has possession of the card. Only used by integrated ICC/MSR terminals (where the terminal has possession of the card and when this condition can be accurately identified).

CHIP_COMMON_DATA_FILE_ERROR
The application common data file on the chip was unable to process. Only used by integrated ICC/MSR terminals (where the terminal has possession of the card and when this condition can be accurately identified).

FORCED_BY_CHIP
The chip application forced the the transaction to go online.

FORCED_BY_ISSUER
Issuer rules forced the transaction to go online. For example, the card is expired.

FORCED_BY_MERCHANT
Rules in the merchant's POS application forced the transaction to go online. For example, the card was used twice or send one in a certain number of authorizations online.

FORCED_BY_TERMINAL
The terminal forced the transaction to go online. For example, the results of tests the terminal carried out during the EMV process indicated to send the transaction online.

MERCHANT_SUSPICIOUS
The merchant has indicated a suspicious transaction. For example, they indicated an unsuccessful signature check or the card returned an inappropriate cryptogram.

OVER_FLOOR_LIMIT
The transaction amount is above the limit set for local processing of the transaction at the terminal.

RANDOM_SELECTION_BY_TERMINAL
The terminal has randomly selected the transaction for online processing.

UNABLE_TO_PROCESS_CHIP
The terminal is not able to process a chip transaction. The transaction was sent online as a fallback.

posTerminal.panEntryMode
Enumeration
OPTIONAL
Indicates how you or the Payer entered the Primary Account Number (PAN) of the card at the terminal.

This field corresponds to EMV tag 9F39

Value must be a member of the following list. The values are case sensitive.

BARCODE_READER
The PAN was entered via a barcode reader.

CHIP
The PAN was entered by reading data from the chip on the card.

CHIP_FALLBACK
A chip-capable terminal failed to process the transaction using data on the card's chip. Therefore, the PAN was read using a fallback mode.

CONTACTLESS
The PAN was entered by a contactless interaction with a chip.

ECOMMERCE
The PAN was entered via an electronic commerce interaction, including chip.

KEYED
The PAN was manually entered.

MOBILE_COMMERCE
OPTICAL_CHARACTER_READER
The PAN was entered via an an optical character reader.

RFID_CHIP
An RFID device was used. Chip data is provided.

RFID_STRIPE
An RFID device was used. Stripe data is provided.

SWIPE
The PAN was read from the magnetic stripe, and the full, unaltered contents of the stripe are provided.

SWIPE_WITH_SIGNATURE
The PAN was read from the magnetic stripe and a signature was provided.

UNKNOWN
The mode of PAN entry is unknown.

VOICE_AUTHORIZATION
VOICE_RESPONSE
The PAN was collected using a Voice Response Unit.

posTerminal.pinEntryCapability
Enumeration
OPTIONAL
Indicates the capability of the terminal to accept entry of the Payer's PIN.

This field corresponds to EMV tag 9F33

Value must be a member of the following list. The values are case sensitive.

OFFLINE_PIN_ONLY
Only offline PIN is supported.

PIN_NOT_SUPPORTED
Neither offline nor online PIN is supported.

PIN_PAD_INOPERATIVE
PIN is supported but the POS or Payment Client has determined that it is not operational.

PIN_SUPPORTED
Both offline & online PIN supported.

SOFTWARE_ONLINE_PIN_ONLY
mPOS Software-based PIN Entry Capability (online PIN supported).

UNKNOWN
The PIN entry capability is not known.

posTerminal.pinLengthCapability
Integer
OPTIONAL
The maximum number of PIN characters that can be entered at the terminal

JSON number data type, restricted to being positive or zero. In addition, the represented number may have no fractional part.

Min value: 4 Max value: 12
posTerminal.serialNumber
ASCII Text
OPTIONAL
The unique serial number assigned by the manufacturer to the terminal device.

Data consists of ASCII characters

Min length: 1 Max length: 16
posTerminal.store
OPTIONAL
Information about the store or business location.

posTerminal.store.id
String
OPTIONAL
Your unique identifier for the specific store or business location where the transaction took place.

Data can consist of any characters

Min length: 1 Max length: 255
posTerminal.store.name
String
OPTIONAL
Your name for the specific store or business location where the transaction took place.

Data can consist of any characters

Min length: 1 Max length: 255
responseControls
OPTIONAL
Container for fields that control the response returned for the request.

responseControls.sensitiveData
String
OPTIONAL
Indicates how sensitive data is returned in the response.

Data can consist of any characters

Min length: 1 Max length: 50
session.id
ASCII Text
OPTIONAL
Identifier of the payment session containing values for any of the request fields to be used in this operation.

Values provided in the request will override values contained in the session.

Data consists of ASCII characters

Min length: 31 Max length: 35
session.version
ASCII Text
OPTIONAL
Use this field to implement optimistic locking of the session content.

Do this if you make business decisions based on data from the session and wish to ensure that the same data is being used for the request operation.

To use optimistic locking, record session.version when you make your decisions, and then pass that value in session.version when you submit your request operation to the gateway.

If session.version provided by you does not match that stored against the session, the gateway will reject the operation with error.cause=INVALID_REQUEST.

See Making Business Decisions Based on Session Content.

Data consists of ASCII characters

Min length: 10 Max length: 10
shipping
OPTIONAL
Shipping information for this order.

shipping.address
OPTIONAL
The address to which this order will be shipped.

shipping.address.city
String
OPTIONAL
The city portion of the address.

Data can consist of any characters

Min length: 1 Max length: 100
shipping.address.company
String
OPTIONAL
The name of the company associated with this address.

Data can consist of any characters

Min length: 1 Max length: 100
shipping.address.country
Upper case alphabetic text
OPTIONAL
The 3 letter ISO standard alpha country code of the address.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
shipping.address.postcodeZip
Alphanumeric + additional characters
OPTIONAL
The post code or zip code of the address.

Data may consist of the characters 0-9, a-z, A-Z, ' ', '-'

Min length: 1 Max length: 10
shipping.address.source
Enumeration
OPTIONAL
How you obtained the shipping address.

Value must be a member of the following list. The values are case sensitive.

ADDRESS_ON_FILE
Order shipped to an address that you have on file.

NEW_ADDRESS
Order shipped to an address provided by the payer for this transaction.

shipping.address.stateProvince
String
OPTIONAL
The state or province of the address.

Data can consist of any characters

Min length: 1 Max length: 20
shipping.address.stateProvinceCode
String
OPTIONAL
The three character ISO 3166-2 country subdivision code for the state or province of the address.

Providing this field might improve your payer experience for 3-D Secure payer authentication.

Data can consist of any characters

Min length: 1 Max length: 3
shipping.address.street
String
OPTIONAL
The first line of the address.

For example, this may be the street name and number, or the Post Office Box details.

Note: The transaction response will contain a concatenation of street and street2 data. If the concatenated value is more than the maximum field length, street2 data will be truncated.

Data can consist of any characters

Min length: 1 Max length: 100
shipping.address.street2
String
OPTIONAL
The second line of the address (if provided).

Note: This field will be empty in the transaction response, as street2 data will be concatenated into the street field.

Data can consist of any characters

Min length: 1 Max length: 100
shipping.address.sameAsBilling
Enumeration
OPTIONAL
Indicates whether the shipping address provided is the same as the payer's billing address.

Provide this value if you are not providing the full shipping and billing addresses, but you can affirm that they are the same or different.

The default value for this field is:

SAME - if the shipping and billing address are supplied, and all fields are the same (ignoring non-alphanumerics).
DIFFERENT - if the shipping and billing address are supplied, and at least one field is different (ignoring non-alphanumerics).
UNKNOWN - either shipping address or billing address is absent.

Value must be a member of the following list. The values are case sensitive.

DIFFERENT
The shipping and billing addresses are different.

SAME
The shipping and billing addresses are the same.

UNKNOWN
It is not known if the shipping and billing addresses are the same.

shipping.contact
OPTIONAL
Details of the contact person at the address the goods will be shipped to.

shipping.contact.email
Email
OPTIONAL
The contact person's email address.

The field format restriction ensures that the email address is longer than 3 characters and adheres to a generous subset of valid RFC 2822 email addresses.

Ensures that the email address is longer than 3 characters and adheres to a generous subset of valid RFC 2822 email addresses

shipping.contact.firstName
String
OPTIONAL
The first name of the person to whom the order is being shipped.

Data can consist of any characters

Min length: 1 Max length: 50
shipping.contact.lastName
String
OPTIONAL
The last name or surname of the person to whom the order is being shipped.

Data can consist of any characters

Min length: 1 Max length: 50
shipping.contact.mobilePhone
Telephone Number
OPTIONAL
The contact person's mobile phone or cell phone number in ITU-T E123 format, for example +1 607 1234 5678

The number consists of:
'+'
country code (1, 2 or 3 digits)
'space'
national number ( which may embed single spaces characters for readability).

Data consists of '+', country code (1, 2 or 3 digits), 'space', and national number (which may embed single space characters for readability)

Mandatory country code: true Max total digits: 15
shipping.contact.phone
Telephone Number
OPTIONAL
The contact person's phone number in ITU-T E123 format, for example +1 607 1234 456

The number consists of:
'+'
country code (1, 2 or 3 digits)
'space'
national number ( which may embed single spaces characters for readability).

Data consists of '+', country code (1, 2 or 3 digits), 'space', and national number (which may embed single space characters for readability)

Mandatory country code: true Max total digits: 15
shipping.method
Enumeration
OPTIONAL
The shipping method used for delivery of this order.

Value must be a member of the following list. The values are case sensitive.

ELECTRONIC
Electronic delivery.

GROUND
Ground (4 or more days).

NOT_SHIPPED
Order for goods that are not shipped (for example, travel and event tickets)

OVERNIGHT
Overnight (next day).

PICKUP
Shipped to a local store for pick up.

PRIORITY
Priority (2-3 days).

SAME_DAY
Same day.

shipping.origin.postcodeZip
Alphanumeric + additional characters
OPTIONAL
The post code or zip code of the address the order is shipped from.

Data may consist of the characters 0-9, a-z, A-Z, ' ', '-'

Min length: 1 Max length: 10
sourceOfFunds
OPTIONAL
Information about the payment type selected by the payer for this payment and the source of the funds.

Depending on the payment type the source of the funds can be a debit or credit card, bank account, or account with a browser payment provider (such as PayPal).

For card payments the source of funds information may be represented by combining one or more of the following: explicitly provided card details, a session identifier which the gateway will use to look up the card details and/or a card token. Precedence rules will be applied in that explicitly provided card details will override session card details which will override card token details. Each of these may represent partial card details, however the combination must result in a full and complete set of card details. See Using Multiple Sources of Card Details for examples.

sourceOfFunds.provided
OPTIONAL
Information about the source of funds when it is directly provided (as opposed to via a token or session).

For browser payments, the source of funds details are usually collected from the payer on the payment provider's website and provided to you when you retrieve the transaction details (for a successful transaction). However, for some payment types (such as giropay), you must collect the information from the payer and supply it here.

sourceOfFunds.provided.ach
OPTIONAL
For ACH payments (sourceOfFunds.type=ACH) you must provide values for all fields within this parameter group, including details about the payers bank account as well as the type of ACH payment.

It is your responsibility to authenticate the payer and obtain authorization from the payer in accordance with the NACHA Operating Rules and Guidelines for the Standard Entry Class (SEC) associated with this payment. For details please refer to https://www.nacha.org/.

sourceOfFunds.provided.ach.accountType
Enumeration
OPTIONAL
An indicator identifying the type of bank account.

Consumer (checking or savings), or
Business
For pre-arranged payments (sourceOfFunds.provided.ach.secCode=PPD) retrieve this information from the payer.

If payments were telephone-initiated (sourceOfFunds.provided.ach.secCode=TEL) or internet-initiated (sourceOfFunds.provided.ach.secCode=WEB) you may choose to limit the payer's options (e.g. only support consumer checking accounts), depending on your type of business (e.g. B2C online webshop).


Value must be a member of the following list. The values are case sensitive.

CONSUMER_CHECKING
Consumer Checking Account

CONSUMER_SAVINGS
Consumer Savings Account

CORPORATE_CHECKING
Business Checking Account

sourceOfFunds.provided.ach.bankAccountHolder
String
OPTIONAL
The name of the bank account holder, as it appears on the account at the receiving financial institution.

Retrieve this information from the payer.

Data can consist of any characters

Min length: 1 Max length: 28
sourceOfFunds.provided.ach.bankAccountNumber
Alphanumeric + additional characters
OPTIONAL
The identifier of the bank account at the receiving financial institution.

Retrieve this information from the payer.

Data may consist of the characters 0-9, a-z, A-Z, ' ', '-', '/'

Min length: 1 Max length: 17
sourceOfFunds.provided.ach.routingNumber
Digits
OPTIONAL
The identifier of the receiving financial institution.

Also known as:
Routing number,
Transit number, or
ABA number
Retrieve this information from the payer.

See also http://en.wikipedia.org/wiki/Routing_transit_number.


Data is a string that consists of the characters 0-9.

Min length: 9 Max length: 9
sourceOfFunds.provided.ach.secCode
Enumeration
OPTIONAL
Identifies the Standard Entry Class (SEC) code to be sent to the issuer.

The SEC is defined by NACHA and describes the origin and intent of the payment. For details please refer to https://www.nacha.org/.

Value must be a member of the following list. The values are case sensitive.

PPD
An ACH debit or credit payment (B2C) that has been authorized by an authenticated customer in written form (signed or similarly authenticated). PPD is used for pre-arranged payments (e.g. employee payroll, mortgage payments, expense reimbursement).

TEL
An ACH debit payment (B2C) that has been authorized by an authenticated customer via phone. TEL may only be used if a relationship already exists between you and the consumer, or, the consumer initiates the contact with you.

WEB
An ACH debit payment (B2C) that has been authorized by an authenticated customer via the internet or a wireless network.

sourceOfFunds.provided.card
OPTIONAL
Details about the card.

Use this parameter group when you have sourced payment details using:
Cards: the card details entered directly or collected using a Point of Sale (POS) terminal.
Device payment methods such as Apple Pay, Android Pay, Samsung Pay or Google Pay.
Digital wallets such as Masterpass, Visa Checkout or Amex Express Checkout.
Card scheme tokens where the card was tokenized using a card scheme tokenization service such as Mastercard Digital Enablement Service (MDES).

sourceOfFunds.provided.card.accountType
Enumeration
OPTIONAL
You can provide this field for card types that have a savings/checking option, such as Maestro cards.

If you do not provide a value, we will use the acquirer's default. You can use paymentTypes.card.cardTypes in the 'Retrieve Payment Options' operation response to determine the card type.

Value must be a member of the following list. The values are case sensitive.

CHECKING
SAVINGS
sourceOfFunds.provided.card.emvRequest
String
OPTIONAL
This field only applies to transactions that originate from an EMV capable terminal.

It contains selected EMV fields as provided by the terminal.

For the list of field tags to include (if provided by the terminal), see Card Present Payments. Requests with any other tags are rejected by the gateway.

Some of the tags represent data that can occur on explicit fields in this API. You can submit the value either in this field, or in both places. For example, the PAN can be presented as EMV tag 5A in this field, or included both the sourceOfFunds.provided.card.number API field and in EMV tag 5A in this field.

If you specify the EMV tag only, we can populate the explicit field in the API. Fields where this is supported have the text "This field corresponds to EMV tag <tag name>" in their field descriptions.

If you specify both places, there will be no population of the explicit field or validation that the data matches.

The API response will not contain PCI sensitive fields.

Data can consist of any characters

Min length: 1 Max length: 250
sourceOfFunds.provided.card.expiry
OPTIONAL
Expiry date, as shown on the card or as provided for a card scheme token.

This field corresponds to EMV tag 5F24

sourceOfFunds.provided.card.expiry.month
Digits
REQUIRED
Month, as shown on the card.

Months are numbered January=1, through to December=12.

Data is a number between 1 and 12 represented as a string.

sourceOfFunds.provided.card.expiry.year
Digits
REQUIRED
Year, as shown on the card.

The Common Era year is 2000 plus this value.

Data is a string that consists of the characters 0-9.

Min length: 2 Max length: 2
sourceOfFunds.provided.card.maskedFpan
Masked digits
OPTIONAL
The Funding Primary Account Number (FPAN) of the payer's account in 6.4 masking format, for example, 000000xxxxxx0000.

You should use this value for display on confirmation or receipt pages presented to the payer.

RequestNormally you do not need to populate this field, as the gateway will populate this field in the session, and populate it into the payment request when you submit the payment using the session. You would only provide this value, if you had access to FPAN information that was not available to the gateway. On responses, the gateway populates it with a form that the payer would recognize (also explained in more detail below).

Retrieve session response

The gateway always populates this field with its best understanding of the masked FPAN.If you are showing PAN data from the session to the payer, then use this field rather than sourceOfFunds.provided.card.number from the session. This is because this field contains a PAN that the payer will recognize whereas sourceOfFunds.provided.card.number could contain a scheme token, or device PAN which the payer will not recognize. After the payment is processed, the gateway will populate the sourceOfFunds.provided.card.number in the payment response with its best understanding of the masked FPAN. You can show this value to the payer after the payment is complete. This logic also applies to the maskedFpanExpiry field.

Data is a string that consists of the characters 0-9, plus 'x' for masking

Min length: 9 Max length: 19
sourceOfFunds.provided.card.maskedFpanExpiry
OPTIONAL
Expiry date, The expiry date of the Funding Primary Account Number (FPAN) in sourceOfFunds.provided.card.maskedFpan.

sourceOfFunds.provided.card.maskedFpanExpiry.month
Digits
OPTIONAL
The expiration month for the Funding Primary Account Number (FPAN).

Months are numbered January=1, through to December=12.

Data is a number between 1 and 12 represented as a string.

sourceOfFunds.provided.card.maskedFpanExpiry.year
Digits
OPTIONAL
The expiration year for the Funding Primary Account Number (FPAN).

The Common Era year is 2000 plus this value.

Data is a string that consists of the characters 0-9.

Min length: 2 Max length: 2
sourceOfFunds.provided.card.nameOnCard
String
OPTIONAL
The cardholder's name as printed on the card.

Data can consist of any characters

Min length: 1 Max length: 256
sourceOfFunds.provided.card.number
Digits
OPTIONAL
The account number of the payer's account used for the payment.

On requests, provide the number in the form that you receive it (as explained below). On responses, the gateway populates it with a form that the payer would recognize (also explained in more detail below).

Request

On request, populate this field based on the payment method you are using for the payment:
• Card: the account number embossed onto the card. This field corresponds to EMV tag 5A.
• Device payment methods such as Apple Pay, Android Pay, Samsung Pay, or Google Pay. Normally for device payments, you would populate sourceOfFunds.provided.card.devicePayment.paymentToken and the gateway will decrypt and extract this field. However, you can populate this field if you decrypt the payment token yourself. In this case use the Device PAN (DPAN) provided in the payment token.
• Digital wallets such as Masterpass, Visa Checkout or Amex Express Checkout. In this case, provide the PAN retrieved from the wallet.
• Scheme tokens such as MDES (Mastercard Digital Enablement Service) or Visa Token Service (VTS). For MDES tokens, supply the value called the "Token PAN". For VTS tokens, supply the value called "Token"
Response

On return, the card number will be populated in 6.4 masking format, for example, 000000xxxxxx0000. If you wish to return unmasked card numbers, you must have the requisite permission, set responseControls.sensitiveData field to UNMASK, and authenticate your call to the API using certificate authentication.

When a DPAN or scheme token was provided in the transaction request, then this field will represent the PAN of the associated payer's account (when supported by the acquirer). This is also referred to as the Funding PAN (FPAN).

Data is a string that consists of the characters 0-9.

Min length: 9 Max length: 19
sourceOfFunds.provided.card.p2pe
OPTIONAL
This holds the PAN in the case where it is encrypted by the terminal using DUKPT key exchange.

sourceOfFunds.provided.card.p2pe.cardBin
Digits
OPTIONAL
The BIN of the card.

If you provide this, the gateway will check that the decrypted PAN has these leading six digits. If the check fails, the gateway will reject the transaction.

If you do not provided this, the gateway will not perform this check.

Data is a string that consists of the characters 0-9.

Min length: 1 Max length: 6
sourceOfFunds.provided.card.p2pe.encryptionState
String
OPTIONAL
The P2PE encryption state as determined by the terminal.

INVALID means the terminal detected some form of error in the encryption process. The gateway will decline transactions with INVALID encryption state. This field may be omitted when the value is VALID.

Data can consist of any characters

Min length: 5 Max length: 7
sourceOfFunds.provided.card.p2pe.initializationVector
Hex
OPTIONAL
The initialization vector supplied by the terminal to seed the encryption of this payload.

Omit this value if the terminal is not using an initialization vector to seed encryption.

Data is hexadecimal encoded

Min length: 16 Max length: 64
sourceOfFunds.provided.card.p2pe.keySerialNumber
Hex
REQUIRED
The DUKPT key serial number supplied by the terminal.

Data is hexadecimal encoded

Min length: 20 Max length: 24
sourceOfFunds.provided.card.p2pe.payload
Hex
REQUIRED
The DUKPT encrypted payload supplied by the terminal.

Data is hexadecimal encoded

Min length: 32 Max length: 1024
sourceOfFunds.provided.card.pin
OPTIONAL
The PIN (Personal Identification Number) entered by a payer at the point of sale that is used to authenticate their identity as the cardholder with the issuer.

Provide this data in the case where you want the PIN verified online by the issuer. The gateway can support PINs encoded in ISO 9564-1 formats 0, 1, 3 and 4, but the supported format will depend on integration.

sourceOfFunds.provided.card.pin.encryptionState
Enumeration
OPTIONAL
The PIN encryption state as determined by the terminal.

INVALID means the terminal detected some form of error in the encryption process. The gateway will decline transactions with INVALID encryption state. This field may be omitted when the value is VALID.

Value must be a member of the following list. The values are case sensitive.

INVALID
The encryption state is invalid.

VALID
The encryption state is valid.

sourceOfFunds.provided.card.pin.keySerialNumber
Hex
OPTIONAL
The DUKPT key serial number supplied by the terminal.

Data is hexadecimal encoded

Min length: 20 Max length: 24
sourceOfFunds.provided.card.pin.keySetId
Hex
OPTIONAL
Unique ID for the key PIN encryption key or BDK exchanged with the gateway.

Data is hexadecimal encoded

Min length: 6 Max length: 8
sourceOfFunds.provided.card.pin.payload
Hex
REQUIRED
The DUKPT encrypted payload supplied by the terminal.

Data is hexadecimal encoded

Min length: 16 Max length: 32
sourceOfFunds.provided.card.securityCode
Digits
OPTIONAL
Card verification code, as printed on the back or front of the card or as provided for a card scheme token.

Data is a string that consists of the characters 0-9.

Min length: 3 Max length: 4
sourceOfFunds.provided.card.sequenceNumber
Digits
OPTIONAL
A number used to differentiate between cards with the same Primary Account Number (PAN).

This field corresponds to EMV tag 5F34

Data is a number between 0 and 999 represented as a string.

sourceOfFunds.provided.card.storedOnFile
Enumeration
OPTIONAL
This field only applies if you collect card details from your payer, store them, and either you or your payer use the stored credentials for subsequent payments.

Refer to Credential on File, Cardholder and Merchant-initiated Transactions for details.

Value must be a member of the following list. The values are case sensitive.

NOT_STORED
Set this value if you are not storing the card details provided by the payer for this transaction. The gateway sets this value by default, if you are submitting a payer-initiated transaction. You must not use this value when submitting merchant-initiated transactions.

STORED
Set this value if you have previously stored the card details provided by the payer and are now using these stored details. The gateway sets this value by default, if you are submitting a merchant-initiated transaction.

TO_BE_STORED
Set this value if this is the first transaction using these card details and you intend to store the card details if the transaction is successful.

sourceOfFunds.provided.card.track1
Track 1 Data
OPTIONAL
This field contains the full track data.

You may optionally include the start and end sentinels and LRC.

Provide this for stripe and EMV fallback to stripe cases.

This field corresponds to EMV tag 56

Data must comply with ISO 7811-2 track 1 data character set.
Data may consist of the characters: 

Min length: 2 Max length: 79
sourceOfFunds.provided.card.track2
Track 2 Data
OPTIONAL
This field contains the full track data.

You may optionally include the start and end sentinels and LRC.

Provide this for stripe and EMV fallback to stripe cases.

The contents of this field must match the PAN and expiry fields included in the Transaction Request.

This field corresponds to EMV tag 57

Data must comply with ISO 7811-2 track 2 data character set.
Data may consist of the characters: 

Min length: 2 Max length: 40
sourceOfFunds.provided.directDebitCanada
OPTIONAL
For Direct Debit payments in Canada you must provide the payer's bank account details in this parameter group.

sourceOfFunds.provided.directDebitCanada.accountType
Enumeration
OPTIONAL
An indicator identifying the type of bank account.

Value must be a member of the following list. The values are case sensitive.

CONSUMER_CHECKING
Consumer Checking Account

CONSUMER_SAVINGS
Consumer Savings Account

sourceOfFunds.provided.directDebitCanada.bankAccountHolder
String
REQUIRED
The name of the bank account holder, as it appears on the account at the receiving financial institution.

Data can consist of any characters

Min length: 1 Max length: 28
sourceOfFunds.provided.directDebitCanada.bankAccountNumber
Alphanumeric + additional characters
REQUIRED
The identifier of the bank account at the receiving financial institution.

The bank account number will be returned in a masked format, for example, xxxxxxxxxxxxx123.

Data may consist of the characters 0-9, a-z, A-Z, ' ', '-', '/'

Min length: 7 Max length: 12
sourceOfFunds.provided.directDebitCanada.financialInstitutionNumber
Digits
REQUIRED
The identifier of the receiving financial institution.

Data is a string that consists of the characters 0-9.

Min length: 3 Max length: 3
sourceOfFunds.provided.directDebitCanada.transitNumber
Digits
REQUIRED
The transit number identifying the branch of the receiving financial institution where the bank account is held.

Data is a string that consists of the characters 0-9.

Min length: 5 Max length: 5
sourceOfFunds.provided.ebt
OPTIONAL
If the payer chose to pay using a Electronic Benefits Transfer card, you must submit sourceOfFunds.type=EBT_CARD and provide the payer's card details in this parameter group.

sourceOfFunds.provided.ebt.accountType
Enumeration
OPTIONAL
Indicates the type of benefits account used for the payment.

Value must be a member of the following list. The values are case sensitive.

CASH_BENEFITS
Benefits provided as cash.

EWIC_BENEFITS
Benefits provided under the Special Supplemental Nutrition Program for Women, Infants, and Children.

SNAP_BENEFITS
Benefits provided under the Supplemental Nutrition Assistance Program.

sourceOfFunds.provided.ebt.manualAuthorizationCode
Digits
OPTIONAL
Value provided to you by the EBT merchant helpline when you requested manual authorization of the payment because you were unable to authorize the payment online.

For example, your point of sale (POS) machine is not working. When you manually authorize a payment you also need to provided the voucher number used to record the payment in sourceOfFunds.provided.EBT.voucherNumber.

Data is a string that consists of the characters 0-9.

Min length: 1 Max length: 6
sourceOfFunds.provided.ebt.merchantFns
Digits
OPTIONAL
The identifier assigned to you by the USDA Food and Nutrition Service (FNS) when they authorized you to accept EBT payments at your store.

Data is a string that consists of the characters 0-9.

Min length: 1 Max length: 7
sourceOfFunds.provided.ebt.voucherNumber
Digits
OPTIONAL
The number of the paper form (voucher) that you used to record details of an EBT payment when you were unable to authorize the payment online.

For example, your point of sale (POS) machine is not working. When you use a voucher you also need to provide an authorization code in sourceOfFunds.provided.benefits.ebt.manualAuthorizationCode.

Data is a string that consists of the characters 0-9.

Min length: 1 Max length: 15
sourceOfFunds.provided.giftCard
OPTIONAL
If the payer chose to pay using a gift card, you must submit sourceOfFunds.type=GIFT_CARD and provide the payer's gift card details in this parameter group.

sourceOfFunds.provided.giftCard.expectedLocalBrand
String
OPTIONAL
Do not provide this field in your request unless instructed to do so by your payment service provider.

The field is required, if your gift card numbers do not use ISO BIN rules and therefore not allowing the gateway to identify the local brand.

Data can consist of any characters

Min length: 4 Max length: 50
sourceOfFunds.provided.giftCard.number
Digits
OPTIONAL
Card number as printed or embossed on the gift card.

Data is a string that consists of the characters 0-9.

Min length: 9 Max length: 19
sourceOfFunds.provided.giftCard.pin
Digits
OPTIONAL
PIN number for the gift card.

Data is a string that consists of the characters 0-9.

Min length: 4 Max length: 8
sourceOfFunds.provided.paysafecard
OPTIONAL
Additional details related to a paysafecard refund.

sourceOfFunds.provided.paysafecard.accountEmail
Email
REQUIRED
The mypaysafecard account email identifying the mypaysafecard account that will be refunded.

Ensures that the email address is longer than 3 characters and adheres to a generous subset of valid RFC 2822 email addresses

sourceOfFunds.provided.sepa
OPTIONAL
Details about the payer's bank account used for a payment made using SEPA

sourceOfFunds.provided.sepa.bankAccountHolder
String
REQUIRED
The name of the bank account holder for the payer's bank account.

Data can consist of any characters

Min length: 3 Max length: 100
sourceOfFunds.provided.sepa.bic
Alphanumeric
REQUIRED
The international Business Identifier Code (BIC) for the payer's bank account.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 8 Max length: 11
sourceOfFunds.provided.sepa.iban
String
REQUIRED
The International Bank Account Number (IBAN) for the payer's bank account.

By default, the bank account number will be returned in a masked format, for example, xxxxxx0000. If you wish to return unmasked bank account numbers, you must have the requisite permission, set responseControls.sensitiveData, and authenticate your call to the API using certificate authentication. Contact your payment service provider for further information.

Data can consist of any characters

Min length: 1 Max length: 50
sourceOfFunds.token
Alphanumeric
OPTIONAL
A gateway token that contains account identifier details.

The account identifier details stored against this token will be used to process the request.
If account identifier details are also contained in the request, or the request contains a session with account identifier details, these take precedence over the details stored against the token.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 1 Max length: 40
sourceOfFunds.tokenRequestorID
Alphanumeric
OPTIONAL
The unique identifier assigned to you by the Token Service Provider that you requested a token from for this payment.

This field is mandatory for payments where the Chase Pay wallet was used.

Data may consist of the characters 0-9, a-z, A-Z

Min length: 11 Max length: 11
sourceOfFunds.type
Enumeration
OPTIONAL
The payment method used for this payment.

If you are passing card data (in any form) on the API, then you need to set this value, and also provide the card details in the sourceOfFunds.provided.card group. In the case of digital wallets or device payment methods, you must also populate the order.walletProvider field.

If you are making a payment with a gateway token, then you can leave this field unset, and only populate the sourceOfFunds.token field. However you can set this to CARD if you want to overwrite or augment the token data with a card security code, expiry date, or cardholder name.

Value must be a member of the following list. The values are case sensitive.

ACH
The payer chose to pay using an electronic fund transfer, to be processed via the Automated Clearing House (ACH) Network. You must provide the payer's bank account details and information about the type of ACH payment under the sourceOfFunds.provided.ach parameter group.

CARD
Use this value for payments that obtained the card details either directly from the card, or from a POS terminal, or from a wallet, or through a device payment method.

DIRECT_DEBIT_CANADA
The payer chose to pay using Direct Debit in Canada, also known as pre-authorized bank debits (PADs). You must provide the payer's bank account details in the sourceOfFunds.provided.directDebitCanada parameter group.

EBT_CARD
Use this value for Electronic Benefits Transfer (EBT) card payments. The additional EBT data must also be provided in the sourceOfFunds.provided.ebt parameter group.

GIFT_CARD
The payer chose to pay using a gift card. The payer's gift card details must be provided under the sourceOfFunds.provided.giftCard parameter group.

PAYSAFECARD
The payer selected the payment method paysafecard.

SCHEME_TOKEN
Use this value for payments using scheme tokens provided by Mastercard Digital Enablement Service (MDES), or Visa Token Service (VTS), or American Express Token Service (AETS).

SEPA
The payer selected the payment method SEPA.

transaction
REQUIRED
Information about this transaction.

transaction.acquirer
OPTIONAL
Additional information to be passed to acquirer.

transaction.acquirer.customData
String
OPTIONAL
Additional information requested by the acquirer which cannot be passed using other available data fields.

This field must not contain sensitive data.

Data can consist of any characters, but sensitive data will be rejected

Min length: 1 Max length: 2048
transaction.acquirer.traceId
String
OPTIONAL
The unique identifier that allows the issuer to link related transactions.

Typically the gateway takes care of submitting this identifier to the issuer on your behalf. However, you must submit this identifier if you have processed the payer-initiated transaction (also called CIT) for the payment agreement outside the gateway or you are submitting a Refund where the Authorization or Payment has been performed outside the gateway.

For a Mastercard transaction this identifier must contain the scheme issued transaction identifier, network code and network date, and is also known as the Trace ID. For a Visa or American Express transaction this identifier matches the scheme issued transaction identifier, also known as Transaction Identifier or TID. Refer to the scheme's documentation for more details.

Payment in a Series

You must provide the information returned in the Authorization/Payment/Verification response for the last payer-initiated transaction in the series (CIT).

Refund

You must provide the information returned in the Authorization/Payment response for the payment for which you are issuing a refund.

Resubmission

For resubmission transactions, the gateway will include the scheme transaction identifier from the failed transaction.

However, you may use this field to directly provide the scheme transaction identifier to be used on the resubmission in certain scenarios:

Original failed transaction was processed outside the gateway.
Multiple failed authorizations exist on the order and the resubmission needs to refer to a failed transaction which is not the latest.

Data can consist of any characters, but sensitive data will be rejected

Min length: 1 Max length: 15
transaction.acquirer.transactionId
String
OPTIONAL
This is the value provided to the acquirer to identify the order.

Ideally this will be the order.id, however if that value cannot be used directly, it will be transformed by the gateway to a unique value that the acquirer will accept. If that behavior is not suitable, you can directly provide the value in this field and it will be passed to the acquirer. You then take responsibility for its correctness. (Note: Contact your payment provider to see if this is supported for your acquirer).

Data can consist of any characters, but sensitive data will be rejected

Min length: 1 Max length: 100
transaction.amount
Decimal
REQUIRED
Transaction Amount.

Expressed as a decimal number in the units of the currency. For example 12.34 in USD is the amount 12 dollars and 34 cents.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
transaction.currency
Upper case alphabetic text
REQUIRED
The currency of the transaction expressed as an ISO 4217 alpha code, e.g. USD.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
transaction.discountAmount
Decimal
OPTIONAL
The total amount deducted from the transaction amount that would normally apply.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
transaction.dutyAmount
Decimal
OPTIONAL
The duty amount (also known as customs tax, tariff or dues) for the transaction.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
transaction.item[n]
OPTIONAL
Information about the items the payer purchases with the order.

transaction.item[n].brand
String
OPTIONAL
The brand of the item.

For example, Dell.

Data can consist of any characters

Min length: 1 Max length: 127
transaction.item[n].category
String
OPTIONAL
Your category for the item.

Data can consist of any characters

Min length: 1 Max length: 127
transaction.item[n].description
String
OPTIONAL
Description for the item with information such as size, color, etc.

For example, 'Color:Red, Size:M'

Data can consist of any characters

Min length: 1 Max length: 127
transaction.item[n].detail
OPTIONAL
Only use this parameter group to provide additional line item details required for a better interchange rate for Purchasing Cards, Business and/or Corporate Cards (Level 3).

Check with your payment service provider if Level 3 data is supported for your acquirer.

transaction.item[n].detail.acquirerCustom
JSON Text
OPTIONAL
Use this field to provide line item details that your acquirer requires you to provide.

Data must be provided in JSON format using the record name and field name (separated by a comma) to identify the value provided. Contact your payment service provider for details about the supported fields including the field definitions.

Data is valid Json Format

Min length: 1 Max length: 4000
transaction.item[n].detail.commodityCode
Digits
OPTIONAL
A code describing a commodity or a group of commodities pertaining to goods classification.

Data is a number between 1 and 9999999999999999 represented as a string.

transaction.item[n].detail.tax[n]
OPTIONAL
Information about the taxes per line item.

transaction.item[n].detail.tax[n].amount
Decimal
OPTIONAL
The tax amount for the tax type defined in order.item[n].detail.tax[m].type for the item.

Note that the tax amount provided must reflect the tax amount applied before a discount was applied.

Data is a string that consists of the characters 0-9, '.' and '-' and represents a valid decimal number.

Min length: 1 Max length: 14
transaction.item[n].detail.tax[n].rate
Decimal
OPTIONAL
The tax rate (percentage) applied to the item for the tax type defined in order.item[n].detail.tax[m].type.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 6
transaction.item[n].detail.tax[n].type
String
OPTIONAL
The tax type for which the amount specified under order.item[n].detail.tax[m].amount has been paid for this item.

The correct value as used by your acquirer may have to be provided. Contact your payment service provider for details.

Data can consist of any characters

Min length: 1 Max length: 127
transaction.item[n].detail.unitDiscountRate
Decimal
OPTIONAL
The discount rate (percentage) applied to this item.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 6
transaction.item[n].detail.unitTaxRate
Decimal
OPTIONAL
The tax rate (percentage) of the tax charged for this item.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 6
transaction.item[n].detail.unitTaxType
String
OPTIONAL
The type of tax charged for this item.

The correct value as used by your acquirer may have to be provided. Contact your payment service provider for details.

Data can consist of any characters

Min length: 1 Max length: 10
transaction.item[n].detail.unspsc
Digits
OPTIONAL
The United Nations Standard Products and Services Code (UNSPSC) for the item.

Data is a number between 1 and 9999999999999999 represented as a string.

transaction.item[n].detail.upc
Digits
OPTIONAL
The Universal Product Code (UPC) for the item.

Data is a number between 1 and 9999999999999999 represented as a string.

transaction.item[n].industryCategory
Enumeration
OPTIONAL
Provide the industry category to send this line item to your acquirer for specialized processing as industry data.

Such processing might have legal obligations, which are your responsibility. Do not provide an industry category, unless you are certain it applies to you, and is accepted by your acquirer.
We support the following industry standard processing:
US health care processing using the IIAS standard.
The supported values for this field are:
HEALTHCARE_VISION, HEALTHCARE_DENTAL, HEALTHCARE_PRESCRIPTION, HEALTHCARE_OTHER
We formulate an IIAS message by summing the amounts of all the line items with the same industry category. The amount of a line item is computed as:
(order.item.unitPrice + order.item.tax) * order.item.quantity

Value must be a member of the following list. The values are case sensitive.

HEALTHCARE_DENTAL
HEALTHCARE_OTHER
HEALTHCARE_PRESCRIPTION
HEALTHCARE_VISION
transaction.item[n].name
String
REQUIRED
A short name describing the item.

Data can consist of any characters

Min length: 1 Max length: 127
transaction.item[n].quantity
Decimal
REQUIRED
The quantity of the item.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number greater than zero.

Min length: 0 Max length: 30
transaction.item[n].sku
String
OPTIONAL
The SKU (Stock Keeping Unit) or the item identifier for this item.

Data can consist of any characters

Min length: 1 Max length: 127
transaction.item[n].unitDiscountAmount
Decimal
OPTIONAL
The discount amount applied to this item.

Data is a string that consists of the characters 0-9, '.' and '-' and represents a valid decimal number.

Min length: 1 Max length: 14
transaction.item[n].unitOfMeasure
String
OPTIONAL
The unit of measure used for the item quantity.

The correct value as used by your acquirer may have to be provided. Contact your payment service provider for details.

Data can consist of any characters

Min length: 1 Max length: 10
transaction.item[n].unitPrice
Decimal
REQUIRED
The cost price for the item.

This amount is multiplied with the item quantity (item.quantity) to determine the total amount for this item (item.amount). This amount does not include the tax amount and/or discount amount applicable to this item.

Data is a string that consists of the characters 0-9, '.' and '-' and represents a valid decimal number.

Min length: 1 Max length: 14
transaction.item[n].unitTaxAmount
Decimal
OPTIONAL
The tax amount for the item.

This amount is multiplied with the item quantity (item.quantity) to determine the total tax amount for this item.

Data is a string that consists of the characters 0-9, '.' and '-' and represents a valid decimal number.

Min length: 1 Max length: 14
transaction.itemAmount
Decimal
OPTIONAL
The total item amount for this transaction.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
transaction.merchantNote
String
OPTIONAL
Your note about this transaction.

Data can consist of any characters

Min length: 1 Max length: 250
transaction.priorApproval
Enumeration
OPTIONAL
Indicates that a transaction requires approval to proceed with the order.

Value must be a member of the following list. The values are case sensitive.

REQUESTED
Requested

transaction.reference
String
OPTIONAL
An optional identifier for this transaction.

Data can consist of any characters

Min length: 1 Max length: 40
transaction.resubmission
Boolean
OPTIONAL
Indicates that this merchant initiated transaction is a resubmission for a previous authorization which failed due to insufficient funds.

A resubmission transaction must also contain the 'scheme transaction Id' from the associated failed transaction. You can provide the referenceOrderId of the relevant order and the gateway will include the 'scheme transaction Id' of the last failed authorization on the order in the resubmission transaction. Alternatively, you can provide the 'scheme transaction Id' of the failed transaction in the resubmission transaction using the field transaction.acquirer.traceId.

JSON boolean values 'true' or 'false'.

transaction.serviceLocation
OPTIONAL
Use this parameter group when transaction service location is different than your registered business location.

transaction.serviceLocation.city
String
OPTIONAL
The city where cardholder received the service.

Data can consist of any characters

Min length: 1 Max length: 100
transaction.serviceLocation.country
Upper case alphabetic text
OPTIONAL
The country where cardholder received the service.

Data must consist of the characters A-Z

Min length: 3 Max length: 3
transaction.serviceLocation.postCodeZip
Alphanumeric + additional characters
OPTIONAL
The zip or postal code where cardholder received the service.

Data may consist of the characters 0-9, a-z, A-Z, ' ', '-'

Min length: 1 Max length: 10
transaction.serviceLocation.stateProvinceCode
String
OPTIONAL
The state or province where cardholder received the service.

The value must match the second part of the ISO 3166-2 code. For an address in the United States provide the 2-letter ISO 3166-2 state code. For US military bases provide one of AE, AA, AP. For an address in Canada provide the 2-letter ISO 3166-2 province code.

Data can consist of any characters

Min length: 1 Max length: 3
transaction.shippingAndHandlingAmount
Decimal
OPTIONAL
The total shipping and handling amount for the transaction, including taxes on the shipping and/or handling.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
transaction.shippingAndHandlingTaxAmount
Decimal
OPTIONAL
The tax amount levied on the shipping and handling amount for the transaction.

This amount is included in the shipping and handling amount provided in field transaction.shippingAndHandlingAmount.

Data is a decimal number.

Max value: 1000000000000 Min value: 0 Max post-decimal digits: 3
transaction.shippingAndHandlingTaxRate
Decimal
OPTIONAL
The tax rate applied to the shipping and handling amount for the transaction to determine the shipping and handling tax amount.

For a tax rate of 2.5% provide 0.025.

Data is a decimal number.

Max value: 1000000000000000000 Min value: 0 Max post-decimal digits: 4
transaction.source
Enumeration
OPTIONAL
Indicates the channel through which you received authorization for the payment for this order from the payer.

For example, set this value to INTERNET if the payer initiated the payment online.

If you have an existing agreement with the payer that authorizes you to process this payment (for example, a recurring payment) then set this value to MERCHANT. You only need to provide transaction.source if you want to override the default value configured for your acquirer link.

Note:

You can only override the default value if you have the requisite permission.
The value you provide must match one of those configured by your payment service provider.
You can only set the transaction source on the initial transaction on an order. It cannot be changed on subsequent transactions.

Value must be a member of the following list. The values are case sensitive.

CALL_CENTRE
Transaction conducted via a call centre.

CARD_PRESENT
Transaction where the card is presented to the merchant.

INTERNET
Transaction conducted over the Internet.

MAIL_ORDER
Transaction received by mail.

MERCHANT
Transaction initiated by you based on an agreement with the payer. For example, a recurring payment, installment payment, or account top-up.

MOTO
Transaction received by mail or telephone.

PAYER_PRESENT
Transaction where a non-card payment method is presented to the Merchant.

TELEPHONE_ORDER
Transaction received by telephone.

VOICE_RESPONSE
Transaction conducted by a voice/DTMF recognition system.

transaction.targetTransactionId
String
OPTIONAL
The identifier for the transaction you wish to refund.

That is the {transactionId} URL field for REST and the transaction.id field for NVP.

If you do not provide a target transaction ID the gateway will try to identify a transaction. If no transaction can be found or more than one transaction is identified, the request is rejected.

Data can consist of any characters

Min length: 1 Max length: 40
transaction.tax[n]
OPTIONAL
Use this parameter group to provide a breakdown of tax types, amount per tax type, and rate per tax type included in transaction.taxAmount.

transaction.tax[n].amount
Decimal
OPTIONAL
The tax amount included in this transaction for the tax type.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
transaction.tax[n].rate
Decimal
OPTIONAL
The tax rate (percentage) used to determine the tax amount included in this transaction for the tax type.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 6
transaction.tax[n].type
String
OPTIONAL
The type of tax included in the transaction amount.

The correct value as used by your acquirer may have to be provided. Contact your payment service provider for details.

Data can consist of any characters

Min length: 1 Max length: 50
transaction.taxAmount
Decimal
OPTIONAL
The total tax amount for the transaction.

You only need to provide this field when you capture or refund part of the order amount. In this case, the amount must not exceed order.taxAmount. If you provide this field when you capture or refund the full amount of the order, then the value provided must match order.taxAmount.

Data is a string that consists of the characters 0-9 and '.' and represents a valid decimal number.

Min length: 1 Max length: 14
transaction.taxStatus
Enumeration
OPTIONAL
Indicates your tax status for this transaction.

Value must be a member of the following list. The values are case sensitive.

EXEMPT
Indicates that you are exempt from tax.

NOT_EXEMPT
Indicates that you are not exempt from tax.

NOT_PROVIDED
Indicates that you are not providing information about being exempt from tax.

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