from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.ach_bank_transfer import AchBankTransfer
    from ..models.bacs_bank_transfer import BacsBankTransfer
    from ..models.pix_bank_transfer import PixBankTransfer
    from ..models.sepa_bank_transfer import SepaBankTransfer


T = TypeVar("T", bound="PayoutMethodDataType1")


@_attrs_define
class PayoutMethodDataType1:
    """
    Attributes:
        bank (Union['AchBankTransfer', 'BacsBankTransfer', 'PixBankTransfer', 'SepaBankTransfer']):
    """

    bank: Union["AchBankTransfer", "BacsBankTransfer", "PixBankTransfer", "SepaBankTransfer"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.ach_bank_transfer import AchBankTransfer
        from ..models.bacs_bank_transfer import BacsBankTransfer
        from ..models.sepa_bank_transfer import SepaBankTransfer

        bank: dict[str, Any]
        if isinstance(self.bank, AchBankTransfer):
            bank = self.bank.to_dict()
        elif isinstance(self.bank, BacsBankTransfer):
            bank = self.bank.to_dict()
        elif isinstance(self.bank, SepaBankTransfer):
            bank = self.bank.to_dict()
        else:
            bank = self.bank.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "bank": bank,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.ach_bank_transfer import AchBankTransfer
        from ..models.bacs_bank_transfer import BacsBankTransfer
        from ..models.pix_bank_transfer import PixBankTransfer
        from ..models.sepa_bank_transfer import SepaBankTransfer

        d = dict(src_dict)

        def _parse_bank(
            data: object,
        ) -> Union["AchBankTransfer", "BacsBankTransfer", "PixBankTransfer", "SepaBankTransfer"]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_bank_type_0 = AchBankTransfer.from_dict(data)

                return componentsschemas_bank_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_bank_type_1 = BacsBankTransfer.from_dict(data)

                return componentsschemas_bank_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_bank_type_2 = SepaBankTransfer.from_dict(data)

                return componentsschemas_bank_type_2
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_bank_type_3 = PixBankTransfer.from_dict(data)

            return componentsschemas_bank_type_3

        bank = _parse_bank(d.pop("bank"))

        payout_method_data_type_1 = cls(
            bank=bank,
        )

        payout_method_data_type_1.additional_properties = d
        return payout_method_data_type_1

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
