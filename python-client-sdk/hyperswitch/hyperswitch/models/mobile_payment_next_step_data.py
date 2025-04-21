from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.mobile_payment_consent import MobilePaymentConsent

T = TypeVar("T", bound="MobilePaymentNextStepData")


@_attrs_define
class MobilePaymentNextStepData:
    """
    Attributes:
        consent_data_required (MobilePaymentConsent):
    """

    consent_data_required: MobilePaymentConsent
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        consent_data_required = self.consent_data_required.value

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "consent_data_required": consent_data_required,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        consent_data_required = MobilePaymentConsent(d.pop("consent_data_required"))

        mobile_payment_next_step_data = cls(
            consent_data_required=consent_data_required,
        )

        mobile_payment_next_step_data.additional_properties = d
        return mobile_payment_next_step_data

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
