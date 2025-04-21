from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="OpenBankingSessionToken")


@_attrs_define
class OpenBankingSessionToken:
    """
    Attributes:
        open_banking_session_token (str): The session token for OpenBanking Connectors
    """

    open_banking_session_token: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        open_banking_session_token = self.open_banking_session_token

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "open_banking_session_token": open_banking_session_token,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        open_banking_session_token = d.pop("open_banking_session_token")

        open_banking_session_token = cls(
            open_banking_session_token=open_banking_session_token,
        )

        open_banking_session_token.additional_properties = d
        return open_banking_session_token

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
