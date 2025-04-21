from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="MobilePaymentDataType0DirectCarrierBilling")


@_attrs_define
class MobilePaymentDataType0DirectCarrierBilling:
    """
    Attributes:
        msisdn (str): The phone number of the user Example: 1234567890.
        client_uid (Union[None, Unset, str]): Unique user id Example: 02iacdYXGI9CnyJdoN8c7.
    """

    msisdn: str
    client_uid: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        msisdn = self.msisdn

        client_uid: Union[None, Unset, str]
        if isinstance(self.client_uid, Unset):
            client_uid = UNSET
        else:
            client_uid = self.client_uid

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "msisdn": msisdn,
            }
        )
        if client_uid is not UNSET:
            field_dict["client_uid"] = client_uid

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        msisdn = d.pop("msisdn")

        def _parse_client_uid(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        client_uid = _parse_client_uid(d.pop("client_uid", UNSET))

        mobile_payment_data_type_0_direct_carrier_billing = cls(
            msisdn=msisdn,
            client_uid=client_uid,
        )

        mobile_payment_data_type_0_direct_carrier_billing.additional_properties = d
        return mobile_payment_data_type_0_direct_carrier_billing

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
