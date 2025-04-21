from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.sdk_next_action import SdkNextAction


T = TypeVar("T", bound="GooglePayThirdPartySdk")


@_attrs_define
class GooglePayThirdPartySdk:
    """
    Attributes:
        delayed_session_token (bool): Identifier for the delayed session response
        connector (str): The name of the connector
        sdk_next_action (SdkNextAction):
    """

    delayed_session_token: bool
    connector: str
    sdk_next_action: "SdkNextAction"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        delayed_session_token = self.delayed_session_token

        connector = self.connector

        sdk_next_action = self.sdk_next_action.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "delayed_session_token": delayed_session_token,
                "connector": connector,
                "sdk_next_action": sdk_next_action,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.sdk_next_action import SdkNextAction

        d = dict(src_dict)
        delayed_session_token = d.pop("delayed_session_token")

        connector = d.pop("connector")

        sdk_next_action = SdkNextAction.from_dict(d.pop("sdk_next_action"))

        google_pay_third_party_sdk = cls(
            delayed_session_token=delayed_session_token,
            connector=connector,
            sdk_next_action=sdk_next_action,
        )

        google_pay_third_party_sdk.additional_properties = d
        return google_pay_third_party_sdk

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
