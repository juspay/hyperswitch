# PaymentListConstraints


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**customer_id** | **str** | The identifier for customer | [optional] 
**starting_after** | **str** | A cursor for use in pagination, fetch the next list after some object | [optional] 
**ending_before** | **str** | A cursor for use in pagination, fetch the previous list before some object | [optional] 
**limit** | **int** | limit on the number of objects to return | [optional] [default to 10]
**created** | **datetime** | The time at which payment is created | [optional] 
**created_lt** | **datetime** | Time less than the payment created time | [optional] 
**created_gt** | **datetime** | Time greater than the payment created time | [optional] 
**created_lte** | **datetime** | Time less than or equals to the payment created time | [optional] 
**created_gte** | **datetime** | Time greater than or equals to the payment created time | [optional] 

## Example

```python
from hyperswitch.models.payment_list_constraints import PaymentListConstraints

# TODO update the JSON string below
json = "{}"
# create an instance of PaymentListConstraints from a JSON string
payment_list_constraints_instance = PaymentListConstraints.from_json(json)
# print the JSON string representation of the object
print(PaymentListConstraints.to_json())

# convert the object into a dict
payment_list_constraints_dict = payment_list_constraints_instance.to_dict()
# create an instance of PaymentListConstraints from a dict
payment_list_constraints_from_dict = PaymentListConstraints.from_dict(payment_list_constraints_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


