from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.create_api_key_request import CreateApiKeyRequest
from ...models.create_api_key_response import CreateApiKeyResponse
from ...types import Response


def _get_kwargs(
    merchant_id: str,
    *,
    body: CreateApiKeyRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": f"/api_keys/{merchant_id}",
    }

    _body = body.to_dict()

    _kwargs["json"] = _body
    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, CreateApiKeyResponse]]:
    if response.status_code == 200:
        response_200 = CreateApiKeyResponse.from_dict(response.json())

        return response_200
    if response.status_code == 400:
        response_400 = cast(Any, None)
        return response_400
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[Union[Any, CreateApiKeyResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    merchant_id: str,
    *,
    client: AuthenticatedClient,
    body: CreateApiKeyRequest,
) -> Response[Union[Any, CreateApiKeyResponse]]:
    """API Key - Create

     Create a new API Key for accessing our APIs from your servers. The plaintext API Key will be
    displayed only once on creation, so ensure you store it securely.

    Args:
        merchant_id (str):
        body (CreateApiKeyRequest): The request body for creating an API Key.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, CreateApiKeyResponse]]
    """

    kwargs = _get_kwargs(
        merchant_id=merchant_id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    merchant_id: str,
    *,
    client: AuthenticatedClient,
    body: CreateApiKeyRequest,
) -> Optional[Union[Any, CreateApiKeyResponse]]:
    """API Key - Create

     Create a new API Key for accessing our APIs from your servers. The plaintext API Key will be
    displayed only once on creation, so ensure you store it securely.

    Args:
        merchant_id (str):
        body (CreateApiKeyRequest): The request body for creating an API Key.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, CreateApiKeyResponse]
    """

    return sync_detailed(
        merchant_id=merchant_id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    merchant_id: str,
    *,
    client: AuthenticatedClient,
    body: CreateApiKeyRequest,
) -> Response[Union[Any, CreateApiKeyResponse]]:
    """API Key - Create

     Create a new API Key for accessing our APIs from your servers. The plaintext API Key will be
    displayed only once on creation, so ensure you store it securely.

    Args:
        merchant_id (str):
        body (CreateApiKeyRequest): The request body for creating an API Key.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, CreateApiKeyResponse]]
    """

    kwargs = _get_kwargs(
        merchant_id=merchant_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    merchant_id: str,
    *,
    client: AuthenticatedClient,
    body: CreateApiKeyRequest,
) -> Optional[Union[Any, CreateApiKeyResponse]]:
    """API Key - Create

     Create a new API Key for accessing our APIs from your servers. The plaintext API Key will be
    displayed only once on creation, so ensure you store it securely.

    Args:
        merchant_id (str):
        body (CreateApiKeyRequest): The request body for creating an API Key.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, CreateApiKeyResponse]
    """

    return (
        await asyncio_detailed(
            merchant_id=merchant_id,
            client=client,
            body=body,
        )
    ).parsed
