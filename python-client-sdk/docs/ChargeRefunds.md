# ChargeRefunds

Charge specific fields for controlling the revert of funds from either platform or connected account. Check sub-fields for more details.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**charge_id** | **str** | Identifier for charge created for the payment | 
**revert_platform_fee** | **bool** | Toggle for reverting the application fee that was collected for the payment. If set to false, the funds are pulled from the destination account. | [optional] 
**revert_transfer** | **bool** | Toggle for reverting the transfer that was made during the charge. If set to false, the funds are pulled from the main platform&#39;s account. | [optional] 

## Example

```python
from hyperswitch.models.charge_refunds import ChargeRefunds

# TODO update the JSON string below
json = "{}"
# create an instance of ChargeRefunds from a JSON string
charge_refunds_instance = ChargeRefunds.from_json(json)
# print the JSON string representation of the object
print(ChargeRefunds.to_json())

# convert the object into a dict
charge_refunds_dict = charge_refunds_instance.to_dict()
# create an instance of ChargeRefunds from a dict
charge_refunds_from_dict = ChargeRefunds.from_dict(charge_refunds_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


