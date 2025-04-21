from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.merchant_account_data_type_0 import MerchantAccountDataType0
    from ..models.merchant_account_data_type_1 import MerchantAccountDataType1


T = TypeVar("T", bound="MerchantRecipientDataType2")


@_attrs_define
class MerchantRecipientDataType2:
    """
    Attributes:
        account_data (Union['MerchantAccountDataType0', 'MerchantAccountDataType1']):
    """

    account_data: Union["MerchantAccountDataType0", "MerchantAccountDataType1"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.merchant_account_data_type_0 import MerchantAccountDataType0

        account_data: dict[str, Any]
        if isinstance(self.account_data, MerchantAccountDataType0):
            account_data = self.account_data.to_dict()
        else:
            account_data = self.account_data.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "account_data": account_data,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.merchant_account_data_type_0 import MerchantAccountDataType0
        from ..models.merchant_account_data_type_1 import MerchantAccountDataType1

        d = dict(src_dict)

        def _parse_account_data(data: object) -> Union["MerchantAccountDataType0", "MerchantAccountDataType1"]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_merchant_account_data_type_0 = MerchantAccountDataType0.from_dict(data)

                return componentsschemas_merchant_account_data_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_merchant_account_data_type_1 = MerchantAccountDataType1.from_dict(data)

            return componentsschemas_merchant_account_data_type_1

        account_data = _parse_account_data(d.pop("account_data"))

        merchant_recipient_data_type_2 = cls(
            account_data=account_data,
        )

        merchant_recipient_data_type_2.additional_properties = d
        return merchant_recipient_data_type_2

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
