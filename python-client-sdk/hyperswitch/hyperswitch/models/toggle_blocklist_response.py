from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="ToggleBlocklistResponse")


@_attrs_define
class ToggleBlocklistResponse:
    """
    Attributes:
        blocklist_guard_status (str):
    """

    blocklist_guard_status: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        blocklist_guard_status = self.blocklist_guard_status

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "blocklist_guard_status": blocklist_guard_status,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        blocklist_guard_status = d.pop("blocklist_guard_status")

        toggle_blocklist_response = cls(
            blocklist_guard_status=blocklist_guard_status,
        )

        toggle_blocklist_response.additional_properties = d
        return toggle_blocklist_response

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
