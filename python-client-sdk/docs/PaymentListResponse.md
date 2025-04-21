# PaymentListResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**size** | **int** | The number of payments included in the list | 
**data** | [**List[PaymentsResponse]**](PaymentsResponse.md) |  | 

## Example

```python
from hyperswitch.models.payment_list_response import PaymentListResponse

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentListResponse from a JSON string
payment_list_response_instance = PaymentListResponse.from_json(json)
# print the JSON string representation of the object
print(PaymentListResponse.to_json())

# convert the object into a dict
payment_list_response_dict = payment_list_response_instance.to_dict()
# create an instance of PaymentListResponse from a dict
payment_list_response_from_dict = PaymentListResponse.from_dict(payment_list_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


