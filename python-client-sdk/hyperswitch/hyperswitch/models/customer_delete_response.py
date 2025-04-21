from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="CustomerDeleteResponse")


@_attrs_define
class CustomerDeleteResponse:
    """
    Attributes:
        customer_id (str): The identifier for the customer object Example: cus_y3oqhf46pyzuxjbcn2giaqnb44.
        customer_deleted (bool): Whether customer was deleted or not
        address_deleted (bool): Whether address was deleted or not
        payment_methods_deleted (bool): Whether payment methods deleted or not
    """

    customer_id: str
    customer_deleted: bool
    address_deleted: bool
    payment_methods_deleted: bool
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        customer_id = self.customer_id

        customer_deleted = self.customer_deleted

        address_deleted = self.address_deleted

        payment_methods_deleted = self.payment_methods_deleted

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "customer_id": customer_id,
                "customer_deleted": customer_deleted,
                "address_deleted": address_deleted,
                "payment_methods_deleted": payment_methods_deleted,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        customer_id = d.pop("customer_id")

        customer_deleted = d.pop("customer_deleted")

        address_deleted = d.pop("address_deleted")

        payment_methods_deleted = d.pop("payment_methods_deleted")

        customer_delete_response = cls(
            customer_id=customer_id,
            customer_deleted=customer_deleted,
            address_deleted=address_deleted,
            payment_methods_deleted=payment_methods_deleted,
        )

        customer_delete_response.additional_properties = d
        return customer_delete_response

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
