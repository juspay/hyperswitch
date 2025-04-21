from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.merchant_connector_details import MerchantConnectorDetails


T = TypeVar("T", bound="MerchantConnectorDetailsWrap")


@_attrs_define
class MerchantConnectorDetailsWrap:
    """Merchant connector details used to make payments.

    Attributes:
        creds_identifier (str): Creds Identifier is to uniquely identify the credentials. Do not send any sensitive
            info, like encoded_data in this field. And do not send the string "null".
        encoded_data (Union['MerchantConnectorDetails', None, Unset]):
    """

    creds_identifier: str
    encoded_data: Union["MerchantConnectorDetails", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.merchant_connector_details import MerchantConnectorDetails

        creds_identifier = self.creds_identifier

        encoded_data: Union[None, Unset, dict[str, Any]]
        if isinstance(self.encoded_data, Unset):
            encoded_data = UNSET
        elif isinstance(self.encoded_data, MerchantConnectorDetails):
            encoded_data = self.encoded_data.to_dict()
        else:
            encoded_data = self.encoded_data

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "creds_identifier": creds_identifier,
            }
        )
        if encoded_data is not UNSET:
            field_dict["encoded_data"] = encoded_data

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.merchant_connector_details import MerchantConnectorDetails

        d = dict(src_dict)
        creds_identifier = d.pop("creds_identifier")

        def _parse_encoded_data(data: object) -> Union["MerchantConnectorDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                encoded_data_type_1 = MerchantConnectorDetails.from_dict(data)

                return encoded_data_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MerchantConnectorDetails", None, Unset], data)

        encoded_data = _parse_encoded_data(d.pop("encoded_data", UNSET))

        merchant_connector_details_wrap = cls(
            creds_identifier=creds_identifier,
            encoded_data=encoded_data,
        )

        merchant_connector_details_wrap.additional_properties = d
        return merchant_connector_details_wrap

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
