from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.bank_debit_response import BankDebitResponse


T = TypeVar("T", bound="PaymentMethodDataResponseType6")


@_attrs_define
class PaymentMethodDataResponseType6:
    """
    Attributes:
        bank_debit (BankDebitResponse):
    """

    bank_debit: "BankDebitResponse"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        bank_debit = self.bank_debit.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "bank_debit": bank_debit,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.bank_debit_response import BankDebitResponse

        d = dict(src_dict)
        bank_debit = BankDebitResponse.from_dict(d.pop("bank_debit"))

        payment_method_data_response_type_6 = cls(
            bank_debit=bank_debit,
        )

        payment_method_data_response_type_6.additional_properties = d
        return payment_method_data_response_type_6

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
