from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.single_type import SingleType

if TYPE_CHECKING:
    from ..models.routable_connector_choice import RoutableConnectorChoice


T = TypeVar("T", bound="Single")


@_attrs_define
class Single:
    """
    Attributes:
        type_ (SingleType):
        data (RoutableConnectorChoice): Routable Connector chosen for a payment
    """

    type_: SingleType
    data: "RoutableConnectorChoice"
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
        from ..models.routable_connector_choice import RoutableConnectorChoice

        d = dict(src_dict)
        type_ = SingleType(d.pop("type"))

        data = RoutableConnectorChoice.from_dict(d.pop("data"))

        single = cls(
            type_=type_,
            data=data,
        )

        single.additional_properties = d
        return single

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
