pub trait ForeignFrom<F> {
    fn foreign_from(from: F) -> Self;
}
