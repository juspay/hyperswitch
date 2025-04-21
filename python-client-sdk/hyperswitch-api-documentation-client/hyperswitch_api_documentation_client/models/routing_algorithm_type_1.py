from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.routing_algorithm_type_1_type import RoutingAlgorithmType1Type

if TYPE_CHECKING:
    from ..models.routable_connector_choice import RoutableConnectorChoice


T = TypeVar("T", bound="RoutingAlgorithmType1")


@_attrs_define
class RoutingAlgorithmType1:
    """
    Attributes:
        type_ (RoutingAlgorithmType1Type):
        data (list['RoutableConnectorChoice']):
    """

    type_: RoutingAlgorithmType1Type
    data: list["RoutableConnectorChoice"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        type_ = self.type_.value

        data = []
        for data_item_data in self.data:
            data_item = data_item_data.to_dict()
            data.append(data_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "type": type_,
                "data": data,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.routable_connector_choice import RoutableConnectorChoice

        d = dict(src_dict)
        type_ = RoutingAlgorithmType1Type(d.pop("type"))

        data = []
        _data = d.pop("data")
        for data_item_data in _data:
            data_item = RoutableConnectorChoice.from_dict(data_item_data)

            data.append(data_item)

        routing_algorithm_type_1 = cls(
            type_=type_,
            data=data,
        )

        routing_algorithm_type_1.additional_properties = d
        return routing_algorithm_type_1

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
