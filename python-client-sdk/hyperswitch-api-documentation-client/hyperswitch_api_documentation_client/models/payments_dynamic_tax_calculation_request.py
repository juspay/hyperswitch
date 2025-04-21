from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.payment_method_type import PaymentMethodType
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.address import Address


T = TypeVar("T", bound="PaymentsDynamicTaxCalculationRequest")


@_attrs_define
class PaymentsDynamicTaxCalculationRequest:
    """
    Attributes:
        shipping (Address):
        client_secret (str): Client Secret
        payment_method_type (PaymentMethodType): Indicates the sub type of payment method. Eg: 'google_pay' &
            'apple_pay' for wallets.
        session_id (Union[None, Unset, str]): Session Id
    """

    shipping: "Address"
    client_secret: str
    payment_method_type: PaymentMethodType
    session_id: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        shipping = self.shipping.to_dict()

        client_secret = self.client_secret

        payment_method_type = self.payment_method_type.value

        session_id: Union[None, Unset, str]
        if isinstance(self.session_id, Unset):
            session_id = UNSET
        else:
            session_id = self.session_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "shipping": shipping,
                "client_secret": client_secret,
                "payment_method_type": payment_method_type,
            }
        )
        if session_id is not UNSET:
            field_dict["session_id"] = session_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.address import Address

        d = dict(src_dict)
        shipping = Address.from_dict(d.pop("shipping"))

        client_secret = d.pop("client_secret")

        payment_method_type = PaymentMethodType(d.pop("payment_method_type"))

        def _parse_session_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        session_id = _parse_session_id(d.pop("session_id", UNSET))

        payments_dynamic_tax_calculation_request = cls(
            shipping=shipping,
            client_secret=client_secret,
            payment_method_type=payment_method_type,
            session_id=session_id,
        )

        payments_dynamic_tax_calculation_request.additional_properties = d
        return payments_dynamic_tax_calculation_request

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
