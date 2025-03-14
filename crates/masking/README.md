# Masking

Personal Identifiable Information protection.
Wrapper types and traits for secret management which help ensure they aren't
accidentally copied, logged, or otherwise exposed (as much as possible), and
also ensure secrets are securely wiped from memory when dropped.
Secret-keeping library inspired by `secrecy`.

This solution has such advantages over alternatives:

- alternatives have not implemented several traits from the box which are needed
- alternatives do not have WeakSecret and Secret differentiation
- alternatives do not support masking strategies
- alternatives had several minor problems

## How to use

To convert a non-secret variable into a secret, use `Secret::new()`:

```rust
use masking::Secret;

let card_number: Secret<String> = Secret::new(String::from("1234 5678 9012 3456"));
assert_eq!(format!("{:?}", card_number), "*** alloc::string::String ***");
```

To get a reference to the inner value from the secret, use `peek()`:

```rust
use masking::{PeekInterface, Secret};

let card_number: Secret<String> = Secret::new(String::from("1234 5678 9012 3456"));
let last4_digits: &str = card_number.peek();
```

To get the owned inner value from the secret, use `expose()`:

```rust
use masking::{ExposeInterface, Secret};

let card_number: Secret<String> = Secret::new(String::from("1234 5678 9012 3456"));
let last4_digits: String = card_number.expose();
```

For fields that are `Option<T>`, you can use `expose_option()`:

```rust
use masking::{ExposeOptionInterface, Secret};

let card_number: Option<Secret<String>> = Some(Secret::new(String::from("1234 5678 9012 3456")));
let card_number_str: String = card_number.expose_option().unwrap_or_default();
assert_eq!(format!("{}", card_number_str), "1234 5678 9012 3456");

let card_number: Option<Secret<String>> = None;
let card_number_str: String = card_number.expose_option().unwrap_or_default();
assert_eq!(format!("{}", card_number_str), "");
```
