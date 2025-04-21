from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..models.payment_method import PaymentMethod
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.request_payment_method_types import RequestPaymentMethodTypes


T = TypeVar("T", bound="PaymentMethodsEnabled")


@_attrs_define
class PaymentMethodsEnabled:
    """Details of all the payment methods enabled for the connector for the given merchant account

    Attributes:
        payment_method (PaymentMethod): Indicates the type of payment method. Eg: 'card', 'wallet', etc.
        payment_method_types (Union[None, Unset, list['RequestPaymentMethodTypes']]): Subtype of payment method Example:
            ['credit'].
    """

    payment_method: PaymentMethod
    payment_method_types: Union[None, Unset, list["RequestPaymentMethodTypes"]] = UNSET

    def to_dict(self) -> dict[str, Any]:
        payment_method = self.payment_method.value

        payment_method_types: Union[None, Unset, list[dict[str, Any]]]
        if isinstance(self.payment_method_types, Unset):
            payment_method_types = UNSET
        elif isinstance(self.payment_method_types, list):
            payment_method_types = []
            for payment_method_types_type_0_item_data in self.payment_method_types:
                payment_method_types_type_0_item = payment_method_types_type_0_item_data.to_dict()
                payment_method_types.append(payment_method_types_type_0_item)

        else:
            payment_method_types = self.payment_method_types

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "payment_method": payment_method,
            }
        )
        if payment_method_types is not UNSET:
            field_dict["payment_method_types"] = payment_method_types

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.request_payment_method_types import RequestPaymentMethodTypes

        d = dict(src_dict)
        payment_method = PaymentMethod(d.pop("payment_method"))

        def _parse_payment_method_types(data: object) -> Union[None, Unset, list["RequestPaymentMethodTypes"]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                payment_method_types_type_0 = []
                _payment_method_types_type_0 = data
                for payment_method_types_type_0_item_data in _payment_method_types_type_0:
                    payment_method_types_type_0_item = RequestPaymentMethodTypes.from_dict(
                        payment_method_types_type_0_item_data
                    )

                    payment_method_types_type_0.append(payment_method_types_type_0_item)

                return payment_method_types_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list["RequestPaymentMethodTypes"]], data)

        payment_method_types = _parse_payment_method_types(d.pop("payment_method_types", UNSET))

        payment_methods_enabled = cls(
            payment_method=payment_method,
            payment_method_types=payment_method_types,
        )

        return payment_methods_enabled
