# PaymentMethodDataResponseWithBilling


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card** | [**CardResponse**](CardResponse.md) |  | 
**bank_transfer** | [**BankTransferResponse**](BankTransferResponse.md) |  | 
**wallet** | [**WalletResponse**](WalletResponse.md) |  | 
**pay_later** | [**PaylaterResponse**](PaylaterResponse.md) |  | 
**bank_redirect** | [**BankRedirectResponse**](BankRedirectResponse.md) |  | 
**crypto** | [**CryptoResponse**](CryptoResponse.md) |  | 
**bank_debit** | [**BankDebitResponse**](BankDebitResponse.md) |  | 
**mandate_payment** | **object** |  | 
**reward** | **object** |  | 
**real_time_payment** | [**RealTimePaymentDataResponse**](RealTimePaymentDataResponse.md) |  | 
**upi** | [**UpiResponse**](UpiResponse.md) |  | 
**voucher** | [**VoucherResponse**](VoucherResponse.md) |  | 
**gift_card** | [**GiftCardResponse**](GiftCardResponse.md) |  | 
**card_redirect** | [**CardRedirectResponse**](CardRedirectResponse.md) |  | 
**card_token** | [**CardTokenResponse**](CardTokenResponse.md) |  | 
**open_banking** | [**OpenBankingResponse**](OpenBankingResponse.md) |  | 
**mobile_payment** | [**MobilePaymentResponse**](MobilePaymentResponse.md) |  | 
**billing** | [**Address**](Address.md) |  | [optional] 

## Example

```python
from hyperswitch.models.payment_method_data_response_with_billing import PaymentMethodDataResponseWithBilling

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodDataResponseWithBilling from a JSON string
payment_method_data_response_with_billing_instance = PaymentMethodDataResponseWithBilling.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodDataResponseWithBilling.to_json())

# convert the object into a dict
payment_method_data_response_with_billing_dict = payment_method_data_response_with_billing_instance.to_dict()
# create an instance of PaymentMethodDataResponseWithBilling from a dict
payment_method_data_response_with_billing_from_dict = PaymentMethodDataResponseWithBilling.from_dict(payment_method_data_response_with_billing_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


