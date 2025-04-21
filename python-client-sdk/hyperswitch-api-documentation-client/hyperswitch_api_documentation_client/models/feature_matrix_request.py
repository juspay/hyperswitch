from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.connector import Connector
from ..types import UNSET, Unset

T = TypeVar("T", bound="FeatureMatrixRequest")


@_attrs_define
class FeatureMatrixRequest:
    """
    Attributes:
        connectors (Union[None, Unset, list[Connector]]):
    """

    connectors: Union[None, Unset, list[Connector]] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        connectors: Union[None, Unset, list[str]]
        if isinstance(self.connectors, Unset):
            connectors = UNSET
        elif isinstance(self.connectors, list):
            connectors = []
            for connectors_type_0_item_data in self.connectors:
                connectors_type_0_item = connectors_type_0_item_data.value
                connectors.append(connectors_type_0_item)

        else:
            connectors = self.connectors

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if connectors is not UNSET:
            field_dict["connectors"] = connectors

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_connectors(data: object) -> Union[None, Unset, list[Connector]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                connectors_type_0 = []
                _connectors_type_0 = data
                for connectors_type_0_item_data in _connectors_type_0:
                    connectors_type_0_item = Connector(connectors_type_0_item_data)

                    connectors_type_0.append(connectors_type_0_item)

                return connectors_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[Connector]], data)

        connectors = _parse_connectors(d.pop("connectors", UNSET))

        feature_matrix_request = cls(
            connectors=connectors,
        )

        feature_matrix_request.additional_properties = d
        return feature_matrix_request

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
