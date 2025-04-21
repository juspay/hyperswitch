from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.xendit_charge_response_data_type_0 import XenditChargeResponseDataType0
    from ..models.xendit_charge_response_data_type_1 import XenditChargeResponseDataType1


T = TypeVar("T", bound="ConnectorChargeResponseDataType2")


@_attrs_define
class ConnectorChargeResponseDataType2:
    """
    Attributes:
        xendit_split_payment (Union['XenditChargeResponseDataType0', 'XenditChargeResponseDataType1']): Charge
            Information
    """

    xendit_split_payment: Union["XenditChargeResponseDataType0", "XenditChargeResponseDataType1"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.xendit_charge_response_data_type_0 import XenditChargeResponseDataType0

        xendit_split_payment: dict[str, Any]
        if isinstance(self.xendit_split_payment, XenditChargeResponseDataType0):
            xendit_split_payment = self.xendit_split_payment.to_dict()
        else:
            xendit_split_payment = self.xendit_split_payment.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "xendit_split_payment": xendit_split_payment,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.xendit_charge_response_data_type_0 import XenditChargeResponseDataType0
        from ..models.xendit_charge_response_data_type_1 import XenditChargeResponseDataType1

        d = dict(src_dict)

        def _parse_xendit_split_payment(
            data: object,
        ) -> Union["XenditChargeResponseDataType0", "XenditChargeResponseDataType1"]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_xendit_charge_response_data_type_0 = XenditChargeResponseDataType0.from_dict(data)

                return componentsschemas_xendit_charge_response_data_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_xendit_charge_response_data_type_1 = XenditChargeResponseDataType1.from_dict(data)

            return componentsschemas_xendit_charge_response_data_type_1

        xendit_split_payment = _parse_xendit_split_payment(d.pop("xendit_split_payment"))

        connector_charge_response_data_type_2 = cls(
            xendit_split_payment=xendit_split_payment,
        )

        connector_charge_response_data_type_2.additional_properties = d
        return connector_charge_response_data_type_2

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
