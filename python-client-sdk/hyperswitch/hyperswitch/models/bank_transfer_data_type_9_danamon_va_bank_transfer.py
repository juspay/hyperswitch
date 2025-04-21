from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.doku_billing_details import DokuBillingDetails


T = TypeVar("T", bound="BankTransferDataType9DanamonVaBankTransfer")


@_attrs_define
class BankTransferDataType9DanamonVaBankTransfer:
    """
    Attributes:
        billing_details (Union['DokuBillingDetails', None, Unset]):
    """

    billing_details: Union["DokuBillingDetails", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.doku_billing_details import DokuBillingDetails

        billing_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.billing_details, Unset):
            billing_details = UNSET
        elif isinstance(self.billing_details, DokuBillingDetails):
            billing_details = self.billing_details.to_dict()
        else:
            billing_details = self.billing_details

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if billing_details is not UNSET:
            field_dict["billing_details"] = billing_details

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.doku_billing_details import DokuBillingDetails

        d = dict(src_dict)

        def _parse_billing_details(data: object) -> Union["DokuBillingDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                billing_details_type_1 = DokuBillingDetails.from_dict(data)

                return billing_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["DokuBillingDetails", None, Unset], data)

        billing_details = _parse_billing_details(d.pop("billing_details", UNSET))

        bank_transfer_data_type_9_danamon_va_bank_transfer = cls(
            billing_details=billing_details,
        )

        bank_transfer_data_type_9_danamon_va_bank_transfer.additional_properties = d
        return bank_transfer_data_type_9_danamon_va_bank_transfer

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
