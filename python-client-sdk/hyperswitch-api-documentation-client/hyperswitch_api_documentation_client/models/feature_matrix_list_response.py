from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.connector_feature_matrix_response import ConnectorFeatureMatrixResponse


T = TypeVar("T", bound="FeatureMatrixListResponse")


@_attrs_define
class FeatureMatrixListResponse:
    """
    Attributes:
        connector_count (int): The number of connectors included in the response
        connectors (list['ConnectorFeatureMatrixResponse']):
    """

    connector_count: int
    connectors: list["ConnectorFeatureMatrixResponse"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        connector_count = self.connector_count

        connectors = []
        for connectors_item_data in self.connectors:
            connectors_item = connectors_item_data.to_dict()
            connectors.append(connectors_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "connector_count": connector_count,
                "connectors": connectors,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.connector_feature_matrix_response import ConnectorFeatureMatrixResponse

        d = dict(src_dict)
        connector_count = d.pop("connector_count")

        connectors = []
        _connectors = d.pop("connectors")
        for connectors_item_data in _connectors:
            connectors_item = ConnectorFeatureMatrixResponse.from_dict(connectors_item_data)

            connectors.append(connectors_item)

        feature_matrix_list_response = cls(
            connector_count=connector_count,
            connectors=connectors,
        )

        feature_matrix_list_response.additional_properties = d
        return feature_matrix_list_response

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
