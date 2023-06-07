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

To convert non-secret variable into secret use `new()`. Sample:

```rust
expiry_year: ccard.map(|x| Secret::new(x.card_exp_year.to_string())),
// output: "expiry_year: *** alloc::string::String ***"
```

To get value from secret use `expose()`. Sample:

```rust
last4_digits: Some(card_number.expose())
```

Most fields are under `Option`. To simplify dealing with `Option`, use `expose_option()`. Sample:

```rust
    card_info.push_str(
        &card_detail
            .card_holder_name
            .expose_option()
            .unwrap_or_default(),
    );
```
