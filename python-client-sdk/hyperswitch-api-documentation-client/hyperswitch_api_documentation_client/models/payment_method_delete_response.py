from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="PaymentMethodDeleteResponse")


@_attrs_define
class PaymentMethodDeleteResponse:
    """
    Attributes:
        payment_method_id (str): The unique identifier of the Payment method Example: card_rGK4Vi5iSW70MY7J2mIg.
        deleted (bool): Whether payment method was deleted or not Example: True.
    """

    payment_method_id: str
    deleted: bool
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        payment_method_id = self.payment_method_id

        deleted = self.deleted

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payment_method_id": payment_method_id,
                "deleted": deleted,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        payment_method_id = d.pop("payment_method_id")

        deleted = d.pop("deleted")

        payment_method_delete_response = cls(
            payment_method_id=payment_method_id,
            deleted=deleted,
        )

        payment_method_delete_response.additional_properties = d
        return payment_method_delete_response

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
