from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..models.merchant_product_type import MerchantProductType
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.business_collect_link_config import BusinessCollectLinkConfig
    from ..models.merchant_account_create_frm_routing_algorithm_type_0 import (
        MerchantAccountCreateFrmRoutingAlgorithmType0,
    )
    from ..models.merchant_account_create_metadata_type_0 import MerchantAccountCreateMetadataType0
    from ..models.merchant_details import MerchantDetails
    from ..models.primary_business_details import PrimaryBusinessDetails
    from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
    from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
    from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2
    from ..models.routing_algorithm_type_3 import RoutingAlgorithmType3
    from ..models.webhook_details import WebhookDetails


T = TypeVar("T", bound="MerchantAccountCreate")


@_attrs_define
class MerchantAccountCreate:
    """
    Attributes:
        merchant_id (str): The identifier for the Merchant Account Example: y3oqhf46pyzuxjbcn2giaqnb44.
        merchant_name (Union[None, Unset, str]): Name of the Merchant Account Example: NewAge Retailer.
        merchant_details (Union['MerchantDetails', None, Unset]):
        return_url (Union[None, Unset, str]): The URL to redirect after the completion of the operation Example:
            https://www.example.com/success.
        webhook_details (Union['WebhookDetails', None, Unset]):
        payout_routing_algorithm (Union['RoutingAlgorithmType0', 'RoutingAlgorithmType1', 'RoutingAlgorithmType2',
            'RoutingAlgorithmType3', None, Unset]):
        sub_merchants_enabled (Union[None, Unset, bool]): A boolean value to indicate if the merchant is a sub-merchant
            under a master or a parent merchant. By default, its value is false. Default: False.
        parent_merchant_id (Union[None, Unset, str]): Refers to the Parent Merchant ID if the merchant being created is
            a sub-merchant Example: xkkdf909012sdjki2dkh5sdf.
        enable_payment_response_hash (Union[None, Unset, bool]): A boolean value to indicate if payment response hash
            needs to be enabled Default: False. Example: True.
        payment_response_hash_key (Union[None, Unset, str]): Refers to the hash key used for calculating the signature
            for webhooks and redirect response. If the value is not provided, a value is automatically generated.
        redirect_to_merchant_with_http_post (Union[None, Unset, bool]): A boolean value to indicate if redirect to
            merchant with http post needs to be enabled. Default: False. Example: True.
        metadata (Union['MerchantAccountCreateMetadataType0', None, Unset]): Metadata is useful for storing additional,
            unstructured information on an object
        publishable_key (Union[None, Unset, str]): API key that will be used for client side API access. A publishable
            key has to be always paired with a `client_secret`.
            A `client_secret` can be obtained by creating a payment with `confirm` set to false Example: AH3423bkjbkjdsfbkj.
        locker_id (Union[None, Unset, str]): An identifier for the vault used to store payment method information.
            Example: locker_abc123.
        primary_business_details (Union['PrimaryBusinessDetails', None, Unset]):
        frm_routing_algorithm (Union['MerchantAccountCreateFrmRoutingAlgorithmType0', None, Unset]): The frm routing
            algorithm to be used for routing payments to desired FRM's
        organization_id (Union[None, Unset, str]): The id of the organization to which the merchant belongs to, if not
            passed an organization is created Example: org_q98uSGAYbjEwqs0mJwnz.
        pm_collect_link_config (Union['BusinessCollectLinkConfig', None, Unset]):
        product_type (Union[MerchantProductType, None, Unset]):
    """

    merchant_id: str
    merchant_name: Union[None, Unset, str] = UNSET
    merchant_details: Union["MerchantDetails", None, Unset] = UNSET
    return_url: Union[None, Unset, str] = UNSET
    webhook_details: Union["WebhookDetails", None, Unset] = UNSET
    payout_routing_algorithm: Union[
        "RoutingAlgorithmType0", "RoutingAlgorithmType1", "RoutingAlgorithmType2", "RoutingAlgorithmType3", None, Unset
    ] = UNSET
    sub_merchants_enabled: Union[None, Unset, bool] = False
    parent_merchant_id: Union[None, Unset, str] = UNSET
    enable_payment_response_hash: Union[None, Unset, bool] = False
    payment_response_hash_key: Union[None, Unset, str] = UNSET
    redirect_to_merchant_with_http_post: Union[None, Unset, bool] = False
    metadata: Union["MerchantAccountCreateMetadataType0", None, Unset] = UNSET
    publishable_key: Union[None, Unset, str] = UNSET
    locker_id: Union[None, Unset, str] = UNSET
    primary_business_details: Union["PrimaryBusinessDetails", None, Unset] = UNSET
    frm_routing_algorithm: Union["MerchantAccountCreateFrmRoutingAlgorithmType0", None, Unset] = UNSET
    organization_id: Union[None, Unset, str] = UNSET
    pm_collect_link_config: Union["BusinessCollectLinkConfig", None, Unset] = UNSET
    product_type: Union[MerchantProductType, None, Unset] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.business_collect_link_config import BusinessCollectLinkConfig
        from ..models.merchant_account_create_frm_routing_algorithm_type_0 import (
            MerchantAccountCreateFrmRoutingAlgorithmType0,
        )
        from ..models.merchant_account_create_metadata_type_0 import MerchantAccountCreateMetadataType0
        from ..models.merchant_details import MerchantDetails
        from ..models.primary_business_details import PrimaryBusinessDetails
        from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
        from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
        from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2
        from ..models.routing_algorithm_type_3 import RoutingAlgorithmType3
        from ..models.webhook_details import WebhookDetails

        merchant_id = self.merchant_id

        merchant_name: Union[None, Unset, str]
        if isinstance(self.merchant_name, Unset):
            merchant_name = UNSET
        else:
            merchant_name = self.merchant_name

        merchant_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.merchant_details, Unset):
            merchant_details = UNSET
        elif isinstance(self.merchant_details, MerchantDetails):
            merchant_details = self.merchant_details.to_dict()
        else:
            merchant_details = self.merchant_details

        return_url: Union[None, Unset, str]
        if isinstance(self.return_url, Unset):
            return_url = UNSET
        else:
            return_url = self.return_url

        webhook_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.webhook_details, Unset):
            webhook_details = UNSET
        elif isinstance(self.webhook_details, WebhookDetails):
            webhook_details = self.webhook_details.to_dict()
        else:
            webhook_details = self.webhook_details

        payout_routing_algorithm: Union[None, Unset, dict[str, Any]]
        if isinstance(self.payout_routing_algorithm, Unset):
            payout_routing_algorithm = UNSET
        elif isinstance(self.payout_routing_algorithm, RoutingAlgorithmType0):
            payout_routing_algorithm = self.payout_routing_algorithm.to_dict()
        elif isinstance(self.payout_routing_algorithm, RoutingAlgorithmType1):
            payout_routing_algorithm = self.payout_routing_algorithm.to_dict()
        elif isinstance(self.payout_routing_algorithm, RoutingAlgorithmType2):
            payout_routing_algorithm = self.payout_routing_algorithm.to_dict()
        elif isinstance(self.payout_routing_algorithm, RoutingAlgorithmType3):
            payout_routing_algorithm = self.payout_routing_algorithm.to_dict()
        else:
            payout_routing_algorithm = self.payout_routing_algorithm

        sub_merchants_enabled: Union[None, Unset, bool]
        if isinstance(self.sub_merchants_enabled, Unset):
            sub_merchants_enabled = UNSET
        else:
            sub_merchants_enabled = self.sub_merchants_enabled

        parent_merchant_id: Union[None, Unset, str]
        if isinstance(self.parent_merchant_id, Unset):
            parent_merchant_id = UNSET
        else:
            parent_merchant_id = self.parent_merchant_id

        enable_payment_response_hash: Union[None, Unset, bool]
        if isinstance(self.enable_payment_response_hash, Unset):
            enable_payment_response_hash = UNSET
        else:
            enable_payment_response_hash = self.enable_payment_response_hash

        payment_response_hash_key: Union[None, Unset, str]
        if isinstance(self.payment_response_hash_key, Unset):
            payment_response_hash_key = UNSET
        else:
            payment_response_hash_key = self.payment_response_hash_key

        redirect_to_merchant_with_http_post: Union[None, Unset, bool]
        if isinstance(self.redirect_to_merchant_with_http_post, Unset):
            redirect_to_merchant_with_http_post = UNSET
        else:
            redirect_to_merchant_with_http_post = self.redirect_to_merchant_with_http_post

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, MerchantAccountCreateMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        publishable_key: Union[None, Unset, str]
        if isinstance(self.publishable_key, Unset):
            publishable_key = UNSET
        else:
            publishable_key = self.publishable_key

        locker_id: Union[None, Unset, str]
        if isinstance(self.locker_id, Unset):
            locker_id = UNSET
        else:
            locker_id = self.locker_id

        primary_business_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.primary_business_details, Unset):
            primary_business_details = UNSET
        elif isinstance(self.primary_business_details, PrimaryBusinessDetails):
            primary_business_details = self.primary_business_details.to_dict()
        else:
            primary_business_details = self.primary_business_details

        frm_routing_algorithm: Union[None, Unset, dict[str, Any]]
        if isinstance(self.frm_routing_algorithm, Unset):
            frm_routing_algorithm = UNSET
        elif isinstance(self.frm_routing_algorithm, MerchantAccountCreateFrmRoutingAlgorithmType0):
            frm_routing_algorithm = self.frm_routing_algorithm.to_dict()
        else:
            frm_routing_algorithm = self.frm_routing_algorithm

        organization_id: Union[None, Unset, str]
        if isinstance(self.organization_id, Unset):
            organization_id = UNSET
        else:
            organization_id = self.organization_id

        pm_collect_link_config: Union[None, Unset, dict[str, Any]]
        if isinstance(self.pm_collect_link_config, Unset):
            pm_collect_link_config = UNSET
        elif isinstance(self.pm_collect_link_config, BusinessCollectLinkConfig):
            pm_collect_link_config = self.pm_collect_link_config.to_dict()
        else:
            pm_collect_link_config = self.pm_collect_link_config

        product_type: Union[None, Unset, str]
        if isinstance(self.product_type, Unset):
            product_type = UNSET
        elif isinstance(self.product_type, MerchantProductType):
            product_type = self.product_type.value
        else:
            product_type = self.product_type

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "merchant_id": merchant_id,
            }
        )
        if merchant_name is not UNSET:
            field_dict["merchant_name"] = merchant_name
        if merchant_details is not UNSET:
            field_dict["merchant_details"] = merchant_details
        if return_url is not UNSET:
            field_dict["return_url"] = return_url
        if webhook_details is not UNSET:
            field_dict["webhook_details"] = webhook_details
        if payout_routing_algorithm is not UNSET:
            field_dict["payout_routing_algorithm"] = payout_routing_algorithm
        if sub_merchants_enabled is not UNSET:
            field_dict["sub_merchants_enabled"] = sub_merchants_enabled
        if parent_merchant_id is not UNSET:
            field_dict["parent_merchant_id"] = parent_merchant_id
        if enable_payment_response_hash is not UNSET:
            field_dict["enable_payment_response_hash"] = enable_payment_response_hash
        if payment_response_hash_key is not UNSET:
            field_dict["payment_response_hash_key"] = payment_response_hash_key
        if redirect_to_merchant_with_http_post is not UNSET:
            field_dict["redirect_to_merchant_with_http_post"] = redirect_to_merchant_with_http_post
        if metadata is not UNSET:
            field_dict["metadata"] = metadata
        if publishable_key is not UNSET:
            field_dict["publishable_key"] = publishable_key
        if locker_id is not UNSET:
            field_dict["locker_id"] = locker_id
        if primary_business_details is not UNSET:
            field_dict["primary_business_details"] = primary_business_details
        if frm_routing_algorithm is not UNSET:
            field_dict["frm_routing_algorithm"] = frm_routing_algorithm
        if organization_id is not UNSET:
            field_dict["organization_id"] = organization_id
        if pm_collect_link_config is not UNSET:
            field_dict["pm_collect_link_config"] = pm_collect_link_config
        if product_type is not UNSET:
            field_dict["product_type"] = product_type

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.business_collect_link_config import BusinessCollectLinkConfig
        from ..models.merchant_account_create_frm_routing_algorithm_type_0 import (
            MerchantAccountCreateFrmRoutingAlgorithmType0,
        )
        from ..models.merchant_account_create_metadata_type_0 import MerchantAccountCreateMetadataType0
        from ..models.merchant_details import MerchantDetails
        from ..models.primary_business_details import PrimaryBusinessDetails
        from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
        from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
        from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2
        from ..models.routing_algorithm_type_3 import RoutingAlgorithmType3
        from ..models.webhook_details import WebhookDetails

        d = dict(src_dict)
        merchant_id = d.pop("merchant_id")

        def _parse_merchant_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_name = _parse_merchant_name(d.pop("merchant_name", UNSET))

        def _parse_merchant_details(data: object) -> Union["MerchantDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                merchant_details_type_1 = MerchantDetails.from_dict(data)

                return merchant_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["MerchantDetails", None, Unset], data)

        merchant_details = _parse_merchant_details(d.pop("merchant_details", UNSET))

        def _parse_return_url(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        return_url = _parse_return_url(d.pop("return_url", UNSET))

        def _parse_webhook_details(data: object) -> Union["WebhookDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                webhook_details_type_1 = WebhookDetails.from_dict(data)

                return webhook_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["WebhookDetails", None, Unset], data)

        webhook_details = _parse_webhook_details(d.pop("webhook_details", UNSET))

        def _parse_payout_routing_algorithm(
            data: object,
        ) -> Union[
            "RoutingAlgorithmType0",
            "RoutingAlgorithmType1",
            "RoutingAlgorithmType2",
            "RoutingAlgorithmType3",
            None,
            Unset,
        ]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_algorithm_type_0 = RoutingAlgorithmType0.from_dict(data)

                return componentsschemas_routing_algorithm_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_algorithm_type_1 = RoutingAlgorithmType1.from_dict(data)

                return componentsschemas_routing_algorithm_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_algorithm_type_2 = RoutingAlgorithmType2.from_dict(data)

                return componentsschemas_routing_algorithm_type_2
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_routing_algorithm_type_3 = RoutingAlgorithmType3.from_dict(data)

                return componentsschemas_routing_algorithm_type_3
            except:  # noqa: E722
                pass
            return cast(
                Union[
                    "RoutingAlgorithmType0",
                    "RoutingAlgorithmType1",
                    "RoutingAlgorithmType2",
                    "RoutingAlgorithmType3",
                    None,
                    Unset,
                ],
                data,
            )

        payout_routing_algorithm = _parse_payout_routing_algorithm(d.pop("payout_routing_algorithm", UNSET))

        def _parse_sub_merchants_enabled(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        sub_merchants_enabled = _parse_sub_merchants_enabled(d.pop("sub_merchants_enabled", UNSET))

        def _parse_parent_merchant_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        parent_merchant_id = _parse_parent_merchant_id(d.pop("parent_merchant_id", UNSET))

        def _parse_enable_payment_response_hash(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        enable_payment_response_hash = _parse_enable_payment_response_hash(d.pop("enable_payment_response_hash", UNSET))

        def _parse_payment_response_hash_key(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_response_hash_key = _parse_payment_response_hash_key(d.pop("payment_response_hash_key", UNSET))

        def _parse_redirect_to_merchant_with_http_post(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        redirect_to_merchant_with_http_post = _parse_redirect_to_merchant_with_http_post(
            d.pop("redirect_to_merchant_with_http_post", UNSET)
        )

        def _parse_metadata(data: object) -> Union["MerchantAccountCreateMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = MerchantAccountCreateMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["MerchantAccountCreateMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        def _parse_publishable_key(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        publishable_key = _parse_publishable_key(d.pop("publishable_key", UNSET))

        def _parse_locker_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        locker_id = _parse_locker_id(d.pop("locker_id", UNSET))

        def _parse_primary_business_details(data: object) -> Union["PrimaryBusinessDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                primary_business_details_type_1 = PrimaryBusinessDetails.from_dict(data)

                return primary_business_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["PrimaryBusinessDetails", None, Unset], data)

        primary_business_details = _parse_primary_business_details(d.pop("primary_business_details", UNSET))

        def _parse_frm_routing_algorithm(
            data: object,
        ) -> Union["MerchantAccountCreateFrmRoutingAlgorithmType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                frm_routing_algorithm_type_0 = MerchantAccountCreateFrmRoutingAlgorithmType0.from_dict(data)

                return frm_routing_algorithm_type_0
            except:  # noqa: E722
                pass
            return cast(Union["MerchantAccountCreateFrmRoutingAlgorithmType0", None, Unset], data)

        frm_routing_algorithm = _parse_frm_routing_algorithm(d.pop("frm_routing_algorithm", UNSET))

        def _parse_organization_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        organization_id = _parse_organization_id(d.pop("organization_id", UNSET))

        def _parse_pm_collect_link_config(data: object) -> Union["BusinessCollectLinkConfig", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                pm_collect_link_config_type_1 = BusinessCollectLinkConfig.from_dict(data)

                return pm_collect_link_config_type_1
            except:  # noqa: E722
                pass
            return cast(Union["BusinessCollectLinkConfig", None, Unset], data)

        pm_collect_link_config = _parse_pm_collect_link_config(d.pop("pm_collect_link_config", UNSET))

        def _parse_product_type(data: object) -> Union[MerchantProductType, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                product_type_type_1 = MerchantProductType(data)

                return product_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[MerchantProductType, None, Unset], data)

        product_type = _parse_product_type(d.pop("product_type", UNSET))

        merchant_account_create = cls(
            merchant_id=merchant_id,
            merchant_name=merchant_name,
            merchant_details=merchant_details,
            return_url=return_url,
            webhook_details=webhook_details,
            payout_routing_algorithm=payout_routing_algorithm,
            sub_merchants_enabled=sub_merchants_enabled,
            parent_merchant_id=parent_merchant_id,
            enable_payment_response_hash=enable_payment_response_hash,
            payment_response_hash_key=payment_response_hash_key,
            redirect_to_merchant_with_http_post=redirect_to_merchant_with_http_post,
            metadata=metadata,
            publishable_key=publishable_key,
            locker_id=locker_id,
            primary_business_details=primary_business_details,
            frm_routing_algorithm=frm_routing_algorithm,
            organization_id=organization_id,
            pm_collect_link_config=pm_collect_link_config,
            product_type=product_type,
        )

        return merchant_account_create
