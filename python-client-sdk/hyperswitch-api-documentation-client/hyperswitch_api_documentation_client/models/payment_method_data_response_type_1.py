from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.bank_transfer_response import BankTransferResponse


T = TypeVar("T", bound="PaymentMethodDataResponseType1")


@_attrs_define
class PaymentMethodDataResponseType1:
    """
    Attributes:
        bank_transfer (BankTransferResponse):
    """

    bank_transfer: "BankTransferResponse"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        bank_transfer = self.bank_transfer.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "bank_transfer": bank_transfer,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.bank_transfer_response import BankTransferResponse

        d = dict(src_dict)
        bank_transfer = BankTransferResponse.from_dict(d.pop("bank_transfer"))

        payment_method_data_response_type_1 = cls(
            bank_transfer=bank_transfer,
        )

        payment_method_data_response_type_1.additional_properties = d
        return payment_method_data_response_type_1

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
