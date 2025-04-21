from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.mandate_status import MandateStatus
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.customer_acceptance import CustomerAcceptance
    from ..models.mandate_card_details import MandateCardDetails


T = TypeVar("T", bound="MandateResponse")


@_attrs_define
class MandateResponse:
    """
    Attributes:
        mandate_id (str): The identifier for mandate
        status (MandateStatus): The status of the mandate, which indicates whether it can be used to initiate a payment.
        payment_method_id (str): The identifier for payment method
        payment_method (str): The payment method
        payment_method_type (Union[None, Unset, str]): The payment method type
        card (Union['MandateCardDetails', None, Unset]):
        customer_acceptance (Union['CustomerAcceptance', None, Unset]):
    """

    mandate_id: str
    status: MandateStatus
    payment_method_id: str
    payment_method: str
    payment_method_type: Union[None, Unset, str] = UNSET
    card: Union["MandateCardDetails", None, Unset] = UNSET
    customer_acceptance: Union["CustomerAcceptance", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.customer_acceptance import CustomerAcceptance
        from ..models.mandate_card_details import MandateCardDetails

        mandate_id = self.mandate_id

        status = self.status.value

        payment_method_id = self.payment_method_id

        payment_method = self.payment_method

        payment_method_type: Union[None, Unset, str]
        if isinstance(self.payment_method_type, Unset):
            payment_method_type = UNSET
        else:
            payment_method_type = self.payment_method_type

        card: Union[None, Unset, dict[str, Any]]
        if isinstance(self.card, Unset):
            card = UNSET
        elif isinstance(self.card, MandateCardDetails):
            card = self.card.to_dict()
        else:
            card = self.card

        customer_acceptance: Union[None, Unset, dict[str, Any]]
        if isinstance(self.customer_acceptance, Unset):
            customer_acceptance = UNSET
        elif isinstance(self.customer_acceptance, CustomerAcceptance):
            customer_acceptance = self.customer_acceptance.to_dict()
        else:
            customer_acceptance = self.customer_acceptance

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "mandate_id": mandate_id,
                "status": status,
                "payment_method_id": payment_method_id,
                "payment_method": payment_method,
            }
        )
        if payment_method_type is not UNSET:
            field_dict["payment_method_type"] = payment_method_type
        if card is not UNSET:
            field_dict["card"] = card
        if customer_acceptance is not UNSET:
            field_dict["customer_acceptance"] = customer_acceptance

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.customer_acceptance import CustomerAcceptance
        from ..models.mandate_card_details import MandateCardDetails

        d = dict(src_dict)
        mandate_id = d.pop("mandate_id")

        status = MandateStatus(d.pop("status"))

        payment_method_id = d.pop("payment_method_id")

        payment_method = d.pop("payment_method")

        def _parse_payment_method_type(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_method_type = _parse_payment_method_type(d.pop("payment_method_type", UNSET))

        def _parse_card(data: object) -> Union["MandateCardDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                card_type_1 = MandateCardDetails.from_dict(data)

                return card_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MandateCardDetails", None, Unset], data)

        card = _parse_card(d.pop("card", UNSET))

        def _parse_customer_acceptance(data: object) -> Union["CustomerAcceptance", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                customer_acceptance_type_1 = CustomerAcceptance.from_dict(data)

                return customer_acceptance_type_1
            except:  # noqa: E722
                pass
            return cast(Union["CustomerAcceptance", None, Unset], data)

        customer_acceptance = _parse_customer_acceptance(d.pop("customer_acceptance", UNSET))

        mandate_response = cls(
            mandate_id=mandate_id,
            status=status,
            payment_method_id=payment_method_id,
            payment_method=payment_method,
            payment_method_type=payment_method_type,
            card=card,
            customer_acceptance=customer_acceptance,
        )

        mandate_response.additional_properties = d
        return mandate_response

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
