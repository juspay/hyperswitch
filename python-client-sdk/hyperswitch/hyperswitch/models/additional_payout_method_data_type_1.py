from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.ach_bank_transfer_additional_data import AchBankTransferAdditionalData
    from ..models.bacs_bank_transfer_additional_data import BacsBankTransferAdditionalData
    from ..models.pix_bank_transfer_additional_data import PixBankTransferAdditionalData
    from ..models.sepa_bank_transfer_additional_data import SepaBankTransferAdditionalData


T = TypeVar("T", bound="AdditionalPayoutMethodDataType1")


@_attrs_define
class AdditionalPayoutMethodDataType1:
    """
    Attributes:
        bank (Union['AchBankTransferAdditionalData', 'BacsBankTransferAdditionalData', 'PixBankTransferAdditionalData',
            'SepaBankTransferAdditionalData']): Masked payout method details for bank payout method
    """

    bank: Union[
        "AchBankTransferAdditionalData",
        "BacsBankTransferAdditionalData",
        "PixBankTransferAdditionalData",
        "SepaBankTransferAdditionalData",
    ]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.ach_bank_transfer_additional_data import AchBankTransferAdditionalData
        from ..models.bacs_bank_transfer_additional_data import BacsBankTransferAdditionalData
        from ..models.sepa_bank_transfer_additional_data import SepaBankTransferAdditionalData

        bank: dict[str, Any]
        if isinstance(self.bank, AchBankTransferAdditionalData):
            bank = self.bank.to_dict()
        elif isinstance(self.bank, BacsBankTransferAdditionalData):
            bank = self.bank.to_dict()
        elif isinstance(self.bank, SepaBankTransferAdditionalData):
            bank = self.bank.to_dict()
        else:
            bank = self.bank.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "Bank": bank,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.ach_bank_transfer_additional_data import AchBankTransferAdditionalData
        from ..models.bacs_bank_transfer_additional_data import BacsBankTransferAdditionalData
        from ..models.pix_bank_transfer_additional_data import PixBankTransferAdditionalData
        from ..models.sepa_bank_transfer_additional_data import SepaBankTransferAdditionalData

        d = dict(src_dict)

        def _parse_bank(
            data: object,
        ) -> Union[
            "AchBankTransferAdditionalData",
            "BacsBankTransferAdditionalData",
            "PixBankTransferAdditionalData",
            "SepaBankTransferAdditionalData",
        ]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_bank_additional_data_type_0 = AchBankTransferAdditionalData.from_dict(data)

                return componentsschemas_bank_additional_data_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_bank_additional_data_type_1 = BacsBankTransferAdditionalData.from_dict(data)

                return componentsschemas_bank_additional_data_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_bank_additional_data_type_2 = SepaBankTransferAdditionalData.from_dict(data)

                return componentsschemas_bank_additional_data_type_2
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_bank_additional_data_type_3 = PixBankTransferAdditionalData.from_dict(data)

            return componentsschemas_bank_additional_data_type_3

        bank = _parse_bank(d.pop("Bank"))

        additional_payout_method_data_type_1 = cls(
            bank=bank,
        )

        additional_payout_method_data_type_1.additional_properties = d
        return additional_payout_method_data_type_1

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
