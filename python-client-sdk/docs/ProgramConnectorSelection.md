# ProgramConnectorSelection

The program, having a default connector selection and a bunch of rules. Also can hold arbitrary metadata.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**default_selection** | [**ConnectorSelection**](ConnectorSelection.md) |  | 
**rules** | [**RuleConnectorSelection**](RuleConnectorSelection.md) |  | 
**metadata** | **Dict[str, object]** |  | 

## Example

```python
from hyperswitch.models.program_connector_selection import ProgramConnectorSelection

# TODO update the JSON string below
json = "{}"
# create an instance of ProgramConnectorSelection from a JSON string
program_connector_selection_instance = ProgramConnectorSelection.from_json(json)
# print the JSON string representation of the object
print(ProgramConnectorSelection.to_json())

# convert the object into a dict
program_connector_selection_dict = program_connector_selection_instance.to_dict()
# create an instance of ProgramConnectorSelection from a dict
program_connector_selection_from_dict = ProgramConnectorSelection.from_dict(program_connector_selection_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


