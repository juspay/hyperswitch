from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="RevokeApiKeyResponse")


@_attrs_define
class RevokeApiKeyResponse:
    """The response body for revoking an API Key.

    Attributes:
        merchant_id (str): The identifier for the Merchant Account. Example: y3oqhf46pyzuxjbcn2giaqnb44.
        key_id (str): The identifier for the API Key. Example: 5hEEqkgJUyuxgSKGArHA4mWSnX.
        revoked (bool): Indicates whether the API key was revoked or not. Example: true.
    """

    merchant_id: str
    key_id: str
    revoked: bool
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        merchant_id = self.merchant_id

        key_id = self.key_id

        revoked = self.revoked

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "merchant_id": merchant_id,
                "key_id": key_id,
                "revoked": revoked,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        merchant_id = d.pop("merchant_id")

        key_id = d.pop("key_id")

        revoked = d.pop("revoked")

        revoke_api_key_response = cls(
            merchant_id=merchant_id,
            key_id=key_id,
            revoked=revoked,
        )

        revoke_api_key_response.additional_properties = d
        return revoke_api_key_response

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
