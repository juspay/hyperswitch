from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.merchant_account_response import MerchantAccountResponse
from ...models.merchant_account_update import MerchantAccountUpdate
from ...types import Response


def _get_kwargs(
    account_id: str,
    *,
    body: MerchantAccountUpdate,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": f"/accounts/{account_id}",
    }

    _body = body.to_dict()

    _kwargs["json"] = _body
    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, MerchantAccountResponse]]:
    if response.status_code == 200:
        response_200 = MerchantAccountResponse.from_dict(response.json())

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
) -> Response[Union[Any, MerchantAccountResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    account_id: str,
    *,
    client: AuthenticatedClient,
    body: MerchantAccountUpdate,
) -> Response[Union[Any, MerchantAccountResponse]]:
    """Merchant Account - Update

     Updates details of an existing merchant account. Helpful in updating merchant details such as email,
    contact details, or other configuration details like webhook, routing algorithm etc

    Args:
        account_id (str):
        body (MerchantAccountUpdate):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, MerchantAccountResponse]]
    """

    kwargs = _get_kwargs(
        account_id=account_id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    account_id: str,
    *,
    client: AuthenticatedClient,
    body: MerchantAccountUpdate,
) -> Optional[Union[Any, MerchantAccountResponse]]:
    """Merchant Account - Update

     Updates details of an existing merchant account. Helpful in updating merchant details such as email,
    contact details, or other configuration details like webhook, routing algorithm etc

    Args:
        account_id (str):
        body (MerchantAccountUpdate):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, MerchantAccountResponse]
    """

    return sync_detailed(
        account_id=account_id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    account_id: str,
    *,
    client: AuthenticatedClient,
    body: MerchantAccountUpdate,
) -> Response[Union[Any, MerchantAccountResponse]]:
    """Merchant Account - Update

     Updates details of an existing merchant account. Helpful in updating merchant details such as email,
    contact details, or other configuration details like webhook, routing algorithm etc

    Args:
        account_id (str):
        body (MerchantAccountUpdate):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, MerchantAccountResponse]]
    """

    kwargs = _get_kwargs(
        account_id=account_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    account_id: str,
    *,
    client: AuthenticatedClient,
    body: MerchantAccountUpdate,
) -> Optional[Union[Any, MerchantAccountResponse]]:
    """Merchant Account - Update

     Updates details of an existing merchant account. Helpful in updating merchant details such as email,
    contact details, or other configuration details like webhook, routing algorithm etc

    Args:
        account_id (str):
        body (MerchantAccountUpdate):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, MerchantAccountResponse]
    """

    return (
        await asyncio_detailed(
            account_id=account_id,
            client=client,
            body=body,
        )
    ).parsed
