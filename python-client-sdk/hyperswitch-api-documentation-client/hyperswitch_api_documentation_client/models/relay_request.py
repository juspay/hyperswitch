from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.relay_type import RelayType
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.relay_data_type_0 import RelayDataType0


T = TypeVar("T", bound="RelayRequest")


@_attrs_define
class RelayRequest:
    """
    Attributes:
        connector_resource_id (str): The identifier that is associated to a resource at the connector reference to which
            the relay request is being made Example: 7256228702616471803954.
        connector_id (str): Identifier of the connector ( merchant connector account ) which was chosen to make the
            payment Example: mca_5apGeP94tMts6rg3U3kR.
        type_ (RelayType):
        data (Union['RelayDataType0', None, Unset]):
    """

    connector_resource_id: str
    connector_id: str
    type_: RelayType
    data: Union["RelayDataType0", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.relay_data_type_0 import RelayDataType0

        connector_resource_id = self.connector_resource_id

        connector_id = self.connector_id

        type_ = self.type_.value

        data: Union[None, Unset, dict[str, Any]]
        if isinstance(self.data, Unset):
            data = UNSET
        elif isinstance(self.data, RelayDataType0):
            data = self.data.to_dict()
        else:
            data = self.data

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "connector_resource_id": connector_resource_id,
                "connector_id": connector_id,
                "type": type_,
            }
        )
        if data is not UNSET:
            field_dict["data"] = data

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.relay_data_type_0 import RelayDataType0

        d = dict(src_dict)
        connector_resource_id = d.pop("connector_resource_id")

        connector_id = d.pop("connector_id")

        type_ = RelayType(d.pop("type"))

        def _parse_data(data: object) -> Union["RelayDataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_relay_data_type_0 = RelayDataType0.from_dict(data)

                return componentsschemas_relay_data_type_0
            except:  # noqa: E722
                pass
            return cast(Union["RelayDataType0", None, Unset], data)

        data = _parse_data(d.pop("data", UNSET))

        relay_request = cls(
            connector_resource_id=connector_resource_id,
            connector_id=connector_id,
            type_=type_,
            data=data,
        )

        relay_request.additional_properties = d
        return relay_request

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
