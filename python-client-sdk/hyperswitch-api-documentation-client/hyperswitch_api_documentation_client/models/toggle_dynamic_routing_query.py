from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.dynamic_routing_features import DynamicRoutingFeatures

T = TypeVar("T", bound="ToggleDynamicRoutingQuery")


@_attrs_define
class ToggleDynamicRoutingQuery:
    """
    Attributes:
        enable (DynamicRoutingFeatures):
    """

    enable: DynamicRoutingFeatures
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        enable = self.enable.value

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "enable": enable,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        enable = DynamicRoutingFeatures(d.pop("enable"))

        toggle_dynamic_routing_query = cls(
            enable=enable,
        )

        toggle_dynamic_routing_query.additional_properties = d
        return toggle_dynamic_routing_query

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
