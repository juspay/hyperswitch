from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.routable_connector_choice import RoutableConnectorChoice


T = TypeVar("T", bound="ConnectorVolumeSplit")


@_attrs_define
class ConnectorVolumeSplit:
    """
    Attributes:
        connector (RoutableConnectorChoice): Routable Connector chosen for a payment
        split (int):
    """

    connector: "RoutableConnectorChoice"
    split: int
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        connector = self.connector.to_dict()

        split = self.split

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "connector": connector,
                "split": split,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.routable_connector_choice import RoutableConnectorChoice

        d = dict(src_dict)
        connector = RoutableConnectorChoice.from_dict(d.pop("connector"))

        split = d.pop("split")

        connector_volume_split = cls(
            connector=connector,
            split=split,
        )

        connector_volume_split.additional_properties = d
        return connector_volume_split

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
