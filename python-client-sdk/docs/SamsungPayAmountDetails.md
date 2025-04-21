# SamsungPayAmountDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**option** | [**SamsungPayAmountFormat**](SamsungPayAmountFormat.md) |  | 
**currency_code** | [**Currency**](Currency.md) |  | 
**total** | **str** | The total amount of the transaction | 

## Example

```python
from hyperswitch.models.samsung_pay_amount_details import SamsungPayAmountDetails

# TODO update the JSON string below
json = "{}"
# create an instance of SamsungPayAmountDetails from a JSON string
samsung_pay_amount_details_instance = SamsungPayAmountDetails.from_json(json)
# print the JSON string representation of the object
print(SamsungPayAmountDetails.to_json())

# convert the object into a dict
samsung_pay_amount_details_dict = samsung_pay_amount_details_instance.to_dict()
# create an instance of SamsungPayAmountDetails from a dict
samsung_pay_amount_details_from_dict = SamsungPayAmountDetails.from_dict(samsung_pay_amount_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


