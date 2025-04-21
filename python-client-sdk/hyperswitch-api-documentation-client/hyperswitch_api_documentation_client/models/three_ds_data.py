from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.poll_config_response import PollConfigResponse
    from ..models.three_ds_method_data_type_0 import ThreeDsMethodDataType0


T = TypeVar("T", bound="ThreeDsData")


@_attrs_define
class ThreeDsData:
    """
    Attributes:
        three_ds_authentication_url (str): ThreeDS authentication url - to initiate authentication
        three_ds_authorize_url (str): ThreeDS authorize url - to complete the payment authorization after authentication
        three_ds_method_details ('ThreeDsMethodDataType0'):
        poll_config (PollConfigResponse):
        message_version (Union[None, Unset, str]): Message Version
        directory_server_id (Union[None, Unset, str]): Directory Server ID
    """

    three_ds_authentication_url: str
    three_ds_authorize_url: str
    three_ds_method_details: "ThreeDsMethodDataType0"
    poll_config: "PollConfigResponse"
    message_version: Union[None, Unset, str] = UNSET
    directory_server_id: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.three_ds_method_data_type_0 import ThreeDsMethodDataType0

        three_ds_authentication_url = self.three_ds_authentication_url

        three_ds_authorize_url = self.three_ds_authorize_url

        three_ds_method_details: dict[str, Any]
        if isinstance(self.three_ds_method_details, ThreeDsMethodDataType0):
            three_ds_method_details = self.three_ds_method_details.to_dict()

        poll_config = self.poll_config.to_dict()

        message_version: Union[None, Unset, str]
        if isinstance(self.message_version, Unset):
            message_version = UNSET
        else:
            message_version = self.message_version

        directory_server_id: Union[None, Unset, str]
        if isinstance(self.directory_server_id, Unset):
            directory_server_id = UNSET
        else:
            directory_server_id = self.directory_server_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "three_ds_authentication_url": three_ds_authentication_url,
                "three_ds_authorize_url": three_ds_authorize_url,
                "three_ds_method_details": three_ds_method_details,
                "poll_config": poll_config,
            }
        )
        if message_version is not UNSET:
            field_dict["message_version"] = message_version
        if directory_server_id is not UNSET:
            field_dict["directory_server_id"] = directory_server_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.poll_config_response import PollConfigResponse
        from ..models.three_ds_method_data_type_0 import ThreeDsMethodDataType0

        d = dict(src_dict)
        three_ds_authentication_url = d.pop("three_ds_authentication_url")

        three_ds_authorize_url = d.pop("three_ds_authorize_url")

        def _parse_three_ds_method_details(data: object) -> "ThreeDsMethodDataType0":
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_three_ds_method_data_type_0 = ThreeDsMethodDataType0.from_dict(data)

            return componentsschemas_three_ds_method_data_type_0

        three_ds_method_details = _parse_three_ds_method_details(d.pop("three_ds_method_details"))

        poll_config = PollConfigResponse.from_dict(d.pop("poll_config"))

        def _parse_message_version(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        message_version = _parse_message_version(d.pop("message_version", UNSET))

        def _parse_directory_server_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        directory_server_id = _parse_directory_server_id(d.pop("directory_server_id", UNSET))

        three_ds_data = cls(
            three_ds_authentication_url=three_ds_authentication_url,
            three_ds_authorize_url=three_ds_authorize_url,
            three_ds_method_details=three_ds_method_details,
            poll_config=poll_config,
            message_version=message_version,
            directory_server_id=directory_server_id,
        )

        three_ds_data.additional_properties = d
        return three_ds_data

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
