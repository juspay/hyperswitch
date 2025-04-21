from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.retrieve_api_key_response import RetrieveApiKeyResponse
from ...models.update_api_key_request import UpdateApiKeyRequest
from ...types import Response


def _get_kwargs(
    merchant_id: str,
    key_id: str,
    *,
    body: UpdateApiKeyRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": f"/api_keys/{merchant_id}/{key_id}",
    }

    _body = body.to_dict()

    _kwargs["json"] = _body
    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, RetrieveApiKeyResponse]]:
    if response.status_code == 200:
        response_200 = RetrieveApiKeyResponse.from_dict(response.json())

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
) -> Response[Union[Any, RetrieveApiKeyResponse]]:
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
    body: UpdateApiKeyRequest,
) -> Response[Union[Any, RetrieveApiKeyResponse]]:
    """API Key - Update

     Update information for the specified API Key.

    Args:
        merchant_id (str):
        key_id (str):
        body (UpdateApiKeyRequest): The request body for updating an API Key.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RetrieveApiKeyResponse]]
    """

    kwargs = _get_kwargs(
        merchant_id=merchant_id,
        key_id=key_id,
        body=body,
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
    body: UpdateApiKeyRequest,
) -> Optional[Union[Any, RetrieveApiKeyResponse]]:
    """API Key - Update

     Update information for the specified API Key.

    Args:
        merchant_id (str):
        key_id (str):
        body (UpdateApiKeyRequest): The request body for updating an API Key.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RetrieveApiKeyResponse]
    """

    return sync_detailed(
        merchant_id=merchant_id,
        key_id=key_id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    merchant_id: str,
    key_id: str,
    *,
    client: AuthenticatedClient,
    body: UpdateApiKeyRequest,
) -> Response[Union[Any, RetrieveApiKeyResponse]]:
    """API Key - Update

     Update information for the specified API Key.

    Args:
        merchant_id (str):
        key_id (str):
        body (UpdateApiKeyRequest): The request body for updating an API Key.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RetrieveApiKeyResponse]]
    """

    kwargs = _get_kwargs(
        merchant_id=merchant_id,
        key_id=key_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    merchant_id: str,
    key_id: str,
    *,
    client: AuthenticatedClient,
    body: UpdateApiKeyRequest,
) -> Optional[Union[Any, RetrieveApiKeyResponse]]:
    """API Key - Update

     Update information for the specified API Key.

    Args:
        merchant_id (str):
        key_id (str):
        body (UpdateApiKeyRequest): The request body for updating an API Key.

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RetrieveApiKeyResponse]
    """

    return (
        await asyncio_detailed(
            merchant_id=merchant_id,
            key_id=key_id,
            client=client,
            body=body,
        )
    ).parsed
