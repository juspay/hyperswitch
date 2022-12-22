pub trait Applicative<R> {
    type WrappedSelf<T>;

    fn pure(v: R) -> Self::WrappedSelf<R>;
}

impl<R> Applicative<R> for Option<R> {
    type WrappedSelf<T> = Option<T>;
    fn pure(v: R) -> Self::WrappedSelf<R> {
        Some(v)
    }
}

impl<R, E> Applicative<R> for Result<R, E> {
    type WrappedSelf<T> = Result<T, E>;
    fn pure(v: R) -> Self::WrappedSelf<R> {
        Ok(v)
    }
}

// This function allows lazy evaluation of the `f` argument
pub fn when<W: Applicative<(), WrappedSelf<()> = W>, F>(predicate: bool, f: F) -> W
where
    F: FnOnce() -> W,
{
    if predicate {
        f()
    } else {
        W::pure(())
    }
}
