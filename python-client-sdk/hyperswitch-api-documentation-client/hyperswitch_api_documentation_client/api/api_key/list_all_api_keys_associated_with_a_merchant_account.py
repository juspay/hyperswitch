from http import HTTPStatus
from typing import Any, Optional, Union

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.retrieve_api_key_response import RetrieveApiKeyResponse
from ...types import UNSET, Response, Unset


def _get_kwargs(
    merchant_id: str,
    *,
    limit: Union[None, Unset, int] = UNSET,
    skip: Union[None, Unset, int] = UNSET,
) -> dict[str, Any]:
    params: dict[str, Any] = {}

    json_limit: Union[None, Unset, int]
    if isinstance(limit, Unset):
        json_limit = UNSET
    else:
        json_limit = limit
    params["limit"] = json_limit

    json_skip: Union[None, Unset, int]
    if isinstance(skip, Unset):
        json_skip = UNSET
    else:
        json_skip = skip
    params["skip"] = json_skip

    params = {k: v for k, v in params.items() if v is not UNSET and v is not None}

    _kwargs: dict[str, Any] = {
        "method": "get",
        "url": f"/api_keys/{merchant_id}/list",
        "params": params,
    }

    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[list["RetrieveApiKeyResponse"]]:
    if response.status_code == 200:
        response_200 = []
        _response_200 = response.json()
        for response_200_item_data in _response_200:
            response_200_item = RetrieveApiKeyResponse.from_dict(response_200_item_data)

            response_200.append(response_200_item)

        return response_200
    if client.raise_on_unexpected_status:
        raise errors.UnexpectedStatus(response.status_code, response.content)
    else:
        return None


def _build_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Response[list["RetrieveApiKeyResponse"]]:
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
    limit: Union[None, Unset, int] = UNSET,
    skip: Union[None, Unset, int] = UNSET,
) -> Response[list["RetrieveApiKeyResponse"]]:
    """API Key - List

     List all the API Keys associated to a merchant account.

    Args:
        merchant_id (str):
        limit (Union[None, Unset, int]):
        skip (Union[None, Unset, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[list['RetrieveApiKeyResponse']]
    """

    kwargs = _get_kwargs(
        merchant_id=merchant_id,
        limit=limit,
        skip=skip,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    merchant_id: str,
    *,
    client: AuthenticatedClient,
    limit: Union[None, Unset, int] = UNSET,
    skip: Union[None, Unset, int] = UNSET,
) -> Optional[list["RetrieveApiKeyResponse"]]:
    """API Key - List

     List all the API Keys associated to a merchant account.

    Args:
        merchant_id (str):
        limit (Union[None, Unset, int]):
        skip (Union[None, Unset, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        list['RetrieveApiKeyResponse']
    """

    return sync_detailed(
        merchant_id=merchant_id,
        client=client,
        limit=limit,
        skip=skip,
    ).parsed


async def asyncio_detailed(
    merchant_id: str,
    *,
    client: AuthenticatedClient,
    limit: Union[None, Unset, int] = UNSET,
    skip: Union[None, Unset, int] = UNSET,
) -> Response[list["RetrieveApiKeyResponse"]]:
    """API Key - List

     List all the API Keys associated to a merchant account.

    Args:
        merchant_id (str):
        limit (Union[None, Unset, int]):
        skip (Union[None, Unset, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[list['RetrieveApiKeyResponse']]
    """

    kwargs = _get_kwargs(
        merchant_id=merchant_id,
        limit=limit,
        skip=skip,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    merchant_id: str,
    *,
    client: AuthenticatedClient,
    limit: Union[None, Unset, int] = UNSET,
    skip: Union[None, Unset, int] = UNSET,
) -> Optional[list["RetrieveApiKeyResponse"]]:
    """API Key - List

     List all the API Keys associated to a merchant account.

    Args:
        merchant_id (str):
        limit (Union[None, Unset, int]):
        skip (Union[None, Unset, int]):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        list['RetrieveApiKeyResponse']
    """

    return (
        await asyncio_detailed(
            merchant_id=merchant_id,
            client=client,
            limit=limit,
            skip=skip,
        )
    ).parsed
