# SplitPaymentsRequestOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**stripe_split_payment** | [**StripeSplitPaymentRequest**](StripeSplitPaymentRequest.md) |  | 

## Example

```python
from hyperswitch.models.split_payments_request_one_of import SplitPaymentsRequestOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of SplitPaymentsRequestOneOf from a JSON string
split_payments_request_one_of_instance = SplitPaymentsRequestOneOf.from_json(json)
# print the JSON string representation of the object
print(SplitPaymentsRequestOneOf.to_json())

# convert the object into a dict
split_payments_request_one_of_dict = split_payments_request_one_of_instance.to_dict()
# create an instance of SplitPaymentsRequestOneOf from a dict
split_payments_request_one_of_from_dict = SplitPaymentsRequestOneOf.from_dict(split_payments_request_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


