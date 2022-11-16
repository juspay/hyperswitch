pub trait Kind<A> {
    type Wrapped;
    type Wrapper;
}

pub trait Applicative<F>: Kind<F> {
    fn pure(v: Self::Wrapped) -> Self::Wrapper;
}

impl<F, A> Kind<F> for Option<A> {
    type Wrapped = A;
    type Wrapper = Option<F>;
}

impl<F, A, B> Kind<F> for Result<A, B> {
    type Wrapped = A;
    type Wrapper = Result<F, B>;
}

impl<A> Applicative<A> for Option<A> {
    fn pure(v: A) -> Self::Wrapper {
        Some(v)
    }
}

impl<A, E> Applicative<A> for Result<A, E> {
    fn pure(v: A) -> Self::Wrapper {
        Ok(v)
    }
}

// FIXME: This method potentially encourages its users to allocate+free resources without need
// for example, in `check_value_present` below, this function is used as follows:
// when(
//     self.is_none(),
//     Err(Report::new(ValidateError)
//         .attach_printable(format!("In {self:?} {key} has not found"))),
// )
// This code allocates a `String` because of format! macro, and potentially allocates inside an error.
// The it should either replaced with `if` or the alternate argument should be a callback.
// Maybe there are other places with extra allocation?
pub fn when<F: Applicative<()> + Kind<(), Wrapped = (), Wrapper = F>>(predicate: bool, f: F) -> F {
    if predicate {
        f
    } else {
        F::pure(())
    }
}
