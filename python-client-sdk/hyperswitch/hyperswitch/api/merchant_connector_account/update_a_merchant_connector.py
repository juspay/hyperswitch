from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.merchant_connector_response import MerchantConnectorResponse
from ...models.merchant_connector_update import MerchantConnectorUpdate
from ...types import Response


def _get_kwargs(
    account_id: str,
    connector_id: int,
    *,
    body: MerchantConnectorUpdate,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": f"/accounts/{account_id}/connectors/{connector_id}",
    }

    _body = body.to_dict()

    _kwargs["json"] = _body
    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, MerchantConnectorResponse]]:
    if response.status_code == 200:
        response_200 = MerchantConnectorResponse.from_dict(response.json())

        return response_200
    if response.status_code == 401:
        response_401 = cast(Any, None)
        return response_401
    if response.status_code == 404:
        response_404 = cast(Any, None)
        return response_404
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[Any, MerchantConnectorResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    account_id: str,
    connector_id: int,
    *,
    client: AuthenticatedClient,
    body: MerchantConnectorUpdate,
) -> Response[Union[Any, MerchantConnectorResponse]]:
    """Merchant Connector - Update

     To update an existing Merchant Connector account. Helpful in enabling/disabling different payment
    methods and other settings for the connector

    Args:
        account_id (str):
        connector_id (int):
        body (MerchantConnectorUpdate): Create a new Merchant Connector for the merchant account.
            The connector could be a payment processor / facilitator / acquirer or specialized
            services like Fraud / Accounting etc."

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, MerchantConnectorResponse]]
    """

    kwargs = _get_kwargs(
        account_id=account_id,
        connector_id=connector_id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    account_id: str,
    connector_id: int,
    *,
    client: AuthenticatedClient,
    body: MerchantConnectorUpdate,
) -> Optional[Union[Any, MerchantConnectorResponse]]:
    """Merchant Connector - Update

     To update an existing Merchant Connector account. Helpful in enabling/disabling different payment
    methods and other settings for the connector

    Args:
        account_id (str):
        connector_id (int):
        body (MerchantConnectorUpdate): Create a new Merchant Connector for the merchant account.
            The connector could be a payment processor / facilitator / acquirer or specialized
            services like Fraud / Accounting etc."

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, MerchantConnectorResponse]
    """

    return sync_detailed(
        account_id=account_id,
        connector_id=connector_id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    account_id: str,
    connector_id: int,
    *,
    client: AuthenticatedClient,
    body: MerchantConnectorUpdate,
) -> Response[Union[Any, MerchantConnectorResponse]]:
    """Merchant Connector - Update

     To update an existing Merchant Connector account. Helpful in enabling/disabling different payment
    methods and other settings for the connector

    Args:
        account_id (str):
        connector_id (int):
        body (MerchantConnectorUpdate): Create a new Merchant Connector for the merchant account.
            The connector could be a payment processor / facilitator / acquirer or specialized
            services like Fraud / Accounting etc."

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, MerchantConnectorResponse]]
    """

    kwargs = _get_kwargs(
        account_id=account_id,
        connector_id=connector_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    account_id: str,
    connector_id: int,
    *,
    client: AuthenticatedClient,
    body: MerchantConnectorUpdate,
) -> Optional[Union[Any, MerchantConnectorResponse]]:
    """Merchant Connector - Update

     To update an existing Merchant Connector account. Helpful in enabling/disabling different payment
    methods and other settings for the connector

    Args:
        account_id (str):
        connector_id (int):
        body (MerchantConnectorUpdate): Create a new Merchant Connector for the merchant account.
            The connector could be a payment processor / facilitator / acquirer or specialized
            services like Fraud / Accounting etc."

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, MerchantConnectorResponse]
    """

    return (
        await asyncio_detailed(
            account_id=account_id,
            connector_id=connector_id,
            client=client,
            body=body,
        )
    ).parsed
