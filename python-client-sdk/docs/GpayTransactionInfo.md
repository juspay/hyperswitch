# GpayTransactionInfo


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**country_code** | [**CountryAlpha2**](CountryAlpha2.md) |  | 
**currency_code** | [**Currency**](Currency.md) |  | 
**total_price_status** | **str** | The total price status (ex: &#39;FINAL&#39;) | 
**total_price** | **str** | The total price | 

## Example

```python
from hyperswitch.models.gpay_transaction_info import GpayTransactionInfo

# TODO update the JSON string below
json = "{}"
# create an instance of GpayTransactionInfo from a JSON string
gpay_transaction_info_instance = GpayTransactionInfo.from_json(json)
# print the JSON string representation of the object
print(GpayTransactionInfo.to_json())

# convert the object into a dict
gpay_transaction_info_dict = gpay_transaction_info_instance.to_dict()
# create an instance of GpayTransactionInfo from a dict
gpay_transaction_info_from_dict = GpayTransactionInfo.from_dict(gpay_transaction_info_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


