pub trait ForeignFrom<F> {
    fn foreign_from(from: F) -> Self;
}

pub trait ForeignTryFrom<F>: Sized {
    type Error;

    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}
