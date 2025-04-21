from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.merchant_recipient_data_type_0 import MerchantRecipientDataType0
    from ..models.merchant_recipient_data_type_1 import MerchantRecipientDataType1
    from ..models.merchant_recipient_data_type_2 import MerchantRecipientDataType2


T = TypeVar("T", bound="AdditionalMerchantDataType0")


@_attrs_define
class AdditionalMerchantDataType0:
    """
    Attributes:
        open_banking_recipient_data (Union['MerchantRecipientDataType0', 'MerchantRecipientDataType1',
            'MerchantRecipientDataType2']):
    """

    open_banking_recipient_data: Union[
        "MerchantRecipientDataType0", "MerchantRecipientDataType1", "MerchantRecipientDataType2"
    ]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.merchant_recipient_data_type_0 import MerchantRecipientDataType0
        from ..models.merchant_recipient_data_type_1 import MerchantRecipientDataType1

        open_banking_recipient_data: dict[str, Any]
        if isinstance(self.open_banking_recipient_data, MerchantRecipientDataType0):
            open_banking_recipient_data = self.open_banking_recipient_data.to_dict()
        elif isinstance(self.open_banking_recipient_data, MerchantRecipientDataType1):
            open_banking_recipient_data = self.open_banking_recipient_data.to_dict()
        else:
            open_banking_recipient_data = self.open_banking_recipient_data.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "open_banking_recipient_data": open_banking_recipient_data,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.merchant_recipient_data_type_0 import MerchantRecipientDataType0
        from ..models.merchant_recipient_data_type_1 import MerchantRecipientDataType1
        from ..models.merchant_recipient_data_type_2 import MerchantRecipientDataType2

        d = dict(src_dict)

        def _parse_open_banking_recipient_data(
            data: object,
        ) -> Union["MerchantRecipientDataType0", "MerchantRecipientDataType1", "MerchantRecipientDataType2"]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_merchant_recipient_data_type_0 = MerchantRecipientDataType0.from_dict(data)

                return componentsschemas_merchant_recipient_data_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_merchant_recipient_data_type_1 = MerchantRecipientDataType1.from_dict(data)

                return componentsschemas_merchant_recipient_data_type_1
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_merchant_recipient_data_type_2 = MerchantRecipientDataType2.from_dict(data)

            return componentsschemas_merchant_recipient_data_type_2

        open_banking_recipient_data = _parse_open_banking_recipient_data(d.pop("open_banking_recipient_data"))

        additional_merchant_data_type_0 = cls(
            open_banking_recipient_data=open_banking_recipient_data,
        )

        additional_merchant_data_type_0.additional_properties = d
        return additional_merchant_data_type_0

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
