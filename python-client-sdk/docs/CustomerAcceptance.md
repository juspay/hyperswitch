# CustomerAcceptance

This \"CustomerAcceptance\" object is passed during Payments-Confirm request, it enlists the type, time, and mode of acceptance properties related to an acceptance done by the customer. The customer_acceptance sub object is usually passed by the SDK or client.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**acceptance_type** | [**AcceptanceType**](AcceptanceType.md) |  | 
**accepted_at** | **datetime** | Specifying when the customer acceptance was provided | [optional] 
**online** | [**OnlineMandate**](OnlineMandate.md) |  | [optional] 

## Example

```python
from hyperswitch.models.customer_acceptance import CustomerAcceptance

# TODO update the JSON string below
json = "{}"
# create an instance of CustomerAcceptance from a JSON string
customer_acceptance_instance = CustomerAcceptance.from_json(json)
# print the JSON string representation of the object
print(CustomerAcceptance.to_json())

# convert the object into a dict
customer_acceptance_dict = customer_acceptance_instance.to_dict()
# create an instance of CustomerAcceptance from a dict
customer_acceptance_from_dict = CustomerAcceptance.from_dict(customer_acceptance_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


