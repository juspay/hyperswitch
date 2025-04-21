# SplitPaymentsRequestOneOf1


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**xendit_split_payment** | [**XenditSplitRequest**](XenditSplitRequest.md) |  | 

## Example

```python
from hyperswitch.models.split_payments_request_one_of1 import SplitPaymentsRequestOneOf1

# TODO update the JSON string below
json = "{}"
# create an instance of SplitPaymentsRequestOneOf1 from a JSON string
split_payments_request_one_of1_instance = SplitPaymentsRequestOneOf1.from_json(json)
# print the JSON string representation of the object
print(SplitPaymentsRequestOneOf1.to_json())

# convert the object into a dict
split_payments_request_one_of1_dict = split_payments_request_one_of1_instance.to_dict()
# create an instance of SplitPaymentsRequestOneOf1 from a dict
split_payments_request_one_of1_from_dict = SplitPaymentsRequestOneOf1.from_dict(split_payments_request_one_of1_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


