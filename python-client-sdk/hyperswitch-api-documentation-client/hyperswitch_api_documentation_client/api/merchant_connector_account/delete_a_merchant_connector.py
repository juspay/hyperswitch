from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.merchant_connector_delete_response import MerchantConnectorDeleteResponse
from ...types import Response


def _get_kwargs(
    account_id: str,
    connector_id: int,
) -> dict[str, Any]:
    _kwargs: dict[str, Any] = {
        "method": "delete",
        "url": f"/accounts/{account_id}/connectors/{connector_id}",
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, MerchantConnectorDeleteResponse]]:
    if response.status_code == 200:
        response_200 = MerchantConnectorDeleteResponse.from_dict(response.json())

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
) -> Response[Union[Any, MerchantConnectorDeleteResponse]]:
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
) -> Response[Union[Any, MerchantConnectorDeleteResponse]]:
    """Merchant Connector - Delete

     Delete or Detach a Merchant Connector from Merchant Account

    Args:
        account_id (str):
        connector_id (int):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, MerchantConnectorDeleteResponse]]
    """

    kwargs = _get_kwargs(
        account_id=account_id,
        connector_id=connector_id,
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
) -> Optional[Union[Any, MerchantConnectorDeleteResponse]]:
    """Merchant Connector - Delete

     Delete or Detach a Merchant Connector from Merchant Account

    Args:
        account_id (str):
        connector_id (int):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, MerchantConnectorDeleteResponse]
    """

    return sync_detailed(
        account_id=account_id,
        connector_id=connector_id,
        client=client,
    ).parsed


async def asyncio_detailed(
    account_id: str,
    connector_id: int,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, MerchantConnectorDeleteResponse]]:
    """Merchant Connector - Delete

     Delete or Detach a Merchant Connector from Merchant Account

    Args:
        account_id (str):
        connector_id (int):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, MerchantConnectorDeleteResponse]]
    """

    kwargs = _get_kwargs(
        account_id=account_id,
        connector_id=connector_id,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    account_id: str,
    connector_id: int,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, MerchantConnectorDeleteResponse]]:
    """Merchant Connector - Delete

     Delete or Detach a Merchant Connector from Merchant Account

    Args:
        account_id (str):
        connector_id (int):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, MerchantConnectorDeleteResponse]
    """

    return (
        await asyncio_detailed(
            account_id=account_id,
            connector_id=connector_id,
            client=client,
        )
    ).parsed
