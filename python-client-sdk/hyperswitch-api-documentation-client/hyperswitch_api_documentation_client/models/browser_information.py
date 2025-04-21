from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="BrowserInformation")


@_attrs_define
class BrowserInformation:
    """Browser information to be used for 3DS 2.0

    Attributes:
        color_depth (Union[None, Unset, int]): Color depth supported by the browser
        java_enabled (Union[None, Unset, bool]): Whether java is enabled in the browser
        java_script_enabled (Union[None, Unset, bool]): Whether javascript is enabled in the browser
        language (Union[None, Unset, str]): Language supported
        screen_height (Union[None, Unset, int]): The screen height in pixels
        screen_width (Union[None, Unset, int]): The screen width in pixels
        time_zone (Union[None, Unset, int]): Time zone of the client
        ip_address (Union[None, Unset, str]): Ip address of the client
        accept_header (Union[None, Unset, str]): List of headers that are accepted Example:
            text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8.
        user_agent (Union[None, Unset, str]): User-agent of the browser
        os_type (Union[None, Unset, str]): The os type of the client device
        os_version (Union[None, Unset, str]): The os version of the client device
        device_model (Union[None, Unset, str]): The device model of the client
    """

    color_depth: Union[None, Unset, int] = UNSET
    java_enabled: Union[None, Unset, bool] = UNSET
    java_script_enabled: Union[None, Unset, bool] = UNSET
    language: Union[None, Unset, str] = UNSET
    screen_height: Union[None, Unset, int] = UNSET
    screen_width: Union[None, Unset, int] = UNSET
    time_zone: Union[None, Unset, int] = UNSET
    ip_address: Union[None, Unset, str] = UNSET
    accept_header: Union[None, Unset, str] = UNSET
    user_agent: Union[None, Unset, str] = UNSET
    os_type: Union[None, Unset, str] = UNSET
    os_version: Union[None, Unset, str] = UNSET
    device_model: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        color_depth: Union[None, Unset, int]
        if isinstance(self.color_depth, Unset):
            color_depth = UNSET
        else:
            color_depth = self.color_depth

        java_enabled: Union[None, Unset, bool]
        if isinstance(self.java_enabled, Unset):
            java_enabled = UNSET
        else:
            java_enabled = self.java_enabled

        java_script_enabled: Union[None, Unset, bool]
        if isinstance(self.java_script_enabled, Unset):
            java_script_enabled = UNSET
        else:
            java_script_enabled = self.java_script_enabled

        language: Union[None, Unset, str]
        if isinstance(self.language, Unset):
            language = UNSET
        else:
            language = self.language

        screen_height: Union[None, Unset, int]
        if isinstance(self.screen_height, Unset):
            screen_height = UNSET
        else:
            screen_height = self.screen_height

        screen_width: Union[None, Unset, int]
        if isinstance(self.screen_width, Unset):
            screen_width = UNSET
        else:
            screen_width = self.screen_width

        time_zone: Union[None, Unset, int]
        if isinstance(self.time_zone, Unset):
            time_zone = UNSET
        else:
            time_zone = self.time_zone

        ip_address: Union[None, Unset, str]
        if isinstance(self.ip_address, Unset):
            ip_address = UNSET
        else:
            ip_address = self.ip_address

        accept_header: Union[None, Unset, str]
        if isinstance(self.accept_header, Unset):
            accept_header = UNSET
        else:
            accept_header = self.accept_header

        user_agent: Union[None, Unset, str]
        if isinstance(self.user_agent, Unset):
            user_agent = UNSET
        else:
            user_agent = self.user_agent

        os_type: Union[None, Unset, str]
        if isinstance(self.os_type, Unset):
            os_type = UNSET
        else:
            os_type = self.os_type

        os_version: Union[None, Unset, str]
        if isinstance(self.os_version, Unset):
            os_version = UNSET
        else:
            os_version = self.os_version

        device_model: Union[None, Unset, str]
        if isinstance(self.device_model, Unset):
            device_model = UNSET
        else:
            device_model = self.device_model

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if color_depth is not UNSET:
            field_dict["color_depth"] = color_depth
        if java_enabled is not UNSET:
            field_dict["java_enabled"] = java_enabled
        if java_script_enabled is not UNSET:
            field_dict["java_script_enabled"] = java_script_enabled
        if language is not UNSET:
            field_dict["language"] = language
        if screen_height is not UNSET:
            field_dict["screen_height"] = screen_height
        if screen_width is not UNSET:
            field_dict["screen_width"] = screen_width
        if time_zone is not UNSET:
            field_dict["time_zone"] = time_zone
        if ip_address is not UNSET:
            field_dict["ip_address"] = ip_address
        if accept_header is not UNSET:
            field_dict["accept_header"] = accept_header
        if user_agent is not UNSET:
            field_dict["user_agent"] = user_agent
        if os_type is not UNSET:
            field_dict["os_type"] = os_type
        if os_version is not UNSET:
            field_dict["os_version"] = os_version
        if device_model is not UNSET:
            field_dict["device_model"] = device_model

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_color_depth(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        color_depth = _parse_color_depth(d.pop("color_depth", UNSET))

        def _parse_java_enabled(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        java_enabled = _parse_java_enabled(d.pop("java_enabled", UNSET))

        def _parse_java_script_enabled(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        java_script_enabled = _parse_java_script_enabled(d.pop("java_script_enabled", UNSET))

        def _parse_language(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        language = _parse_language(d.pop("language", UNSET))

        def _parse_screen_height(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        screen_height = _parse_screen_height(d.pop("screen_height", UNSET))

        def _parse_screen_width(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        screen_width = _parse_screen_width(d.pop("screen_width", UNSET))

        def _parse_time_zone(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        time_zone = _parse_time_zone(d.pop("time_zone", UNSET))

        def _parse_ip_address(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        ip_address = _parse_ip_address(d.pop("ip_address", UNSET))

        def _parse_accept_header(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        accept_header = _parse_accept_header(d.pop("accept_header", UNSET))

        def _parse_user_agent(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        user_agent = _parse_user_agent(d.pop("user_agent", UNSET))

        def _parse_os_type(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        os_type = _parse_os_type(d.pop("os_type", UNSET))

        def _parse_os_version(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        os_version = _parse_os_version(d.pop("os_version", UNSET))

        def _parse_device_model(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        device_model = _parse_device_model(d.pop("device_model", UNSET))

        browser_information = cls(
            color_depth=color_depth,
            java_enabled=java_enabled,
            java_script_enabled=java_script_enabled,
            language=language,
            screen_height=screen_height,
            screen_width=screen_width,
            time_zone=time_zone,
            ip_address=ip_address,
            accept_header=accept_header,
            user_agent=user_agent,
            os_type=os_type,
            os_version=os_version,
            device_model=device_model,
        )

        browser_information.additional_properties = d
        return browser_information

    @property
    def additional_keys(self) -> list[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
