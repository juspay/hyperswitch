from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.device_channel import DeviceChannel
from ..models.three_ds_completion_indicator import ThreeDsCompletionIndicator
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.sdk_information import SdkInformation


T = TypeVar("T", bound="PaymentsExternalAuthenticationRequest")


@_attrs_define
class PaymentsExternalAuthenticationRequest:
    """
    Attributes:
        client_secret (str): Client Secret
        device_channel (DeviceChannel): Device Channel indicating whether request is coming from App or Browser
        threeds_method_comp_ind (ThreeDsCompletionIndicator):
        sdk_information (Union['SdkInformation', None, Unset]):
    """

    client_secret: str
    device_channel: DeviceChannel
    threeds_method_comp_ind: ThreeDsCompletionIndicator
    sdk_information: Union["SdkInformation", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.sdk_information import SdkInformation

        client_secret = self.client_secret

        device_channel = self.device_channel.value

        threeds_method_comp_ind = self.threeds_method_comp_ind.value

        sdk_information: Union[None, Unset, dict[str, Any]]
        if isinstance(self.sdk_information, Unset):
            sdk_information = UNSET
        elif isinstance(self.sdk_information, SdkInformation):
            sdk_information = self.sdk_information.to_dict()
        else:
            sdk_information = self.sdk_information

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "client_secret": client_secret,
                "device_channel": device_channel,
                "threeds_method_comp_ind": threeds_method_comp_ind,
            }
        )
        if sdk_information is not UNSET:
            field_dict["sdk_information"] = sdk_information

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.sdk_information import SdkInformation

        d = dict(src_dict)
        client_secret = d.pop("client_secret")

        device_channel = DeviceChannel(d.pop("device_channel"))

        threeds_method_comp_ind = ThreeDsCompletionIndicator(d.pop("threeds_method_comp_ind"))

        def _parse_sdk_information(data: object) -> Union["SdkInformation", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                sdk_information_type_1 = SdkInformation.from_dict(data)

                return sdk_information_type_1
            except:  # noqa: E722
                pass
            return cast(Union["SdkInformation", None, Unset], data)

        sdk_information = _parse_sdk_information(d.pop("sdk_information", UNSET))

        payments_external_authentication_request = cls(
            client_secret=client_secret,
            device_channel=device_channel,
            threeds_method_comp_ind=threeds_method_comp_ind,
            sdk_information=sdk_information,
        )

        payments_external_authentication_request.additional_properties = d
        return payments_external_authentication_request

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
