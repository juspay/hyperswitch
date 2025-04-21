from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.google_pay_assurance_details import GooglePayAssuranceDetails


T = TypeVar("T", bound="GooglePayPaymentMethodInfo")


@_attrs_define
class GooglePayPaymentMethodInfo:
    """
    Attributes:
        card_network (str): The name of the card network
        card_details (str): The details of the card
        assurance_details (Union['GooglePayAssuranceDetails', None, Unset]):
    """

    card_network: str
    card_details: str
    assurance_details: Union["GooglePayAssuranceDetails", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.google_pay_assurance_details import GooglePayAssuranceDetails

        card_network = self.card_network

        card_details = self.card_details

        assurance_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.assurance_details, Unset):
            assurance_details = UNSET
        elif isinstance(self.assurance_details, GooglePayAssuranceDetails):
            assurance_details = self.assurance_details.to_dict()
        else:
            assurance_details = self.assurance_details

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "card_network": card_network,
                "card_details": card_details,
            }
        )
        if assurance_details is not UNSET:
            field_dict["assurance_details"] = assurance_details

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.google_pay_assurance_details import GooglePayAssuranceDetails

        d = dict(src_dict)
        card_network = d.pop("card_network")

        card_details = d.pop("card_details")

        def _parse_assurance_details(data: object) -> Union["GooglePayAssuranceDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                assurance_details_type_1 = GooglePayAssuranceDetails.from_dict(data)

                return assurance_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["GooglePayAssuranceDetails", None, Unset], data)

        assurance_details = _parse_assurance_details(d.pop("assurance_details", UNSET))

        google_pay_payment_method_info = cls(
            card_network=card_network,
            card_details=card_details,
            assurance_details=assurance_details,
        )

        google_pay_payment_method_info.additional_properties = d
        return google_pay_payment_method_info

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
