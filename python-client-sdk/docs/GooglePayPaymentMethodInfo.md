# GooglePayPaymentMethodInfo


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_network** | **str** | The name of the card network | 
**card_details** | **str** | The details of the card | 
**assurance_details** | [**GooglePayAssuranceDetails**](GooglePayAssuranceDetails.md) |  | [optional] 

## Example

```python
from hyperswitch.models.google_pay_payment_method_info import GooglePayPaymentMethodInfo

# TODO update the JSON string below
json = "{}"
# create an instance of GooglePayPaymentMethodInfo from a JSON string
google_pay_payment_method_info_instance = GooglePayPaymentMethodInfo.from_json(json)
# print the JSON string representation of the object
print(GooglePayPaymentMethodInfo.to_json())

# convert the object into a dict
google_pay_payment_method_info_dict = google_pay_payment_method_info_instance.to_dict()
# create an instance of GooglePayPaymentMethodInfo from a dict
google_pay_payment_method_info_from_dict = GooglePayPaymentMethodInfo.from_dict(google_pay_payment_method_info_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


