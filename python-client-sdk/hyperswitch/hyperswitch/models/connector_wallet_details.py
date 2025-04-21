from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.connector_wallet_details_apple_pay_combined_type_0 import ConnectorWalletDetailsApplePayCombinedType0
    from ..models.connector_wallet_details_apple_pay_type_0 import ConnectorWalletDetailsApplePayType0
    from ..models.connector_wallet_details_google_pay_type_0 import ConnectorWalletDetailsGooglePayType0
    from ..models.connector_wallet_details_paze_type_0 import ConnectorWalletDetailsPazeType0
    from ..models.connector_wallet_details_samsung_pay_type_0 import ConnectorWalletDetailsSamsungPayType0


T = TypeVar("T", bound="ConnectorWalletDetails")


@_attrs_define
class ConnectorWalletDetails:
    """
    Attributes:
        apple_pay_combined (Union['ConnectorWalletDetailsApplePayCombinedType0', None, Unset]): This field contains the
            Apple Pay certificates and credentials for iOS and Web Apple Pay flow
        apple_pay (Union['ConnectorWalletDetailsApplePayType0', None, Unset]): This field is for our legacy Apple Pay
            flow that contains the Apple Pay certificates and credentials for only iOS Apple Pay flow
        samsung_pay (Union['ConnectorWalletDetailsSamsungPayType0', None, Unset]): This field contains the Samsung Pay
            certificates and credentials
        paze (Union['ConnectorWalletDetailsPazeType0', None, Unset]): This field contains the Paze certificates and
            credentials
        google_pay (Union['ConnectorWalletDetailsGooglePayType0', None, Unset]): This field contains the Google Pay
            certificates and credentials
    """

    apple_pay_combined: Union["ConnectorWalletDetailsApplePayCombinedType0", None, Unset] = UNSET
    apple_pay: Union["ConnectorWalletDetailsApplePayType0", None, Unset] = UNSET
    samsung_pay: Union["ConnectorWalletDetailsSamsungPayType0", None, Unset] = UNSET
    paze: Union["ConnectorWalletDetailsPazeType0", None, Unset] = UNSET
    google_pay: Union["ConnectorWalletDetailsGooglePayType0", None, Unset] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.connector_wallet_details_apple_pay_combined_type_0 import (
            ConnectorWalletDetailsApplePayCombinedType0,
        )
        from ..models.connector_wallet_details_apple_pay_type_0 import ConnectorWalletDetailsApplePayType0
        from ..models.connector_wallet_details_google_pay_type_0 import ConnectorWalletDetailsGooglePayType0
        from ..models.connector_wallet_details_paze_type_0 import ConnectorWalletDetailsPazeType0
        from ..models.connector_wallet_details_samsung_pay_type_0 import ConnectorWalletDetailsSamsungPayType0

        apple_pay_combined: Union[None, Unset, dict[str, Any]]
        if isinstance(self.apple_pay_combined, Unset):
            apple_pay_combined = UNSET
        elif isinstance(self.apple_pay_combined, ConnectorWalletDetailsApplePayCombinedType0):
            apple_pay_combined = self.apple_pay_combined.to_dict()
        else:
            apple_pay_combined = self.apple_pay_combined

        apple_pay: Union[None, Unset, dict[str, Any]]
        if isinstance(self.apple_pay, Unset):
            apple_pay = UNSET
        elif isinstance(self.apple_pay, ConnectorWalletDetailsApplePayType0):
            apple_pay = self.apple_pay.to_dict()
        else:
            apple_pay = self.apple_pay

        samsung_pay: Union[None, Unset, dict[str, Any]]
        if isinstance(self.samsung_pay, Unset):
            samsung_pay = UNSET
        elif isinstance(self.samsung_pay, ConnectorWalletDetailsSamsungPayType0):
            samsung_pay = self.samsung_pay.to_dict()
        else:
            samsung_pay = self.samsung_pay

        paze: Union[None, Unset, dict[str, Any]]
        if isinstance(self.paze, Unset):
            paze = UNSET
        elif isinstance(self.paze, ConnectorWalletDetailsPazeType0):
            paze = self.paze.to_dict()
        else:
            paze = self.paze

        google_pay: Union[None, Unset, dict[str, Any]]
        if isinstance(self.google_pay, Unset):
            google_pay = UNSET
        elif isinstance(self.google_pay, ConnectorWalletDetailsGooglePayType0):
            google_pay = self.google_pay.to_dict()
        else:
            google_pay = self.google_pay

        field_dict: dict[str, Any] = {}
        field_dict.update({})
        if apple_pay_combined is not UNSET:
            field_dict["apple_pay_combined"] = apple_pay_combined
        if apple_pay is not UNSET:
            field_dict["apple_pay"] = apple_pay
        if samsung_pay is not UNSET:
            field_dict["samsung_pay"] = samsung_pay
        if paze is not UNSET:
            field_dict["paze"] = paze
        if google_pay is not UNSET:
            field_dict["google_pay"] = google_pay

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.connector_wallet_details_apple_pay_combined_type_0 import (
            ConnectorWalletDetailsApplePayCombinedType0,
        )
        from ..models.connector_wallet_details_apple_pay_type_0 import ConnectorWalletDetailsApplePayType0
        from ..models.connector_wallet_details_google_pay_type_0 import ConnectorWalletDetailsGooglePayType0
        from ..models.connector_wallet_details_paze_type_0 import ConnectorWalletDetailsPazeType0
        from ..models.connector_wallet_details_samsung_pay_type_0 import ConnectorWalletDetailsSamsungPayType0

        d = dict(src_dict)

        def _parse_apple_pay_combined(
            data: object,
        ) -> Union["ConnectorWalletDetailsApplePayCombinedType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                apple_pay_combined_type_0 = ConnectorWalletDetailsApplePayCombinedType0.from_dict(data)

                return apple_pay_combined_type_0
            except:  # noqa: E722
                pass
            return cast(Union["ConnectorWalletDetailsApplePayCombinedType0", None, Unset], data)

        apple_pay_combined = _parse_apple_pay_combined(d.pop("apple_pay_combined", UNSET))

        def _parse_apple_pay(data: object) -> Union["ConnectorWalletDetailsApplePayType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                apple_pay_type_0 = ConnectorWalletDetailsApplePayType0.from_dict(data)

                return apple_pay_type_0
            except:  # noqa: E722
                pass
            return cast(Union["ConnectorWalletDetailsApplePayType0", None, Unset], data)

        apple_pay = _parse_apple_pay(d.pop("apple_pay", UNSET))

        def _parse_samsung_pay(data: object) -> Union["ConnectorWalletDetailsSamsungPayType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                samsung_pay_type_0 = ConnectorWalletDetailsSamsungPayType0.from_dict(data)

                return samsung_pay_type_0
            except:  # noqa: E722
                pass
            return cast(Union["ConnectorWalletDetailsSamsungPayType0", None, Unset], data)

        samsung_pay = _parse_samsung_pay(d.pop("samsung_pay", UNSET))

        def _parse_paze(data: object) -> Union["ConnectorWalletDetailsPazeType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                paze_type_0 = ConnectorWalletDetailsPazeType0.from_dict(data)

                return paze_type_0
            except:  # noqa: E722
                pass
            return cast(Union["ConnectorWalletDetailsPazeType0", None, Unset], data)

        paze = _parse_paze(d.pop("paze", UNSET))

        def _parse_google_pay(data: object) -> Union["ConnectorWalletDetailsGooglePayType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                google_pay_type_0 = ConnectorWalletDetailsGooglePayType0.from_dict(data)

                return google_pay_type_0
            except:  # noqa: E722
                pass
            return cast(Union["ConnectorWalletDetailsGooglePayType0", None, Unset], data)

        google_pay = _parse_google_pay(d.pop("google_pay", UNSET))

        connector_wallet_details = cls(
            apple_pay_combined=apple_pay_combined,
            apple_pay=apple_pay,
            samsung_pay=samsung_pay,
            paze=paze,
            google_pay=google_pay,
        )

        return connector_wallet_details
