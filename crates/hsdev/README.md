# HSDEV

`hsdev` is a simple diesel Postgres migration tool. It is designed to simply running a Postgres database migration with diesel.

## Installing hsdev
`hsdev` can be installed using `cargo`
```shell
cargo install --force --path crates/hsdev
```

## Using hsdev
Using `hsdev` is simple. All you need to do is run the following command.
```shell
hsdev --toml-file [path/to/TOML/file]
```

provide `hsdev` with a TOML file containing the following keys:
```toml
username = "your_username"
password = "your_password"
dbname = "your_db_name"
```

Simply run the command and let `hsdev` handle the rest.
