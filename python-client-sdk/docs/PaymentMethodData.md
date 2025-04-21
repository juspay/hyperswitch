# PaymentMethodData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card** | [**Card**](Card.md) |  | 
**card_redirect** | [**CardRedirectData**](CardRedirectData.md) |  | 
**wallet** | [**WalletData**](WalletData.md) |  | 
**pay_later** | [**PayLaterData**](PayLaterData.md) |  | 
**bank_redirect** | [**BankRedirectData**](BankRedirectData.md) |  | 
**bank_debit** | [**BankDebitData**](BankDebitData.md) |  | 
**bank_transfer** | [**BankTransferData**](BankTransferData.md) |  | 
**real_time_payment** | [**RealTimePaymentData**](RealTimePaymentData.md) |  | 
**crypto** | [**CryptoData**](CryptoData.md) |  | 
**upi** | [**UpiData**](UpiData.md) |  | 
**voucher** | [**VoucherData**](VoucherData.md) |  | 
**gift_card** | [**GiftCardData**](GiftCardData.md) |  | 
**card_token** | [**CardToken**](CardToken.md) |  | 
**open_banking** | [**OpenBankingData**](OpenBankingData.md) |  | 
**mobile_payment** | [**MobilePaymentData**](MobilePaymentData.md) |  | 

## Example

```python
from hyperswitch.models.payment_method_data import PaymentMethodData

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentMethodData from a JSON string
payment_method_data_instance = PaymentMethodData.from_json(json)
# print the JSON string representation of the object
print(PaymentMethodData.to_json())

# convert the object into a dict
payment_method_data_dict = payment_method_data_instance.to_dict()
# create an instance of PaymentMethodData from a dict
payment_method_data_from_dict = PaymentMethodData.from_dict(payment_method_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


