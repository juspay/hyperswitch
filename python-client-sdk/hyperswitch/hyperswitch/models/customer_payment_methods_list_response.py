from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.customer_payment_method import CustomerPaymentMethod


T = TypeVar("T", bound="CustomerPaymentMethodsListResponse")


@_attrs_define
class CustomerPaymentMethodsListResponse:
    """
    Attributes:
        customer_payment_methods (list['CustomerPaymentMethod']): List of payment methods for customer
        is_guest_customer (Union[None, Unset, bool]): Returns whether a customer id is not tied to a payment intent
            (only when the request is made against a client secret)
    """

    customer_payment_methods: list["CustomerPaymentMethod"]
    is_guest_customer: Union[None, Unset, bool] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        customer_payment_methods = []
        for customer_payment_methods_item_data in self.customer_payment_methods:
            customer_payment_methods_item = customer_payment_methods_item_data.to_dict()
            customer_payment_methods.append(customer_payment_methods_item)

        is_guest_customer: Union[None, Unset, bool]
        if isinstance(self.is_guest_customer, Unset):
            is_guest_customer = UNSET
        else:
            is_guest_customer = self.is_guest_customer

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "customer_payment_methods": customer_payment_methods,
            }
        )
        if is_guest_customer is not UNSET:
            field_dict["is_guest_customer"] = is_guest_customer

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.customer_payment_method import CustomerPaymentMethod

        d = dict(src_dict)
        customer_payment_methods = []
        _customer_payment_methods = d.pop("customer_payment_methods")
        for customer_payment_methods_item_data in _customer_payment_methods:
            customer_payment_methods_item = CustomerPaymentMethod.from_dict(customer_payment_methods_item_data)

            customer_payment_methods.append(customer_payment_methods_item)

        def _parse_is_guest_customer(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        is_guest_customer = _parse_is_guest_customer(d.pop("is_guest_customer", UNSET))

        customer_payment_methods_list_response = cls(
            customer_payment_methods=customer_payment_methods,
            is_guest_customer=is_guest_customer,
        )

        customer_payment_methods_list_response.additional_properties = d
        return customer_payment_methods_list_response

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
