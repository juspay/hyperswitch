Test Cards
When testing your Mastercard Gateway integration for card payments, you can trigger specific responses and results for your transaction operations using various test cards. When accessing a card emulator, use your test merchant profile with the "TEST" prefix that your payment service provider supplies.

Card transaction test details
Following are the different card transaction test details.

Standard test cards – all supported regions
Use the following standard test cards unless specific cards for your acquirer and region are provided in the other sections.

You can use different expiry dates, CSC/CVV values, and the billing address street names in the request to generate different responses.

UATP cards do not support CSC/CVV and 3DS.
Please note that "3-D Secure Enrolled" means 3DS is supported for testing these cards with authentication.channel=PAYER_BROWSER in the INITIATE_AUTHENTICATION API, but not with other channels.

To test the 3D Secure authentication functionality in more detail and using the 3DS emulator, see testing your 3DS integration.

Standard test cards

Test Cards	Card Number
Mastercard
5123450000000008
2223000000000007
5111111111111118
2223000000000023
Visa
4508750015741019
4012000033330026
Diners Club
30123400000000
36259600000012
JCB
3528000000000007
3528111100000001
Discover	6011003179988686
6011963280099774
Maestro	5000000000000000005
5666555544443333
UATP
(UATP cards do not support CSC/CVV and 3DS)	135492354874528
135420001569134
UnionPay
3DS enrolled	6201089999995464
6201089999991455
6201089999994020
6201089999999300
6201089999994749
UnionPay
Non-3DS enrolled	6214239999999611
6214239999999546
PayPak	2205459999997832
2205439999999541
Jaywan 3DS enrolled	6690109900000010
6690109000011008
6690109000011016
6690109000011024
6690109000011032
Jaywan 3DS not enrolled	6690109000011057
6690109000011065
Transaction responses for standard test cards

Expiry Date	Transaction Response Gateway Code
01/39
APPROVED
05/39
DECLINED
04/27
EXPIRED_CARD
08/28
TIMED_OUT
01/37	ACQUIRER_SYSTEM_ERROR
02/37	UNSPECIFIED_FAILURE
05/37	UNKNOWN
CSC/CVV responses for standard test cards

CSC/CVV	CSC/CVV Response Gateway Code
100
MATCH
101
NOT_PROCESSED
102
NO_MATCH
For American Express cards
1000	MATCH
1010	NOT_PROCESSED
1020	NO_MATCH
AVS responses for standard test cards

Billing Address Street	AVS Response Gateway Code
Alpha St
ADDRESS_MATCH
Gamma St
NOT_VERIFIED
November St
NO_MATCH
Romeo St
SERVICE_NOT_AVAILABLE_RETRY
Sierra St
SERVICE_NOT_SUPPORTED
Uniform St
NOT_AVAILABLE
Whiskey St
ZIP_MATCH
X-ray St
ADDRESS_ZIP_MATCH
Kilo St	NAME_MATCH
Oscar St	NAME_ADDRESS_MATCH
Lima St	NAME_ZIP_MATCH
Zero St	NOT_REQUESTED
NPCI BEPG Mastercard Gateway internal simulator
To access the Mastercard Gateway test simulator, enter "TEST" as a prefix to the Merchant ID supplied by your payment service provider. If the Merchant ID supplied already has "TEST" as the first four letters, you are already using the test simulator, and your payment service provider sends you another Merchant ID when you are ready to process live transactions.

The test simulator is configured to generate predictable results based on the transaction request and card details you supply.

Refer the following cards for Seamless flow and Alternate Identifier (Alt ID).

Use Cryptogram = AJgBASOERgAgIwYgEwcpAAAAAAE for Alt ID, also known as Guest checkout transaction
Card expiry date and CVV
Expiry	CVV
05/28
111
 

Rupay Test Cards according to the Use Case/Scenario
Rupay Use Case/Scenario	Authentication Mode	Card Number	Cryptogram	OTP
Non-SI transaction for signed-in customers
Redirection	6074849200004917	APJUR+bB46ysAAKYEAOYGgADFA==	123456
Seamless	6074849900004936	APJUR+bB46ysAAKYEAOYGgADFA==	123456
Seamless	6074849900004936	APJUR+bB46ysAAKYEAOYGgADFA==	12345
Guest Checkout
Redirection	6074849200004917	AJgBASOERgAgIwYgEwcpAAAAAAE	123456
Seamless	6074849900004936	AJgBASOERgAgIwYgEwcpAAAAAAE	123456
Wrong OTP generation
Seamless	6074849900004936	APJUR+bB46ysAAKYEAOYGgADFA==	NA
OTP verification Fails
Seamless	6074849900004936	APJUR+bB46ysAAKYEAOYGgADFA==	1236
Seamless	6074849900004936	APJUR+bB46ysAAKYEAOYGgADFA==	1235
Seamless	6074849900004936	APJUR+bB46ysAAKYEAOYGgADFA==	123456
Simulation behavior
Amount	Response
0.00
APPROVED
1.20
INSUFFICIENT_FUNDS
8.88
TIMED_OUT
6.66
EXPIRED_CARD
.45
OTP Attempt exhausted
.71
Amount Error
.92
NO ROUTING AVAILABLE
NA
Expired OTP
NA
OTP Attempt exhausted
NA
Request successful
Standing instructions for signed-in customers
Rupay Use Case/Scenario	Authentication Mode	Card Number	Cryptogram	OTP
Standing instructions for signed-in customers
Redirection	6074849400004980	APJUR+bB46ysAAKYEAOYGgADFA==	123456
Seamless	6073849300004958	APJUR+bB46ysAAKYEAOYGgADFA==	123456
Simulation behavior
Amount	Response
.00
APPROVED
.01
SI Not Registered
.77
SI_NOT_AVAILABLE
.03
Incorrect Card Number For SI Registration ID
.04
Late Intimation
EFTPOS acquirer links - Australia
Test Cards	Card Number
Eftpos Australia/Mastercard
5555229999999975
Eftpos Australia/Mastercard
5555229999997722
Eftpos Australia/Visa
4043409999991437
Eftpos Australia/Visa
4029939999997636
 

Expiry Date	Transaction Response Gateway Code
01/39
APPROVED
05/39
DECLINED
04/27
EXPIRED_CARD
08/28
TIMED_OUT
01/27
INSUFFICIENT_FUNDS
01/37
ACQUIRER_SYSTEM_ERROR
05/37
UNKNOWN
 

CSC/CVV	CSC/CVV Response Gateway Code
100
MATCH
102
NO MATCH
Do not use these test cards in Production environment for testing. These cards are valid to use in MTF environment testing only.
Verve test cards - Nigeria
Test Cards	Card Number
Verve
5060990580000217499
5079539999990592
 

Expiry Date	Transaction Response Gateway Code
01/39
APPROVED
05/39
DECLINED
04/27
EXPIRED_CARD
08/28
TIMED_OUT
01/37
ACQUIRER_SYSTEM_ERROR
02/37
UNSPECIFIED_FAILURE
05/37
UNKNOWN
 

CSC/CVV	CSC/CVV Response Gateway Code
100
MATCH
101
NOT_PROCESSED
102
NO MATCH
 

Jaywan test cards – UAE
Jaywan mono-badge test cards
Test Cards	Card Number
Jaywan mono-badge 3DS enrolled
6690109900000010
6690109000011008
6690109000011016
6690109000011024
6690109000011032
Jaywan mono-badge 3DS not enrolled
6690109900001125
6690109900001216
6690109900001315
6690109900001414
6690109000011040
6690109000011057
6690109000011065
6690109000011073
6690109000011081
6690109000011099
 

Jaywan co-badge Mastercard test Cards
Test Cards	Card Number
Jaywan co-badge Mastercard 3DS enrolled	5175540000050008
5175540000050099
5175540000050073
Jaywan co-badge Mastercard 3DS not enrolled	5175540000050016
5175540000050024
5175540000050032
5175540000050040
5175540000050057
5175540000050065
5175540000050081
 

Jaywan co-badge Visa test Cards
Test Cards	Card Number
Jaywan co-badge Visa 3DS enrolled	4439130000050003
4439130000050011
4439130000050086
4439130000050094
Jaywan co-badge Visa 3DS not enrolled	4439130000050029
4439130000050037
4439130000050045
4439130000050052
4439130000050060
4439130000050078
 

Expiry Date	Transaction Response Gateway Code
01/39
APPROVED
05/39
DECLINED
04/27
EXPIRED_CARD
08/28
TIMED_OUT
01/37
ACQUIRER_SYSTEM_ERROR
02/37
UNSPECIFIED_FAILURE
05/37
UNKNOWN
 

CSC/CVV	CSC/CVV Response Gateway Code
100
MATCH
101
NOT_PROCESSED
102
NO MATCH
First Data test cards – United States
You can use different expiry dates, CSC/CVV values, and the billing address street names in the request to generate different responses.

First Data US test cards

Test Cards	Card Number
Mastercard
5149612222222229
2223000000000007
Visa
4012000033330026
Discover
6011000991300009
Diners Club
36555500001111
JCB
3566007770017510
Transaction responses for First Data US test cards

Expiry Date	Transaction Response Gateway Code
01/39
APPROVED
05/28
DECLINED
06/33
EXPIRED_CARD
10/37	TIMED_OUT
06/38
ACQUIRER_SYSTEM_ERROR
CSC/CVV responses for First Data US test cards

CSC/CVV	CSC/CVV Response Gateway Code
000
MATCH
111
NO_MATCH
222
NOT_PROCESSED
444
NOT_PRESENT
888
NOT_SUPPORTED
AVS responses for First Data US test cards

Billing Address Street	AVS Response Gateway Code
Alpha St
ADDRESS_MATCH
Gamma St
NOT_VERIFIED
November St
NO_MATCH
Romeo St
SERVICE_NOT_AVAILABLE_RETRY
Sierra St
SERVICE_NOT_SUPPORTED
Uniform St
NOT_AVAILABLE
Wiskey St
ZIP_MATCH
X-ray St
ADDRESS_ZIP_MATCH
First Data test cards – Australia
You can use different expiry dates and CSC/CVV values in the request to generate different responses.

Currently, Australian banks do not support the Address Verification Service (AVS).
First Data Australia test cards

Test Cards	Card Number
Visa
4005550000000019
Mastercard
5123450000000008
2223000000000007
Diners Club
30123400000000
Transaction responses for First Data Australia test cards

Expiry Date	Transaction Response Gateway Code
01/39
APPROVED
05/39
DECLINED
08/28
TIMED_OUT
04/27
EXPIRED_CARD
CSC/CVV responses for First Data Australia test cards

CSC/CVV	CSC/CVV Response Gateway Code
100
MATCH
101
NOT_PROCESSED
102
NO_MATCH
Other acquirer's test cards – United States
You can use different expiry dates, CSC/CVV values, and the billing address street names in the request to generate different responses.

US test cards

Test Cards	Card Number
Mastercard
5123456789012346
2223000000000007
Visa
4012000033330026
Discover
6011000991300009
Diners Club
36409300000008
JCB
3566007770017510
Transaction responses for US test cards

Expiry Date	Transaction Response Gateway Code
01/39
APPROVED
01/28
DECLINED
08/29
TIMED_OUT
03/28
EXPIRED_CARD
09/29
ACQUIRER_SYSTEM_ERROR
CSC/CVV responses for US test cards

CSC/CVV	CSC/CVV Response Gateway Code
000
MATCH
111
NO_MATCH
222
NOT_PROCESSED
333
NOT_PRESENT
444
NOT_SUPPORTED
AVS responses for US test cards

Billing Address Street	AVS Response Gateway Code
Alpha St
ADDRESS_MATCH
Mike St
ADDRESS_ZIP_MATCH
Uniform St
NOT_AVAILABLE
Kilo St
NAME_MATCH
November St
NO_MATCH
Zulu St
ZIP_MATCH
Romeo St
SERVICE_NOT_AVAILABLE_RETRY
Sierra St
SERVICE_NOT_SUPPORTED
Love St	NAME_ZIP_MATCH
Olive St	NAME_ADDRESS_MATCH
Zero St	NOT_REQUESTED
Test cards – United Kingdom
You can use different expiry dates, CSC/CVV values, and the billing address street names in the request to generate different responses.

If you are not implementing 3D Secure authentication, you can use any of the cards.
UK test cards

Test Cards	Card Number	3-D Secure Enrolled
Visa
4508750015741019	Yes
Visa Debit
4539791001730106	Yes
Mastercard
5123456789012346
2223000000000007
Yes
Maestro
6759010012345678914	Yes
Mastercard
5111111111111118
2223000000000023	No
Visa
4005550000000001	No
Transaction responses for UK test cards

Expiry Date	Transaction Response Gateway Code
01/39
APPROVED
03/28
DECLINED
04/33
EXPIRED_CARD
01/37
ACQUIRER_SYSTEM_ERROR
CSC/CVV responses for UK test cards

CSC/CVV	CSC/CVV Response Gateway Code
222
MATCH
444
NO_MATCH
111
NOT_PROCESSED
000
NOT_PRESENT
AVS responses for UK test cards

Billing Address Street	AVS Response Gateway Code
Whiskey St
ADDRESS_MATCH
Juliet St
NO_MATCH
Alpha St
SERIVCE_NOT_SUPPORTED
X-ray St
ZIP_MATCH
Mike St
ADDRESS_ZIP_MATCH
Test cards – Mexico
You can use different expiry dates in the request to generate different responses.

If you are not implementing 3D Secure authentication, you can use any of the cards.
Mexican banks do not currently support CSC/CVV matching. You can pass the CSC/CVV value, but the issuer does not process it, and no specific match response is returned.
Address Verification (AVS) is not supported for all acquirers. Check with your payment service provider.
Mexico test cards

Test Cards	Card Number	3-D Secure Enrolled
Visa
4012000033330026	No
Mastercard
5424180279791732
5200030000000004
2223000000000007	Yes
Visa
4002260000000000	No
Mastercard
2223000000000023	No
Transaction responses for Mexico test cards

Expiry Date	Transaction Response Gateway Code
01/39
APPROVED
04/28
DECLINED
02/30
EXPIRED_CARD
03/31
ACQUIRER_SYSTEM_ERROR
01/30
INSUFFICIENT_FUNDS
Test cards – France and Germany
You can use different expiry dates, CSC/CVV values, and the billing address street names in the request to generate different responses.

France and Germany test cards

Test Cards	Card Number	3-D Secure Enrolled
Mastercard
5123456789012346
2223000000000007
Yes

Visa
4012000033330026	No
JCB
3566007770017510	No
Maestro
6759010012345678914	Yes
Transaction responses for France and Germany test cards

Expiry Date	Transaction Response Gateway Code
01/39
APPROVED
01/28
DECLINED
08/29
TIMED_OUT
03/28
EXPIRED_CARD
09/29
ACQUIRER_SYSTEM_ERROR
CSC/CVV responses for France and Germany test cards

CSC/CVV	CSC/CVV Response Gateway Code
000
MATCH
111
NO_MATCH
222
NOT_PROCESSED
333
NOT_PRESENT
444
NOT_PRESENT
AVS responses for France and Germany test cards

Billing Address Street	AVS Response Gateway Code
Alpha St
ADDRESS_MATCH
X-ray St
ADDRESS_ZIP_MATCH
Uniform St
NOT_AVAILABLE
Whiskey St
ZIP_MATCH
November St
NO_MATCH
Cielo test cards – Brazil
You can use different expiry dates and CSC/CVV values in the request to generate different responses.

Cielo Brazil test cards

Test Cards	Card Number	3-D Secure Enrolled
Mastercard
5123456789012346
2223000000000007
Yes

Visa
4012000033330026	No
JCB
3566007770017510	No
Maestro
6759010012345678914	Yes
Transaction responses for Cielo Brazil test cards

Expiry Date	Transaction Response Gateway Code
01/39
APPROVED
01/28
DECLINED
08/29
TIMED_OUT
03/28
EXPIRED_CARD
09/29
ACQUIRER_SYSTEM_ERROR
CSC/CVV responses for Cielo Brazil test cards

CSC/CVV	CSC/CVV Response Gateway Code
000
MATCH
111
NO_MATCH
222
NOT_PROCESSED
333
NOT_PRESENT
444
NOT_PRESENT
Test gift cards
To test your integration using the test gift card numbers and response codes, contact your payment service provider for the test data.

Payment options inquiry test cards – Dynamic Currency Conversion
If you are enabled for Dynamic Currency Conversion (DCC), you can use different currency pairs in the PAYMENT OPTIONS INQUIRY request to trigger a response with a specific exchange rate for a specific payer currency.

In requests that use a test merchant profile, the Mastercard Payment Gateway modifies the exchange rate by retaining the first 2 relevant, non-zero values (3.2 in case of MYR) and appending 9s to fill up to 6 significant values. This is to indicate that the returned exchange rate is for test purposes only.

DCC Examples

Successful Rate Quote Request
Provide a request for a test merchant profile with:
Card number: 5313359999999089 (currency of card is MYR)
Order amount: 20 USD
The returned response has:
Payer currency: MYR
Payer exchange rate: 3.29999
Payer amount: 20 * 3.29999 = 65.9998 MYR
Currency conversion gateway response code: QUOTE_PROVIDED

Failed Rate Quote Request
Provide a request for a test merchant profile with:
Card number with a prefix for a valid DCC card type (Visa, Mastercard, or Maestro) that is not in the list of the DCC test cards (refer to the tables below).
Order amount of 20 KWD, where KWD is a currency which is not in the list of DCC test currencies (refer to the tables below).
The returned response has the currency conversion gateway response code: NOT_ELIGIBLE

You can continue testing AUTHORIZE/CAPTURE/PAY transactions using the DCC card numbers and the DCC information returned in the PAYMENT OPTIONS INQUIRY response. However, you cannot perform the Mod 10 (Luhn algorithm) check with the DCC test card numbers (as the rate quotes only consider the first 10 digits of the card number).


The following test card numbers and corresponding currencies can be used in all regions.

Base Currency = USD (US Dollar)
Test cards for USD

Test Card	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
United States Dollar(USD)	Test Outcome in Payer Authentication
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	1.11	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	13.24	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	1.27	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	7.88	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.73	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	7.76	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	102.67	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	2.32	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	32.32	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	3.27	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	1.10	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	1.19	Frictionless success
Base Currency = AUD (Australian Dollar)
Test cards for AUD

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
Australian Dollar(AUD)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	0.90	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	11.93	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	1.14	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	7.10	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.66	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	6.99	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	92.50	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	2.09	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	29.12	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	2.95	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	0.99	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	1.07	Frictionless success
Base Currency = NZD (New Zealand Dollar)
Test cards for NZD

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
New Zealand Dollar(NZD)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	0.84	Frictionless success
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	0.93	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	11.13	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	1.07	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	6.62	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.61	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	6.52	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	86.28	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	1.95	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	27.16	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	2.75	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	0.92	Frictionless success
Base Currency = SGD (Singapore Dollar)
Test cards for SGD

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
Singapore Dollar(SGD)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	0.79	Frictionless success
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	0.87	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	10.43	3DS2 - Challenge
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	6.20	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.57	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	6.11	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	80.84	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	1.83	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	25.45	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	2.57	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	0.87	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	0.94	Frictionless success
Base Currency = HKD (Hongkong Dollar)
Test cards for HKD

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
Hong Kong Dollar(HKD)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	0.13	Frictionless success
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	0.14	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	1.71	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	0.16	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	1.02	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.094	Frictionless success
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	13.23	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	0.30	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	4.16	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	0.42	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	0.14	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	0.15	Frictionless success
Base Currency = MYR (Malaysian Ringgit)
Test cards for MYR Currency

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
Malaysian Ringgit(MYR)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	0.31	Frictionless success
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	0.34	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	4.05	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	0.39	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	2.41	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.22	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	2.37	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	31.40	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	0.71	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	9.88	Frictionless success
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	0.34	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	0.36	Frictionless success
Base Currency = AED (UAE Dirham)
Test cards for AED

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
United Arab Emirates Dirham(AED)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	0.27	Frictionless success
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	0.30	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	3.58	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	0.34	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	2.13	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.20	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	2.10	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	27.75	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	0.63	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	8.74	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	0.88	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	0.30	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	0.32	Frictionless success
Base Currency = QAR (Qatari Riyal)
Test cards for QAR

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
Qatari Riyal(QAR)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	0.27	Frictionless success
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	0.30	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	3.58	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	0.34	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	2.13	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.20	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	2.10	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	27.75	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	0.63	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	8.74	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	0.88	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	0.30	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	0.32	Frictionless success
Base Currency = ZAR (South African Rand)
Test cards for ZAR

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
South African Rand(ZAR)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	0.074	Frictionless success
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	0.082	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	0.98	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	0.094	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	0.58	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.054	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	0.57	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	7.60	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	0.17	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	2.39	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	0.24	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	0.081	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	0.088	Frictionless success
Base Currency = NGN (Nigerian Naira)
Test cards for NGN

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
Nigerian Naira(NGN)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	0.0050	Frictionless success
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	0.0056	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	0.066	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	0.0064	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	0.039	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.0037	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	0.039	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	0.51	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	0.012	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	0.16	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	0.016	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	0.0055	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	0.0060	Frictionless success
Base Currency = INR (Indian Rupee)
Test cards for INR

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
Indian Rupee(INR)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	0.016	Frictionless success
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	0.018	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	0.22	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	0.021	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	0.13	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.012	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	0.13	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	1.67	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	0.038	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	0.53	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	0.053	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	0.018	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	0.019	Frictionless success
Base Currency = IDR (Indonesian Rupiah)
Test Cards for IDR

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
Indonesian Rupiah(IDR)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	0.000069	Frictionless success
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	0.0.000077	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	0.00091	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	0.000088	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	0.00054	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.000050	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	0.00054	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	0.0071	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	0.00016	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	0.0022	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	0.00023	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	0.000076	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	0.00082	Frictionless success
Base Currency = VND (Vietnamese Dong)
Test cards for VND

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
Vietnamese Dong(VND)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	0.000044	Frictionless success
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	0.000049	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	0.00058	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	0.000056	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	0.00035	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.000032	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	0.00034	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	0.0045	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	0.00010	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	0.0014	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	0.00014	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	0.000048	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	0.000052	Frictionless success
Base Currency = BHD (Bahraini Dinar)
Test cards for BHD

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
Bahraini Dinar(BHD)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	2.63	Frictionless success
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	2.92	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	34.84	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	3.34	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	20.74	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	1.92	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	20.42	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	270.18	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	6.11	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	85.05	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	8.61	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	2.89	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	3.13	Frictionless success
Base Currency = PKR (Pakistani Rupee)
Test cards for PKR

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
Pakistani Rupee(PKR)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	0.0096	Frictionless success
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	0.011	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	0.13	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	0.012	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	0.076	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.0070	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	0.074	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	0.99	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	0.022	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	0.31	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	0.031	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	0.011	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	0.011	Frictionless success
Base Currency = CNY (Chinese Yuan)
Test cards for CNY

Test Cards	Card Number	Currency	Currency Name	Exchange Rate
for Base Currency
Chinese Yuan(CNY)	Test Outcome in Payer Authentication
Mastercard
5100049999999372	USD	US Dollar	0.16	Frictionless success
Visa
Mastercard
4532249999994172
5402159999995430	AUD	Australian Dollar	0.18	Frictionless success
Visa
Mastercard
4180779999996392
5288049999998964	MXN	Mexican Peso	2.12	3DS2 - Challenge
Visa
4119089999999842	SGD	Singapore Dollar	0.20	Frictionless success
Visa
Mastercard
4923079999995763
5221379999994154	ARS	Argentinian Peso	1.26	Frictionless success
Visa
Mastercard
4907449999991296
5490019999991271	EUR	Euro	0.12	Frictionless success
Visa
Mastercard
4541879999990975
5410969999990342	HKD	Hong Kong Dollar	1.24	Frictionless decline
Visa
Mastercard
4986689999994675
5438179999994611	JPY	Japanese Yen	16.43	Frictionless decline
Mastercard
5157439999991796	BRL	Brazilian Real	0.37	Frictionless success
Visa
Mastercard
4546239999992650
5434499999995101	THB	Thai Baht	5.17	Frictionless success
Visa
Mastercard
4563769999994205
5313359999999089	MYR	Malaysian Ringgit	0.52	Frictionless decline
Visa
Mastercard
4523369999993056
5192699999999753	CAD	Canadian Dollar	0.18	Frictionless success
Visa
Mastercard
4761209999994624
5191639999996152	NZD	New Zealand Dollar	0.19	Frictionless success