from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.revoke_api_key_response import RevokeApiKeyResponse
from ...types import Response


def _get_kwargs(
    merchant_id: str,
    key_id: str,
) -> dict[str, Any]:
    _kwargs: dict[str, Any] = {
        "method": "delete",
        "url": f"/api_keys/{merchant_id}/{key_id}",
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, RevokeApiKeyResponse]]:
    if response.status_code == 200:
        response_200 = RevokeApiKeyResponse.from_dict(response.json())

        return response_200
    if response.status_code == 404:
        response_404 = cast(Any, None)
        return response_404
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[Any, RevokeApiKeyResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    merchant_id: str,
    key_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, RevokeApiKeyResponse]]:
    """API Key - Revoke

     Revoke the specified API Key. Once revoked, the API Key can no longer be used for
    authenticating with our APIs.

    Args:
        merchant_id (str):
        key_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RevokeApiKeyResponse]]
    """

    kwargs = _get_kwargs(
        merchant_id=merchant_id,
        key_id=key_id,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    merchant_id: str,
    key_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, RevokeApiKeyResponse]]:
    """API Key - Revoke

     Revoke the specified API Key. Once revoked, the API Key can no longer be used for
    authenticating with our APIs.

    Args:
        merchant_id (str):
        key_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RevokeApiKeyResponse]
    """

    return sync_detailed(
        merchant_id=merchant_id,
        key_id=key_id,
        client=client,
    ).parsed


async def asyncio_detailed(
    merchant_id: str,
    key_id: str,
    *,
    client: AuthenticatedClient,
) -> Response[Union[Any, RevokeApiKeyResponse]]:
    """API Key - Revoke

     Revoke the specified API Key. Once revoked, the API Key can no longer be used for
    authenticating with our APIs.

    Args:
        merchant_id (str):
        key_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RevokeApiKeyResponse]]
    """

    kwargs = _get_kwargs(
        merchant_id=merchant_id,
        key_id=key_id,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    merchant_id: str,
    key_id: str,
    *,
    client: AuthenticatedClient,
) -> Optional[Union[Any, RevokeApiKeyResponse]]:
    """API Key - Revoke

     Revoke the specified API Key. Once revoked, the API Key can no longer be used for
    authenticating with our APIs.

    Args:
        merchant_id (str):
        key_id (str):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RevokeApiKeyResponse]
    """

    return (
        await asyncio_detailed(
            merchant_id=merchant_id,
            key_id=key_id,
            client=client,
        )
    ).parsed
