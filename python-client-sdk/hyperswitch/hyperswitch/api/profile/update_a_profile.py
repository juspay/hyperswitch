from http import HTTPStatus
from typing import Any, Optional, Union, cast

import httpx

from ... import errors
from ...client import AuthenticatedClient, Client
from ...models.profile_create import ProfileCreate
from ...models.profile_response import ProfileResponse
from ...types import Response


def _get_kwargs(
    account_id: str,
    profile_id: str,
    *,
    body: ProfileCreate,
) -> dict[str, Any]:
    headers: dict[str, Any] = {}

    _kwargs: dict[str, Any] = {
        "method": "post",
        "url": f"/account/{account_id}/business_profile/{profile_id}",
    }

    _body = body.to_dict()

    _kwargs["json"] = _body
    headers["Content-Type"] = "application/json"

    _kwargs["headers"] = headers
    return _kwargs


def _parse_response(
    *, client: Union[AuthenticatedClient, Client], response: httpx.Response
) -> Optional[Union[Any, ProfileResponse]]:
    if response.status_code == 200:
        response_200 = ProfileResponse.from_dict(response.json())

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
) -> Response[Union[Any, ProfileResponse]]:
    return Response(
        status_code=HTTPStatus(response.status_code),
        content=response.content,
        headers=response.headers,
        parsed=_parse_response(client=client, response=response),
    )


def sync_detailed(
    account_id: str,
    profile_id: str,
    *,
    client: AuthenticatedClient,
    body: ProfileCreate,
) -> Response[Union[Any, ProfileResponse]]:
    """Profile - Update

     Update the *profile*

    Args:
        account_id (str):
        profile_id (str):
        body (ProfileCreate):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, ProfileResponse]]
    """

    kwargs = _get_kwargs(
        account_id=account_id,
        profile_id=profile_id,
        body=body,
    )

    response = client.get_httpx_client().request(
        **kwargs,
    )

    return _build_response(client=client, response=response)


def sync(
    account_id: str,
    profile_id: str,
    *,
    client: AuthenticatedClient,
    body: ProfileCreate,
) -> Optional[Union[Any, ProfileResponse]]:
    """Profile - Update

     Update the *profile*

    Args:
        account_id (str):
        profile_id (str):
        body (ProfileCreate):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, ProfileResponse]
    """

    return sync_detailed(
        account_id=account_id,
        profile_id=profile_id,
        client=client,
        body=body,
    ).parsed


async def asyncio_detailed(
    account_id: str,
    profile_id: str,
    *,
    client: AuthenticatedClient,
    body: ProfileCreate,
) -> Response[Union[Any, ProfileResponse]]:
    """Profile - Update

     Update the *profile*

    Args:
        account_id (str):
        profile_id (str):
        body (ProfileCreate):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Response[Union[Any, ProfileResponse]]
    """

    kwargs = _get_kwargs(
        account_id=account_id,
        profile_id=profile_id,
        body=body,
    )

    response = await client.get_async_httpx_client().request(**kwargs)

    return _build_response(client=client, response=response)


async def asyncio(
    account_id: str,
    profile_id: str,
    *,
    client: AuthenticatedClient,
    body: ProfileCreate,
) -> Optional[Union[Any, ProfileResponse]]:
    """Profile - Update

     Update the *profile*

    Args:
        account_id (str):
        profile_id (str):
        body (ProfileCreate):

    Raises:
        errors.UnexpectedStatus: If the server returns an undocumented status code and Client.raise_on_unexpected_status is True.
        httpx.TimeoutException: If the request takes longer than Client.timeout.

    Returns:
        Union[Any, ProfileResponse]
    """

    return (
        await asyncio_detailed(
            account_id=account_id,
            profile_id=profile_id,
            client=client,
            body=body,
        )
    ).parsed
