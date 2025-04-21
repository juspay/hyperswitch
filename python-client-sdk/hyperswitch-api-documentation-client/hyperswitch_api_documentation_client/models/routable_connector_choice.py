from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.routable_connectors import RoutableConnectors
from ..types import UNSET, Unset

T = TypeVar("T", bound="RoutableConnectorChoice")


@_attrs_define
class RoutableConnectorChoice:
    """Routable Connector chosen for a payment

    Attributes:
        connector (RoutableConnectors): RoutableConnectors are the subset of Connectors that are eligible for payments
            routing
        merchant_connector_id (Union[None, Unset, str]):
    """

    connector: RoutableConnectors
    merchant_connector_id: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        connector = self.connector.value

        merchant_connector_id: Union[None, Unset, str]
        if isinstance(self.merchant_connector_id, Unset):
            merchant_connector_id = UNSET
        else:
            merchant_connector_id = self.merchant_connector_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "connector": connector,
            }
        )
        if merchant_connector_id is not UNSET:
            field_dict["merchant_connector_id"] = merchant_connector_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        connector = RoutableConnectors(d.pop("connector"))

        def _parse_merchant_connector_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_connector_id = _parse_merchant_connector_id(d.pop("merchant_connector_id", UNSET))

        routable_connector_choice = cls(
            connector=connector,
            merchant_connector_id=merchant_connector_id,
        )

        routable_connector_choice.additional_properties = d
        return routable_connector_choice

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
