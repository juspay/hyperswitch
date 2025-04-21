from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.routable_connector_choice import RoutableConnectorChoice


T = TypeVar("T", bound="ProfileDefaultRoutingConfig")


@_attrs_define
class ProfileDefaultRoutingConfig:
    """
    Attributes:
        profile_id (str):
        connectors (list['RoutableConnectorChoice']):
    """

    profile_id: str
    connectors: list["RoutableConnectorChoice"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        profile_id = self.profile_id

        connectors = []
        for connectors_item_data in self.connectors:
            connectors_item = connectors_item_data.to_dict()
            connectors.append(connectors_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "profile_id": profile_id,
                "connectors": connectors,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.routable_connector_choice import RoutableConnectorChoice

        d = dict(src_dict)
        profile_id = d.pop("profile_id")

        connectors = []
        _connectors = d.pop("connectors")
        for connectors_item_data in _connectors:
            connectors_item = RoutableConnectorChoice.from_dict(connectors_item_data)

            connectors.append(connectors_item)

        profile_default_routing_config = cls(
            profile_id=profile_id,
            connectors=connectors,
        )

        profile_default_routing_config.additional_properties = d
        return profile_default_routing_config

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
