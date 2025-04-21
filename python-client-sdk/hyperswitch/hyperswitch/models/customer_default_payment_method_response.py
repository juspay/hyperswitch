from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.payment_method import PaymentMethod
from ..models.payment_method_type import PaymentMethodType
from ..types import UNSET, Unset

T = TypeVar("T", bound="CustomerDefaultPaymentMethodResponse")


@_attrs_define
class CustomerDefaultPaymentMethodResponse:
    """
    Attributes:
        customer_id (str): The unique identifier of the customer. Example: cus_y3oqhf46pyzuxjbcn2giaqnb44.
        payment_method (PaymentMethod): Indicates the type of payment method. Eg: 'card', 'wallet', etc.
        default_payment_method_id (Union[None, Unset, str]): The unique identifier of the Payment method Example:
            card_rGK4Vi5iSW70MY7J2mIg.
        payment_method_type (Union[None, PaymentMethodType, Unset]):
    """

    customer_id: str
    payment_method: PaymentMethod
    default_payment_method_id: Union[None, Unset, str] = UNSET
    payment_method_type: Union[None, PaymentMethodType, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        customer_id = self.customer_id

        payment_method = self.payment_method.value

        default_payment_method_id: Union[None, Unset, str]
        if isinstance(self.default_payment_method_id, Unset):
            default_payment_method_id = UNSET
        else:
            default_payment_method_id = self.default_payment_method_id

        payment_method_type: Union[None, Unset, str]
        if isinstance(self.payment_method_type, Unset):
            payment_method_type = UNSET
        elif isinstance(self.payment_method_type, PaymentMethodType):
            payment_method_type = self.payment_method_type.value
        else:
            payment_method_type = self.payment_method_type

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "customer_id": customer_id,
                "payment_method": payment_method,
            }
        )
        if default_payment_method_id is not UNSET:
            field_dict["default_payment_method_id"] = default_payment_method_id
        if payment_method_type is not UNSET:
            field_dict["payment_method_type"] = payment_method_type

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        customer_id = d.pop("customer_id")

        payment_method = PaymentMethod(d.pop("payment_method"))

        def _parse_default_payment_method_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        default_payment_method_id = _parse_default_payment_method_id(d.pop("default_payment_method_id", UNSET))

        def _parse_payment_method_type(data: object) -> Union[None, PaymentMethodType, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payment_method_type_type_1 = PaymentMethodType(data)

                return payment_method_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentMethodType, Unset], data)

        payment_method_type = _parse_payment_method_type(d.pop("payment_method_type", UNSET))

        customer_default_payment_method_response = cls(
            customer_id=customer_id,
            payment_method=payment_method,
            default_payment_method_id=default_payment_method_id,
            payment_method_type=payment_method_type,
        )

        customer_default_payment_method_response.additional_properties = d
        return customer_default_payment_method_response

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
