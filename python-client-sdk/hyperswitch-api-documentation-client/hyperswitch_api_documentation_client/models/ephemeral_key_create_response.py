from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="EphemeralKeyCreateResponse")


@_attrs_define
class EphemeralKeyCreateResponse:
    """ephemeral_key for the customer_id mentioned

    Attributes:
        customer_id (str): customer_id to which this ephemeral key belongs to Example: cus_y3oqhf46pyzuxjbcn2giaqnb44.
        created_at (int): time at which this ephemeral key was created
        expires (int): time at which this ephemeral key would expire
        secret (str): ephemeral key
    """

    customer_id: str
    created_at: int
    expires: int
    secret: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        customer_id = self.customer_id

        created_at = self.created_at

        expires = self.expires

        secret = self.secret

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "customer_id": customer_id,
                "created_at": created_at,
                "expires": expires,
                "secret": secret,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        customer_id = d.pop("customer_id")

        created_at = d.pop("created_at")

        expires = d.pop("expires")

        secret = d.pop("secret")

        ephemeral_key_create_response = cls(
            customer_id=customer_id,
            created_at=created_at,
            expires=expires,
            secret=secret,
        )

        ephemeral_key_create_response.additional_properties = d
        return ephemeral_key_create_response

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
