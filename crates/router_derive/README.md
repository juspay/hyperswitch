# `router_derive`

Utility macros for the `router` crate.

This crate provides the following macros:

- `#[derive(DebugAsDisplay)]`: To use the `Debug` implementation of a type as its `Display` implementation.
- `#[derive(DieselEnum)]` and `#[diesel_enum]`: To derive the boilerplate code required to use enums with the `diesel` crate and a PostgreSQL database.
