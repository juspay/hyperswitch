from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="MerchantConnectorDeleteResponse")


@_attrs_define
class MerchantConnectorDeleteResponse:
    """
    Attributes:
        merchant_id (str): The identifier for the Merchant Account Example: y3oqhf46pyzuxjbcn2giaqnb44.
        merchant_connector_id (str): Unique ID of the connector Example: mca_5apGeP94tMts6rg3U3kR.
        deleted (bool): If the connector is deleted or not
    """

    merchant_id: str
    merchant_connector_id: str
    deleted: bool
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        merchant_id = self.merchant_id

        merchant_connector_id = self.merchant_connector_id

        deleted = self.deleted

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "merchant_id": merchant_id,
                "merchant_connector_id": merchant_connector_id,
                "deleted": deleted,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        merchant_id = d.pop("merchant_id")

        merchant_connector_id = d.pop("merchant_connector_id")

        deleted = d.pop("deleted")

        merchant_connector_delete_response = cls(
            merchant_id=merchant_id,
            merchant_connector_id=merchant_connector_id,
            deleted=deleted,
        )

        merchant_connector_delete_response.additional_properties = d
        return merchant_connector_delete_response

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
