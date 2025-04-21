from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.routing_algorithm_type_3_type import RoutingAlgorithmType3Type

if TYPE_CHECKING:
    from ..models.program_connector_selection import ProgramConnectorSelection


T = TypeVar("T", bound="RoutingAlgorithmType3")


@_attrs_define
class RoutingAlgorithmType3:
    """
    Attributes:
        type_ (RoutingAlgorithmType3Type):
        data (ProgramConnectorSelection): The program, having a default connector selection and
            a bunch of rules. Also can hold arbitrary metadata.
    """

    type_: RoutingAlgorithmType3Type
    data: "ProgramConnectorSelection"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        type_ = self.type_.value

        data = self.data.to_dict()

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
        from ..models.program_connector_selection import ProgramConnectorSelection

        d = dict(src_dict)
        type_ = RoutingAlgorithmType3Type(d.pop("type"))

        data = ProgramConnectorSelection.from_dict(d.pop("data"))

        routing_algorithm_type_3 = cls(
            type_=type_,
            data=data,
        )

        routing_algorithm_type_3.additional_properties = d
        return routing_algorithm_type_3

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
