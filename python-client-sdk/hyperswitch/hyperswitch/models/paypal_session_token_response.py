from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.sdk_next_action import SdkNextAction


T = TypeVar("T", bound="PaypalSessionTokenResponse")


@_attrs_define
class PaypalSessionTokenResponse:
    """
    Attributes:
        connector (str): Name of the connector
        session_token (str): The session token for PayPal
        sdk_next_action (SdkNextAction):
    """

    connector: str
    session_token: str
    sdk_next_action: "SdkNextAction"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        connector = self.connector

        session_token = self.session_token

        sdk_next_action = self.sdk_next_action.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "connector": connector,
                "session_token": session_token,
                "sdk_next_action": sdk_next_action,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.sdk_next_action import SdkNextAction

        d = dict(src_dict)
        connector = d.pop("connector")

        session_token = d.pop("session_token")

        sdk_next_action = SdkNextAction.from_dict(d.pop("sdk_next_action"))

        paypal_session_token_response = cls(
            connector=connector,
            session_token=session_token,
            sdk_next_action=sdk_next_action,
        )

        paypal_session_token_response.additional_properties = d
        return paypal_session_token_response

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
