from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="ToggleKVResponse")


@_attrs_define
class ToggleKVResponse:
    """
    Attributes:
        merchant_id (str): The identifier for the Merchant Account Example: y3oqhf46pyzuxjbcn2giaqnb44.
        kv_enabled (bool): Status of KV for the specific merchant Example: True.
    """

    merchant_id: str
    kv_enabled: bool
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        merchant_id = self.merchant_id

        kv_enabled = self.kv_enabled

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "merchant_id": merchant_id,
                "kv_enabled": kv_enabled,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        merchant_id = d.pop("merchant_id")

        kv_enabled = d.pop("kv_enabled")

        toggle_kv_response = cls(
            merchant_id=merchant_id,
            kv_enabled=kv_enabled,
        )

        toggle_kv_response.additional_properties = d
        return toggle_kv_response

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
