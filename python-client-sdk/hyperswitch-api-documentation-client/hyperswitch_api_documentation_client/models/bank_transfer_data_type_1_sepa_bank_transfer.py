from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.country_alpha_2 import CountryAlpha2
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.sepa_and_bacs_billing_details import SepaAndBacsBillingDetails


T = TypeVar("T", bound="BankTransferDataType1SepaBankTransfer")


@_attrs_define
class BankTransferDataType1SepaBankTransfer:
    """
    Attributes:
        country (CountryAlpha2):
        billing_details (Union['SepaAndBacsBillingDetails', None, Unset]):
    """

    country: CountryAlpha2
    billing_details: Union["SepaAndBacsBillingDetails", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.sepa_and_bacs_billing_details import SepaAndBacsBillingDetails

        country = self.country.value

        billing_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.billing_details, Unset):
            billing_details = UNSET
        elif isinstance(self.billing_details, SepaAndBacsBillingDetails):
            billing_details = self.billing_details.to_dict()
        else:
            billing_details = self.billing_details

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "country": country,
            }
        )
        if billing_details is not UNSET:
            field_dict["billing_details"] = billing_details

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.sepa_and_bacs_billing_details import SepaAndBacsBillingDetails

        d = dict(src_dict)
        country = CountryAlpha2(d.pop("country"))

        def _parse_billing_details(data: object) -> Union["SepaAndBacsBillingDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                billing_details_type_1 = SepaAndBacsBillingDetails.from_dict(data)

                return billing_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["SepaAndBacsBillingDetails", None, Unset], data)

        billing_details = _parse_billing_details(d.pop("billing_details", UNSET))

        bank_transfer_data_type_1_sepa_bank_transfer = cls(
            country=country,
            billing_details=billing_details,
        )

        bank_transfer_data_type_1_sepa_bank_transfer.additional_properties = d
        return bank_transfer_data_type_1_sepa_bank_transfer

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
