# Upi


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**upi** | [**UpiData**](UpiData.md) |  | 

## Example

```python
from hyperswitch.models.upi import Upi

# TODO update the JSON string below
json = "{}"
# create an instance of Upi from a JSON string
upi_instance = Upi.from_json(json)
# print the JSON string representation of the object
print(Upi.to_json())

# convert the object into a dict
upi_dict = upi_instance.to_dict()
# create an instance of Upi from a dict
upi_from_dict = Upi.from_dict(upi_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


