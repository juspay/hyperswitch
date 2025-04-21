from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.secret_info_to_initiate_sdk import SecretInfoToInitiateSdk


T = TypeVar("T", bound="ThirdPartySdkSessionResponse")


@_attrs_define
class ThirdPartySdkSessionResponse:
    """
    Attributes:
        secrets (SecretInfoToInitiateSdk):
    """

    secrets: "SecretInfoToInitiateSdk"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        secrets = self.secrets.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "secrets": secrets,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.secret_info_to_initiate_sdk import SecretInfoToInitiateSdk

        d = dict(src_dict)
        secrets = SecretInfoToInitiateSdk.from_dict(d.pop("secrets"))

        third_party_sdk_session_response = cls(
            secrets=secrets,
        )

        third_party_sdk_session_response.additional_properties = d
        return third_party_sdk_session_response

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
