from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.surcharge_percentage import SurchargePercentage
    from ..models.surcharge_response_type_0 import SurchargeResponseType0
    from ..models.surcharge_response_type_1 import SurchargeResponseType1


T = TypeVar("T", bound="SurchargeDetailsResponse")


@_attrs_define
class SurchargeDetailsResponse:
    """
    Attributes:
        surcharge (Union['SurchargeResponseType0', 'SurchargeResponseType1']):
        display_surcharge_amount (float): surcharge amount for this payment
        display_tax_on_surcharge_amount (float): tax on surcharge amount for this payment
        display_total_surcharge_amount (float): sum of display_surcharge_amount and display_tax_on_surcharge_amount
        tax_on_surcharge (Union['SurchargePercentage', None, Unset]):
    """

    surcharge: Union["SurchargeResponseType0", "SurchargeResponseType1"]
    display_surcharge_amount: float
    display_tax_on_surcharge_amount: float
    display_total_surcharge_amount: float
    tax_on_surcharge: Union["SurchargePercentage", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.surcharge_percentage import SurchargePercentage
        from ..models.surcharge_response_type_0 import SurchargeResponseType0

        surcharge: dict[str, Any]
        if isinstance(self.surcharge, SurchargeResponseType0):
            surcharge = self.surcharge.to_dict()
        else:
            surcharge = self.surcharge.to_dict()

        display_surcharge_amount = self.display_surcharge_amount

        display_tax_on_surcharge_amount = self.display_tax_on_surcharge_amount

        display_total_surcharge_amount = self.display_total_surcharge_amount

        tax_on_surcharge: Union[None, Unset, dict[str, Any]]
        if isinstance(self.tax_on_surcharge, Unset):
            tax_on_surcharge = UNSET
        elif isinstance(self.tax_on_surcharge, SurchargePercentage):
            tax_on_surcharge = self.tax_on_surcharge.to_dict()
        else:
            tax_on_surcharge = self.tax_on_surcharge

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "surcharge": surcharge,
                "display_surcharge_amount": display_surcharge_amount,
                "display_tax_on_surcharge_amount": display_tax_on_surcharge_amount,
                "display_total_surcharge_amount": display_total_surcharge_amount,
            }
        )
        if tax_on_surcharge is not UNSET:
            field_dict["tax_on_surcharge"] = tax_on_surcharge

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.surcharge_percentage import SurchargePercentage
        from ..models.surcharge_response_type_0 import SurchargeResponseType0
        from ..models.surcharge_response_type_1 import SurchargeResponseType1

        d = dict(src_dict)

        def _parse_surcharge(data: object) -> Union["SurchargeResponseType0", "SurchargeResponseType1"]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_surcharge_response_type_0 = SurchargeResponseType0.from_dict(data)

                return componentsschemas_surcharge_response_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_surcharge_response_type_1 = SurchargeResponseType1.from_dict(data)

            return componentsschemas_surcharge_response_type_1

        surcharge = _parse_surcharge(d.pop("surcharge"))

        display_surcharge_amount = d.pop("display_surcharge_amount")

        display_tax_on_surcharge_amount = d.pop("display_tax_on_surcharge_amount")

        display_total_surcharge_amount = d.pop("display_total_surcharge_amount")

        def _parse_tax_on_surcharge(data: object) -> Union["SurchargePercentage", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                tax_on_surcharge_type_1 = SurchargePercentage.from_dict(data)

                return tax_on_surcharge_type_1
            except:  # noqa: E722
                pass
            return cast(Union["SurchargePercentage", None, Unset], data)

        tax_on_surcharge = _parse_tax_on_surcharge(d.pop("tax_on_surcharge", UNSET))

        surcharge_details_response = cls(
            surcharge=surcharge,
            display_surcharge_amount=display_surcharge_amount,
            display_tax_on_surcharge_amount=display_tax_on_surcharge_amount,
            display_total_surcharge_amount=display_total_surcharge_amount,
            tax_on_surcharge=tax_on_surcharge,
        )

        surcharge_details_response.additional_properties = d
        return surcharge_details_response

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
