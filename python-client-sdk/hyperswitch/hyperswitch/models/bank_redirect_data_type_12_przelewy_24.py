from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.bank_names import BankNames
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.bank_redirect_billing import BankRedirectBilling


T = TypeVar("T", bound="BankRedirectDataType12Przelewy24")


@_attrs_define
class BankRedirectDataType12Przelewy24:
    """
    Attributes:
        bank_name (Union[BankNames, None, Unset]):
        billing_details (Union['BankRedirectBilling', None, Unset]):
    """

    bank_name: Union[BankNames, None, Unset] = UNSET
    billing_details: Union["BankRedirectBilling", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.bank_redirect_billing import BankRedirectBilling

        bank_name: Union[None, Unset, str]
        if isinstance(self.bank_name, Unset):
            bank_name = UNSET
        elif isinstance(self.bank_name, BankNames):
            bank_name = self.bank_name.value
        else:
            bank_name = self.bank_name

        billing_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.billing_details, Unset):
            billing_details = UNSET
        elif isinstance(self.billing_details, BankRedirectBilling):
            billing_details = self.billing_details.to_dict()
        else:
            billing_details = self.billing_details

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if bank_name is not UNSET:
            field_dict["bank_name"] = bank_name
        if billing_details is not UNSET:
            field_dict["billing_details"] = billing_details

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.bank_redirect_billing import BankRedirectBilling

        d = dict(src_dict)

        def _parse_bank_name(data: object) -> Union[BankNames, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                bank_name_type_1 = BankNames(data)

                return bank_name_type_1
            except:  # noqa: E722
                pass
            return cast(Union[BankNames, None, Unset], data)

        bank_name = _parse_bank_name(d.pop("bank_name", UNSET))

        def _parse_billing_details(data: object) -> Union["BankRedirectBilling", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                billing_details_type_1 = BankRedirectBilling.from_dict(data)

                return billing_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["BankRedirectBilling", None, Unset], data)

        billing_details = _parse_billing_details(d.pop("billing_details", UNSET))

        bank_redirect_data_type_12_przelewy_24 = cls(
            bank_name=bank_name,
            billing_details=billing_details,
        )

        bank_redirect_data_type_12_przelewy_24.additional_properties = d
        return bank_redirect_data_type_12_przelewy_24

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
