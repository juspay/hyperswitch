# OnlineMandate


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**ip_address** | **str** | Ip address of the customer machine from which the mandate was created | 
**user_agent** | **str** | The user-agent of the customer&#39;s browser | 

## Example

```python
from hyperswitch.models.online_mandate import OnlineMandate

# TODO update the JSON string below
json = "{}"
# create an instance of OnlineMandate from a JSON string
online_mandate_instance = OnlineMandate.from_json(json)
# print the JSON string representation of the object
print(OnlineMandate.to_json())

# convert the object into a dict
online_mandate_dict = online_mandate_instance.to_dict()
# create an instance of OnlineMandate from a dict
online_mandate_from_dict = OnlineMandate.from_dict(online_mandate_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


