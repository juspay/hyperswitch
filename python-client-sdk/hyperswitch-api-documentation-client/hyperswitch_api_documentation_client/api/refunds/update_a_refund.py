from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.refund_response import RefundResponse
from ...models.refund_update_request import RefundUpdateRequest
from ...types import Response


def _get_kwargs(
    refund_id: str,
    *,
    body: RefundUpdateRequest,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": f"/refunds/{refund_id}",
    }

    _body = body.to_dict()

    _kwargs["json"] = _body
    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, RefundResponse]]:
    if response.status_code == 200:
        response_200 = RefundResponse.from_dict(response.json())

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
) -> Response[Union[Any, RefundResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    refund_id: str,
    *,
    client: AuthenticatedClient,
    body: RefundUpdateRequest,
) -> Response[Union[Any, RefundResponse]]:
    """Refunds - Update

     Updates the properties of a Refund object. This API can be used to attach a reason for the refund or
    metadata fields

    Args:
        refund_id (str):
        body (RefundUpdateRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RefundResponse]]
    """

    kwargs = _get_kwargs(
        refund_id=refund_id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    refund_id: str,
    *,
    client: AuthenticatedClient,
    body: RefundUpdateRequest,
) -> Optional[Union[Any, RefundResponse]]:
    """Refunds - Update

     Updates the properties of a Refund object. This API can be used to attach a reason for the refund or
    metadata fields

    Args:
        refund_id (str):
        body (RefundUpdateRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RefundResponse]
    """

    return sync_detailed(
        refund_id=refund_id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    refund_id: str,
    *,
    client: AuthenticatedClient,
    body: RefundUpdateRequest,
) -> Response[Union[Any, RefundResponse]]:
    """Refunds - Update

     Updates the properties of a Refund object. This API can be used to attach a reason for the refund or
    metadata fields

    Args:
        refund_id (str):
        body (RefundUpdateRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, RefundResponse]]
    """

    kwargs = _get_kwargs(
        refund_id=refund_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    refund_id: str,
    *,
    client: AuthenticatedClient,
    body: RefundUpdateRequest,
) -> Optional[Union[Any, RefundResponse]]:
    """Refunds - Update

     Updates the properties of a Refund object. This API can be used to attach a reason for the refund or
    metadata fields

    Args:
        refund_id (str):
        body (RefundUpdateRequest):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, RefundResponse]
    """

    return (
        await asyncio_detailed(
            refund_id=refund_id,
            client=client,
            body=body,
        )
    ).parsed
