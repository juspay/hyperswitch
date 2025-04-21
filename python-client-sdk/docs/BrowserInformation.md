# BrowserInformation

Browser information to be used for 3DS 2.0

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**color_depth** | **int** | Color depth supported by the browser | [optional] 
**java_enabled** | **bool** | Whether java is enabled in the browser | [optional] 
**java_script_enabled** | **bool** | Whether javascript is enabled in the browser | [optional] 
**language** | **str** | Language supported | [optional] 
**screen_height** | **int** | The screen height in pixels | [optional] 
**screen_width** | **int** | The screen width in pixels | [optional] 
**time_zone** | **int** | Time zone of the client | [optional] 
**ip_address** | **str** | Ip address of the client | [optional] 
**accept_header** | **str** | List of headers that are accepted | [optional] 
**user_agent** | **str** | User-agent of the browser | [optional] 
**os_type** | **str** | The os type of the client device | [optional] 
**os_version** | **str** | The os version of the client device | [optional] 
**device_model** | **str** | The device model of the client | [optional] 

## Example

```python
from hyperswitch.models.browser_information import BrowserInformation

# TODO update the JSON string below
json = "{}"
# create an instance of BrowserInformation from a JSON string
browser_information_instance = BrowserInformation.from_json(json)
# print the JSON string representation of the object
print(BrowserInformation.to_json())

# convert the object into a dict
browser_information_dict = browser_information_instance.to_dict()
# create an instance of BrowserInformation from a dict
browser_information_from_dict = BrowserInformation.from_dict(browser_information_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


