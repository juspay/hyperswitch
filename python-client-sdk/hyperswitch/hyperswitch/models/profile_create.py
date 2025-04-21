from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..models.country_alpha_2 import CountryAlpha2
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.authentication_connector_details import AuthenticationConnectorDetails
    from ..models.business_payment_link_config import BusinessPaymentLinkConfig
    from ..models.business_payout_link_config import BusinessPayoutLinkConfig
    from ..models.card_testing_guard_config import CardTestingGuardConfig
    from ..models.profile_create_authentication_product_ids_type_0 import ProfileCreateAuthenticationProductIdsType0
    from ..models.profile_create_frm_routing_algorithm_type_0 import ProfileCreateFrmRoutingAlgorithmType0
    from ..models.profile_create_metadata_type_0 import ProfileCreateMetadataType0
    from ..models.profile_create_outgoing_webhook_custom_http_headers_type_0 import (
        ProfileCreateOutgoingWebhookCustomHttpHeadersType0,
    )
    from ..models.profile_create_routing_algorithm_type_0 import ProfileCreateRoutingAlgorithmType0
    from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
    from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
    from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2
    from ..models.routing_algorithm_type_3 import RoutingAlgorithmType3
    from ..models.webhook_details import WebhookDetails


T = TypeVar("T", bound="ProfileCreate")


@_attrs_define
class ProfileCreate:
    """
    Attributes:
        profile_name (Union[None, Unset, str]): The name of profile
        return_url (Union[None, Unset, str]): The URL to redirect after the completion of the operation Example:
            https://www.example.com/success.
        enable_payment_response_hash (Union[None, Unset, bool]): A boolean value to indicate if payment response hash
            needs to be enabled Default: True. Example: True.
        payment_response_hash_key (Union[None, Unset, str]): Refers to the hash key used for calculating the signature
            for webhooks and redirect response. If the value is not provided, a value is automatically generated.
        redirect_to_merchant_with_http_post (Union[None, Unset, bool]): A boolean value to indicate if redirect to
            merchant with http post needs to be enabled Default: False. Example: True.
        webhook_details (Union['WebhookDetails', None, Unset]):
        metadata (Union['ProfileCreateMetadataType0', None, Unset]): Metadata is useful for storing additional,
            unstructured information on an object.
        routing_algorithm (Union['ProfileCreateRoutingAlgorithmType0', None, Unset]): The routing algorithm to be used
            for routing payments to desired connectors
        intent_fulfillment_time (Union[None, Unset, int]): Will be used to determine the time till which your payment
            will be active once the payment session starts Example: 900.
        frm_routing_algorithm (Union['ProfileCreateFrmRoutingAlgorithmType0', None, Unset]): The frm routing algorithm
            to be used for routing payments to desired FRM's
        payout_routing_algorithm (Union['RoutingAlgorithmType0', 'RoutingAlgorithmType1', 'RoutingAlgorithmType2',
            'RoutingAlgorithmType3', None, Unset]):
        applepay_verified_domains (Union[None, Unset, list[str]]): Verified Apple Pay domains for a particular profile
        session_expiry (Union[None, Unset, int]): Client Secret Default expiry for all payments created under this
            profile Example: 900.
        payment_link_config (Union['BusinessPaymentLinkConfig', None, Unset]):
        authentication_connector_details (Union['AuthenticationConnectorDetails', None, Unset]):
        use_billing_as_payment_method_billing (Union[None, Unset, bool]): Whether to use the billing details passed when
            creating the intent as payment method billing
        collect_shipping_details_from_wallet_connector (Union[None, Unset, bool]): A boolean value to indicate if
            customer shipping details needs to be collected from wallet
            connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc) Default: False.
        collect_billing_details_from_wallet_connector (Union[None, Unset, bool]): A boolean value to indicate if
            customer billing details needs to be collected from wallet
            connector only if it is required field for connector (Eg. Apple Pay, Google Pay etc) Default: False.
        always_collect_shipping_details_from_wallet_connector (Union[None, Unset, bool]): A boolean value to indicate if
            customer shipping details needs to be collected from wallet
            connector irrespective of connector required fields (Eg. Apple pay, Google pay etc) Default: False.
        always_collect_billing_details_from_wallet_connector (Union[None, Unset, bool]): A boolean value to indicate if
            customer billing details needs to be collected from wallet
            connector irrespective of connector required fields (Eg. Apple pay, Google pay etc) Default: False.
        is_connector_agnostic_mit_enabled (Union[None, Unset, bool]): Indicates if the MIT (merchant initiated
            transaction) payments can be made connector
            agnostic, i.e., MITs may be processed through different connector than CIT (customer
            initiated transaction) based on the routing rules.
            If set to `false`, MIT will go through the same connector as the CIT.
        payout_link_config (Union['BusinessPayoutLinkConfig', None, Unset]):
        outgoing_webhook_custom_http_headers (Union['ProfileCreateOutgoingWebhookCustomHttpHeadersType0', None, Unset]):
            These key-value pairs are sent as additional custom headers in the outgoing webhook request. It is recommended
            not to use more than four key-value pairs.
        tax_connector_id (Union[None, Unset, str]): Merchant Connector id to be stored for tax_calculator connector
        is_tax_connector_enabled (Union[Unset, bool]): Indicates if tax_calculator connector is enabled or not.
            If set to `true` tax_connector_id will be checked.
        is_network_tokenization_enabled (Union[Unset, bool]): Indicates if network tokenization is enabled or not.
        is_auto_retries_enabled (Union[None, Unset, bool]): Indicates if is_auto_retries_enabled is enabled or not.
        max_auto_retries_enabled (Union[None, Unset, int]): Maximum number of auto retries allowed for a payment
        always_request_extended_authorization (Union[None, Unset, bool]): Bool indicating if extended authentication
            must be requested for all payments
        is_click_to_pay_enabled (Union[Unset, bool]): Indicates if click to pay is enabled or not.
        authentication_product_ids (Union['ProfileCreateAuthenticationProductIdsType0', None, Unset]): Product
            authentication ids
        card_testing_guard_config (Union['CardTestingGuardConfig', None, Unset]):
        is_clear_pan_retries_enabled (Union[None, Unset, bool]): Indicates if clear pan retries is enabled or not.
        force_3ds_challenge (Union[None, Unset, bool]): Indicates if 3ds challenge is forced
        is_debit_routing_enabled (Union[None, Unset, bool]): Indicates if debit routing is enabled or not
        merchant_business_country (Union[CountryAlpha2, None, Unset]):
    """

    profile_name: Union[None, Unset, str] = UNSET
    return_url: Union[None, Unset, str] = UNSET
    enable_payment_response_hash: Union[None, Unset, bool] = True
    payment_response_hash_key: Union[None, Unset, str] = UNSET
    redirect_to_merchant_with_http_post: Union[None, Unset, bool] = False
    webhook_details: Union["WebhookDetails", None, Unset] = UNSET
    metadata: Union["ProfileCreateMetadataType0", None, Unset] = UNSET
    routing_algorithm: Union["ProfileCreateRoutingAlgorithmType0", None, Unset] = UNSET
    intent_fulfillment_time: Union[None, Unset, int] = UNSET
    frm_routing_algorithm: Union["ProfileCreateFrmRoutingAlgorithmType0", None, Unset] = UNSET
    payout_routing_algorithm: Union[
        "RoutingAlgorithmType0", "RoutingAlgorithmType1", "RoutingAlgorithmType2", "RoutingAlgorithmType3", None, Unset
    ] = UNSET
    applepay_verified_domains: Union[None, Unset, list[str]] = UNSET
    session_expiry: Union[None, Unset, int] = UNSET
    payment_link_config: Union["BusinessPaymentLinkConfig", None, Unset] = UNSET
    authentication_connector_details: Union["AuthenticationConnectorDetails", None, Unset] = UNSET
    use_billing_as_payment_method_billing: Union[None, Unset, bool] = UNSET
    collect_shipping_details_from_wallet_connector: Union[None, Unset, bool] = False
    collect_billing_details_from_wallet_connector: Union[None, Unset, bool] = False
    always_collect_shipping_details_from_wallet_connector: Union[None, Unset, bool] = False
    always_collect_billing_details_from_wallet_connector: Union[None, Unset, bool] = False
    is_connector_agnostic_mit_enabled: Union[None, Unset, bool] = UNSET
    payout_link_config: Union["BusinessPayoutLinkConfig", None, Unset] = UNSET
    outgoing_webhook_custom_http_headers: Union["ProfileCreateOutgoingWebhookCustomHttpHeadersType0", None, Unset] = (
        UNSET
    )
    tax_connector_id: Union[None, Unset, str] = UNSET
    is_tax_connector_enabled: Union[Unset, bool] = UNSET
    is_network_tokenization_enabled: Union[Unset, bool] = UNSET
    is_auto_retries_enabled: Union[None, Unset, bool] = UNSET
    max_auto_retries_enabled: Union[None, Unset, int] = UNSET
    always_request_extended_authorization: Union[None, Unset, bool] = UNSET
    is_click_to_pay_enabled: Union[Unset, bool] = UNSET
    authentication_product_ids: Union["ProfileCreateAuthenticationProductIdsType0", None, Unset] = UNSET
    card_testing_guard_config: Union["CardTestingGuardConfig", None, Unset] = UNSET
    is_clear_pan_retries_enabled: Union[None, Unset, bool] = UNSET
    force_3ds_challenge: Union[None, Unset, bool] = UNSET
    is_debit_routing_enabled: Union[None, Unset, bool] = UNSET
    merchant_business_country: Union[CountryAlpha2, None, Unset] = UNSET

    def to_dict(self) -> dict[str, Any]:
        from ..models.authentication_connector_details import AuthenticationConnectorDetails
        from ..models.business_payment_link_config import BusinessPaymentLinkConfig
        from ..models.business_payout_link_config import BusinessPayoutLinkConfig
        from ..models.card_testing_guard_config import CardTestingGuardConfig
        from ..models.profile_create_authentication_product_ids_type_0 import ProfileCreateAuthenticationProductIdsType0
        from ..models.profile_create_frm_routing_algorithm_type_0 import ProfileCreateFrmRoutingAlgorithmType0
        from ..models.profile_create_metadata_type_0 import ProfileCreateMetadataType0
        from ..models.profile_create_outgoing_webhook_custom_http_headers_type_0 import (
            ProfileCreateOutgoingWebhookCustomHttpHeadersType0,
        )
        from ..models.profile_create_routing_algorithm_type_0 import ProfileCreateRoutingAlgorithmType0
        from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
        from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
        from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2
        from ..models.routing_algorithm_type_3 import RoutingAlgorithmType3
        from ..models.webhook_details import WebhookDetails

        profile_name: Union[None, Unset, str]
        if isinstance(self.profile_name, Unset):
            profile_name = UNSET
        else:
            profile_name = self.profile_name

        return_url: Union[None, Unset, str]
        if isinstance(self.return_url, Unset):
            return_url = UNSET
        else:
            return_url = self.return_url

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

        webhook_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.webhook_details, Unset):
            webhook_details = UNSET
        elif isinstance(self.webhook_details, WebhookDetails):
            webhook_details = self.webhook_details.to_dict()
        else:
            webhook_details = self.webhook_details

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, ProfileCreateMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        routing_algorithm: Union[None, Unset, dict[str, Any]]
        if isinstance(self.routing_algorithm, Unset):
            routing_algorithm = UNSET
        elif isinstance(self.routing_algorithm, ProfileCreateRoutingAlgorithmType0):
            routing_algorithm = self.routing_algorithm.to_dict()
        else:
            routing_algorithm = self.routing_algorithm

        intent_fulfillment_time: Union[None, Unset, int]
        if isinstance(self.intent_fulfillment_time, Unset):
            intent_fulfillment_time = UNSET
        else:
            intent_fulfillment_time = self.intent_fulfillment_time

        frm_routing_algorithm: Union[None, Unset, dict[str, Any]]
        if isinstance(self.frm_routing_algorithm, Unset):
            frm_routing_algorithm = UNSET
        elif isinstance(self.frm_routing_algorithm, ProfileCreateFrmRoutingAlgorithmType0):
            frm_routing_algorithm = self.frm_routing_algorithm.to_dict()
        else:
            frm_routing_algorithm = self.frm_routing_algorithm

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

        applepay_verified_domains: Union[None, Unset, list[str]]
        if isinstance(self.applepay_verified_domains, Unset):
            applepay_verified_domains = UNSET
        elif isinstance(self.applepay_verified_domains, list):
            applepay_verified_domains = self.applepay_verified_domains

        else:
            applepay_verified_domains = self.applepay_verified_domains

        session_expiry: Union[None, Unset, int]
        if isinstance(self.session_expiry, Unset):
            session_expiry = UNSET
        else:
            session_expiry = self.session_expiry

        payment_link_config: Union[None, Unset, dict[str, Any]]
        if isinstance(self.payment_link_config, Unset):
            payment_link_config = UNSET
        elif isinstance(self.payment_link_config, BusinessPaymentLinkConfig):
            payment_link_config = self.payment_link_config.to_dict()
        else:
            payment_link_config = self.payment_link_config

        authentication_connector_details: Union[None, Unset, dict[str, Any]]
        if isinstance(self.authentication_connector_details, Unset):
            authentication_connector_details = UNSET
        elif isinstance(self.authentication_connector_details, AuthenticationConnectorDetails):
            authentication_connector_details = self.authentication_connector_details.to_dict()
        else:
            authentication_connector_details = self.authentication_connector_details

        use_billing_as_payment_method_billing: Union[None, Unset, bool]
        if isinstance(self.use_billing_as_payment_method_billing, Unset):
            use_billing_as_payment_method_billing = UNSET
        else:
            use_billing_as_payment_method_billing = self.use_billing_as_payment_method_billing

        collect_shipping_details_from_wallet_connector: Union[None, Unset, bool]
        if isinstance(self.collect_shipping_details_from_wallet_connector, Unset):
            collect_shipping_details_from_wallet_connector = UNSET
        else:
            collect_shipping_details_from_wallet_connector = self.collect_shipping_details_from_wallet_connector

        collect_billing_details_from_wallet_connector: Union[None, Unset, bool]
        if isinstance(self.collect_billing_details_from_wallet_connector, Unset):
            collect_billing_details_from_wallet_connector = UNSET
        else:
            collect_billing_details_from_wallet_connector = self.collect_billing_details_from_wallet_connector

        always_collect_shipping_details_from_wallet_connector: Union[None, Unset, bool]
        if isinstance(self.always_collect_shipping_details_from_wallet_connector, Unset):
            always_collect_shipping_details_from_wallet_connector = UNSET
        else:
            always_collect_shipping_details_from_wallet_connector = (
                self.always_collect_shipping_details_from_wallet_connector
            )

        always_collect_billing_details_from_wallet_connector: Union[None, Unset, bool]
        if isinstance(self.always_collect_billing_details_from_wallet_connector, Unset):
            always_collect_billing_details_from_wallet_connector = UNSET
        else:
            always_collect_billing_details_from_wallet_connector = (
                self.always_collect_billing_details_from_wallet_connector
            )

        is_connector_agnostic_mit_enabled: Union[None, Unset, bool]
        if isinstance(self.is_connector_agnostic_mit_enabled, Unset):
            is_connector_agnostic_mit_enabled = UNSET
        else:
            is_connector_agnostic_mit_enabled = self.is_connector_agnostic_mit_enabled

        payout_link_config: Union[None, Unset, dict[str, Any]]
        if isinstance(self.payout_link_config, Unset):
            payout_link_config = UNSET
        elif isinstance(self.payout_link_config, BusinessPayoutLinkConfig):
            payout_link_config = self.payout_link_config.to_dict()
        else:
            payout_link_config = self.payout_link_config

        outgoing_webhook_custom_http_headers: Union[None, Unset, dict[str, Any]]
        if isinstance(self.outgoing_webhook_custom_http_headers, Unset):
            outgoing_webhook_custom_http_headers = UNSET
        elif isinstance(self.outgoing_webhook_custom_http_headers, ProfileCreateOutgoingWebhookCustomHttpHeadersType0):
            outgoing_webhook_custom_http_headers = self.outgoing_webhook_custom_http_headers.to_dict()
        else:
            outgoing_webhook_custom_http_headers = self.outgoing_webhook_custom_http_headers

        tax_connector_id: Union[None, Unset, str]
        if isinstance(self.tax_connector_id, Unset):
            tax_connector_id = UNSET
        else:
            tax_connector_id = self.tax_connector_id

        is_tax_connector_enabled = self.is_tax_connector_enabled

        is_network_tokenization_enabled = self.is_network_tokenization_enabled

        is_auto_retries_enabled: Union[None, Unset, bool]
        if isinstance(self.is_auto_retries_enabled, Unset):
            is_auto_retries_enabled = UNSET
        else:
            is_auto_retries_enabled = self.is_auto_retries_enabled

        max_auto_retries_enabled: Union[None, Unset, int]
        if isinstance(self.max_auto_retries_enabled, Unset):
            max_auto_retries_enabled = UNSET
        else:
            max_auto_retries_enabled = self.max_auto_retries_enabled

        always_request_extended_authorization: Union[None, Unset, bool]
        if isinstance(self.always_request_extended_authorization, Unset):
            always_request_extended_authorization = UNSET
        else:
            always_request_extended_authorization = self.always_request_extended_authorization

        is_click_to_pay_enabled = self.is_click_to_pay_enabled

        authentication_product_ids: Union[None, Unset, dict[str, Any]]
        if isinstance(self.authentication_product_ids, Unset):
            authentication_product_ids = UNSET
        elif isinstance(self.authentication_product_ids, ProfileCreateAuthenticationProductIdsType0):
            authentication_product_ids = self.authentication_product_ids.to_dict()
        else:
            authentication_product_ids = self.authentication_product_ids

        card_testing_guard_config: Union[None, Unset, dict[str, Any]]
        if isinstance(self.card_testing_guard_config, Unset):
            card_testing_guard_config = UNSET
        elif isinstance(self.card_testing_guard_config, CardTestingGuardConfig):
            card_testing_guard_config = self.card_testing_guard_config.to_dict()
        else:
            card_testing_guard_config = self.card_testing_guard_config

        is_clear_pan_retries_enabled: Union[None, Unset, bool]
        if isinstance(self.is_clear_pan_retries_enabled, Unset):
            is_clear_pan_retries_enabled = UNSET
        else:
            is_clear_pan_retries_enabled = self.is_clear_pan_retries_enabled

        force_3ds_challenge: Union[None, Unset, bool]
        if isinstance(self.force_3ds_challenge, Unset):
            force_3ds_challenge = UNSET
        else:
            force_3ds_challenge = self.force_3ds_challenge

        is_debit_routing_enabled: Union[None, Unset, bool]
        if isinstance(self.is_debit_routing_enabled, Unset):
            is_debit_routing_enabled = UNSET
        else:
            is_debit_routing_enabled = self.is_debit_routing_enabled

        merchant_business_country: Union[None, Unset, str]
        if isinstance(self.merchant_business_country, Unset):
            merchant_business_country = UNSET
        elif isinstance(self.merchant_business_country, CountryAlpha2):
            merchant_business_country = self.merchant_business_country.value
        else:
            merchant_business_country = self.merchant_business_country

        field_dict: dict[str, Any] = {}
        field_dict.update({})
        if profile_name is not UNSET:
            field_dict["profile_name"] = profile_name
        if return_url is not UNSET:
            field_dict["return_url"] = return_url
        if enable_payment_response_hash is not UNSET:
            field_dict["enable_payment_response_hash"] = enable_payment_response_hash
        if payment_response_hash_key is not UNSET:
            field_dict["payment_response_hash_key"] = payment_response_hash_key
        if redirect_to_merchant_with_http_post is not UNSET:
            field_dict["redirect_to_merchant_with_http_post"] = redirect_to_merchant_with_http_post
        if webhook_details is not UNSET:
            field_dict["webhook_details"] = webhook_details
        if metadata is not UNSET:
            field_dict["metadata"] = metadata
        if routing_algorithm is not UNSET:
            field_dict["routing_algorithm"] = routing_algorithm
        if intent_fulfillment_time is not UNSET:
            field_dict["intent_fulfillment_time"] = intent_fulfillment_time
        if frm_routing_algorithm is not UNSET:
            field_dict["frm_routing_algorithm"] = frm_routing_algorithm
        if payout_routing_algorithm is not UNSET:
            field_dict["payout_routing_algorithm"] = payout_routing_algorithm
        if applepay_verified_domains is not UNSET:
            field_dict["applepay_verified_domains"] = applepay_verified_domains
        if session_expiry is not UNSET:
            field_dict["session_expiry"] = session_expiry
        if payment_link_config is not UNSET:
            field_dict["payment_link_config"] = payment_link_config
        if authentication_connector_details is not UNSET:
            field_dict["authentication_connector_details"] = authentication_connector_details
        if use_billing_as_payment_method_billing is not UNSET:
            field_dict["use_billing_as_payment_method_billing"] = use_billing_as_payment_method_billing
        if collect_shipping_details_from_wallet_connector is not UNSET:
            field_dict["collect_shipping_details_from_wallet_connector"] = (
                collect_shipping_details_from_wallet_connector
            )
        if collect_billing_details_from_wallet_connector is not UNSET:
            field_dict["collect_billing_details_from_wallet_connector"] = collect_billing_details_from_wallet_connector
        if always_collect_shipping_details_from_wallet_connector is not UNSET:
            field_dict["always_collect_shipping_details_from_wallet_connector"] = (
                always_collect_shipping_details_from_wallet_connector
            )
        if always_collect_billing_details_from_wallet_connector is not UNSET:
            field_dict["always_collect_billing_details_from_wallet_connector"] = (
                always_collect_billing_details_from_wallet_connector
            )
        if is_connector_agnostic_mit_enabled is not UNSET:
            field_dict["is_connector_agnostic_mit_enabled"] = is_connector_agnostic_mit_enabled
        if payout_link_config is not UNSET:
            field_dict["payout_link_config"] = payout_link_config
        if outgoing_webhook_custom_http_headers is not UNSET:
            field_dict["outgoing_webhook_custom_http_headers"] = outgoing_webhook_custom_http_headers
        if tax_connector_id is not UNSET:
            field_dict["tax_connector_id"] = tax_connector_id
        if is_tax_connector_enabled is not UNSET:
            field_dict["is_tax_connector_enabled"] = is_tax_connector_enabled
        if is_network_tokenization_enabled is not UNSET:
            field_dict["is_network_tokenization_enabled"] = is_network_tokenization_enabled
        if is_auto_retries_enabled is not UNSET:
            field_dict["is_auto_retries_enabled"] = is_auto_retries_enabled
        if max_auto_retries_enabled is not UNSET:
            field_dict["max_auto_retries_enabled"] = max_auto_retries_enabled
        if always_request_extended_authorization is not UNSET:
            field_dict["always_request_extended_authorization"] = always_request_extended_authorization
        if is_click_to_pay_enabled is not UNSET:
            field_dict["is_click_to_pay_enabled"] = is_click_to_pay_enabled
        if authentication_product_ids is not UNSET:
            field_dict["authentication_product_ids"] = authentication_product_ids
        if card_testing_guard_config is not UNSET:
            field_dict["card_testing_guard_config"] = card_testing_guard_config
        if is_clear_pan_retries_enabled is not UNSET:
            field_dict["is_clear_pan_retries_enabled"] = is_clear_pan_retries_enabled
        if force_3ds_challenge is not UNSET:
            field_dict["force_3ds_challenge"] = force_3ds_challenge
        if is_debit_routing_enabled is not UNSET:
            field_dict["is_debit_routing_enabled"] = is_debit_routing_enabled
        if merchant_business_country is not UNSET:
            field_dict["merchant_business_country"] = merchant_business_country

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.authentication_connector_details import AuthenticationConnectorDetails
        from ..models.business_payment_link_config import BusinessPaymentLinkConfig
        from ..models.business_payout_link_config import BusinessPayoutLinkConfig
        from ..models.card_testing_guard_config import CardTestingGuardConfig
        from ..models.profile_create_authentication_product_ids_type_0 import ProfileCreateAuthenticationProductIdsType0
        from ..models.profile_create_frm_routing_algorithm_type_0 import ProfileCreateFrmRoutingAlgorithmType0
        from ..models.profile_create_metadata_type_0 import ProfileCreateMetadataType0
        from ..models.profile_create_outgoing_webhook_custom_http_headers_type_0 import (
            ProfileCreateOutgoingWebhookCustomHttpHeadersType0,
        )
        from ..models.profile_create_routing_algorithm_type_0 import ProfileCreateRoutingAlgorithmType0
        from ..models.routing_algorithm_type_0 import RoutingAlgorithmType0
        from ..models.routing_algorithm_type_1 import RoutingAlgorithmType1
        from ..models.routing_algorithm_type_2 import RoutingAlgorithmType2
        from ..models.routing_algorithm_type_3 import RoutingAlgorithmType3
        from ..models.webhook_details import WebhookDetails

        d = dict(src_dict)

        def _parse_profile_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        profile_name = _parse_profile_name(d.pop("profile_name", UNSET))

        def _parse_return_url(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        return_url = _parse_return_url(d.pop("return_url", UNSET))

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

        def _parse_metadata(data: object) -> Union["ProfileCreateMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = ProfileCreateMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["ProfileCreateMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        def _parse_routing_algorithm(data: object) -> Union["ProfileCreateRoutingAlgorithmType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                routing_algorithm_type_0 = ProfileCreateRoutingAlgorithmType0.from_dict(data)

                return routing_algorithm_type_0
            except:  # noqa: E722
                pass
            return cast(Union["ProfileCreateRoutingAlgorithmType0", None, Unset], data)

        routing_algorithm = _parse_routing_algorithm(d.pop("routing_algorithm", UNSET))

        def _parse_intent_fulfillment_time(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        intent_fulfillment_time = _parse_intent_fulfillment_time(d.pop("intent_fulfillment_time", UNSET))

        def _parse_frm_routing_algorithm(data: object) -> Union["ProfileCreateFrmRoutingAlgorithmType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                frm_routing_algorithm_type_0 = ProfileCreateFrmRoutingAlgorithmType0.from_dict(data)

                return frm_routing_algorithm_type_0
            except:  # noqa: E722
                pass
            return cast(Union["ProfileCreateFrmRoutingAlgorithmType0", None, Unset], data)

        frm_routing_algorithm = _parse_frm_routing_algorithm(d.pop("frm_routing_algorithm", UNSET))

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

        def _parse_applepay_verified_domains(data: object) -> Union[None, Unset, list[str]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                applepay_verified_domains_type_0 = cast(list[str], data)

                return applepay_verified_domains_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[str]], data)

        applepay_verified_domains = _parse_applepay_verified_domains(d.pop("applepay_verified_domains", UNSET))

        def _parse_session_expiry(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        session_expiry = _parse_session_expiry(d.pop("session_expiry", UNSET))

        def _parse_payment_link_config(data: object) -> Union["BusinessPaymentLinkConfig", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                payment_link_config_type_1 = BusinessPaymentLinkConfig.from_dict(data)

                return payment_link_config_type_1
            except:  # noqa: E722
                pass
            return cast(Union["BusinessPaymentLinkConfig", None, Unset], data)

        payment_link_config = _parse_payment_link_config(d.pop("payment_link_config", UNSET))

        def _parse_authentication_connector_details(
            data: object,
        ) -> Union["AuthenticationConnectorDetails", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                authentication_connector_details_type_1 = AuthenticationConnectorDetails.from_dict(data)

                return authentication_connector_details_type_1
            except:  # noqa: E722
                pass
            return cast(Union["AuthenticationConnectorDetails", None, Unset], data)

        authentication_connector_details = _parse_authentication_connector_details(
            d.pop("authentication_connector_details", UNSET)
        )

        def _parse_use_billing_as_payment_method_billing(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        use_billing_as_payment_method_billing = _parse_use_billing_as_payment_method_billing(
            d.pop("use_billing_as_payment_method_billing", UNSET)
        )

        def _parse_collect_shipping_details_from_wallet_connector(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        collect_shipping_details_from_wallet_connector = _parse_collect_shipping_details_from_wallet_connector(
            d.pop("collect_shipping_details_from_wallet_connector", UNSET)
        )

        def _parse_collect_billing_details_from_wallet_connector(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        collect_billing_details_from_wallet_connector = _parse_collect_billing_details_from_wallet_connector(
            d.pop("collect_billing_details_from_wallet_connector", UNSET)
        )

        def _parse_always_collect_shipping_details_from_wallet_connector(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        always_collect_shipping_details_from_wallet_connector = (
            _parse_always_collect_shipping_details_from_wallet_connector(
                d.pop("always_collect_shipping_details_from_wallet_connector", UNSET)
            )
        )

        def _parse_always_collect_billing_details_from_wallet_connector(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        always_collect_billing_details_from_wallet_connector = (
            _parse_always_collect_billing_details_from_wallet_connector(
                d.pop("always_collect_billing_details_from_wallet_connector", UNSET)
            )
        )

        def _parse_is_connector_agnostic_mit_enabled(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        is_connector_agnostic_mit_enabled = _parse_is_connector_agnostic_mit_enabled(
            d.pop("is_connector_agnostic_mit_enabled", UNSET)
        )

        def _parse_payout_link_config(data: object) -> Union["BusinessPayoutLinkConfig", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                payout_link_config_type_1 = BusinessPayoutLinkConfig.from_dict(data)

                return payout_link_config_type_1
            except:  # noqa: E722
                pass
            return cast(Union["BusinessPayoutLinkConfig", None, Unset], data)

        payout_link_config = _parse_payout_link_config(d.pop("payout_link_config", UNSET))

        def _parse_outgoing_webhook_custom_http_headers(
            data: object,
        ) -> Union["ProfileCreateOutgoingWebhookCustomHttpHeadersType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                outgoing_webhook_custom_http_headers_type_0 = (
                    ProfileCreateOutgoingWebhookCustomHttpHeadersType0.from_dict(data)
                )

                return outgoing_webhook_custom_http_headers_type_0
            except:  # noqa: E722
                pass
            return cast(Union["ProfileCreateOutgoingWebhookCustomHttpHeadersType0", None, Unset], data)

        outgoing_webhook_custom_http_headers = _parse_outgoing_webhook_custom_http_headers(
            d.pop("outgoing_webhook_custom_http_headers", UNSET)
        )

        def _parse_tax_connector_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        tax_connector_id = _parse_tax_connector_id(d.pop("tax_connector_id", UNSET))

        is_tax_connector_enabled = d.pop("is_tax_connector_enabled", UNSET)

        is_network_tokenization_enabled = d.pop("is_network_tokenization_enabled", UNSET)

        def _parse_is_auto_retries_enabled(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        is_auto_retries_enabled = _parse_is_auto_retries_enabled(d.pop("is_auto_retries_enabled", UNSET))

        def _parse_max_auto_retries_enabled(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        max_auto_retries_enabled = _parse_max_auto_retries_enabled(d.pop("max_auto_retries_enabled", UNSET))

        def _parse_always_request_extended_authorization(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        always_request_extended_authorization = _parse_always_request_extended_authorization(
            d.pop("always_request_extended_authorization", UNSET)
        )

        is_click_to_pay_enabled = d.pop("is_click_to_pay_enabled", UNSET)

        def _parse_authentication_product_ids(
            data: object,
        ) -> Union["ProfileCreateAuthenticationProductIdsType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                authentication_product_ids_type_0 = ProfileCreateAuthenticationProductIdsType0.from_dict(data)

                return authentication_product_ids_type_0
            except:  # noqa: E722
                pass
            return cast(Union["ProfileCreateAuthenticationProductIdsType0", None, Unset], data)

        authentication_product_ids = _parse_authentication_product_ids(d.pop("authentication_product_ids", UNSET))

        def _parse_card_testing_guard_config(data: object) -> Union["CardTestingGuardConfig", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                card_testing_guard_config_type_1 = CardTestingGuardConfig.from_dict(data)

                return card_testing_guard_config_type_1
            except:  # noqa: E722
                pass
            return cast(Union["CardTestingGuardConfig", None, Unset], data)

        card_testing_guard_config = _parse_card_testing_guard_config(d.pop("card_testing_guard_config", UNSET))

        def _parse_is_clear_pan_retries_enabled(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        is_clear_pan_retries_enabled = _parse_is_clear_pan_retries_enabled(d.pop("is_clear_pan_retries_enabled", UNSET))

        def _parse_force_3ds_challenge(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        force_3ds_challenge = _parse_force_3ds_challenge(d.pop("force_3ds_challenge", UNSET))

        def _parse_is_debit_routing_enabled(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        is_debit_routing_enabled = _parse_is_debit_routing_enabled(d.pop("is_debit_routing_enabled", UNSET))

        def _parse_merchant_business_country(data: object) -> Union[CountryAlpha2, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                merchant_business_country_type_1 = CountryAlpha2(data)

                return merchant_business_country_type_1
            except:  # noqa: E722
                pass
            return cast(Union[CountryAlpha2, None, Unset], data)

        merchant_business_country = _parse_merchant_business_country(d.pop("merchant_business_country", UNSET))

        profile_create = cls(
            profile_name=profile_name,
            return_url=return_url,
            enable_payment_response_hash=enable_payment_response_hash,
            payment_response_hash_key=payment_response_hash_key,
            redirect_to_merchant_with_http_post=redirect_to_merchant_with_http_post,
            webhook_details=webhook_details,
            metadata=metadata,
            routing_algorithm=routing_algorithm,
            intent_fulfillment_time=intent_fulfillment_time,
            frm_routing_algorithm=frm_routing_algorithm,
            payout_routing_algorithm=payout_routing_algorithm,
            applepay_verified_domains=applepay_verified_domains,
            session_expiry=session_expiry,
            payment_link_config=payment_link_config,
            authentication_connector_details=authentication_connector_details,
            use_billing_as_payment_method_billing=use_billing_as_payment_method_billing,
            collect_shipping_details_from_wallet_connector=collect_shipping_details_from_wallet_connector,
            collect_billing_details_from_wallet_connector=collect_billing_details_from_wallet_connector,
            always_collect_shipping_details_from_wallet_connector=always_collect_shipping_details_from_wallet_connector,
            always_collect_billing_details_from_wallet_connector=always_collect_billing_details_from_wallet_connector,
            is_connector_agnostic_mit_enabled=is_connector_agnostic_mit_enabled,
            payout_link_config=payout_link_config,
            outgoing_webhook_custom_http_headers=outgoing_webhook_custom_http_headers,
            tax_connector_id=tax_connector_id,
            is_tax_connector_enabled=is_tax_connector_enabled,
            is_network_tokenization_enabled=is_network_tokenization_enabled,
            is_auto_retries_enabled=is_auto_retries_enabled,
            max_auto_retries_enabled=max_auto_retries_enabled,
            always_request_extended_authorization=always_request_extended_authorization,
            is_click_to_pay_enabled=is_click_to_pay_enabled,
            authentication_product_ids=authentication_product_ids,
            card_testing_guard_config=card_testing_guard_config,
            is_clear_pan_retries_enabled=is_clear_pan_retries_enabled,
            force_3ds_challenge=force_3ds_challenge,
            is_debit_routing_enabled=is_debit_routing_enabled,
            merchant_business_country=merchant_business_country,
        )

        return profile_create
