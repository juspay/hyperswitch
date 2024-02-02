//!
//! Diesel-related.
//!

use diesel::{
    backend::Backend,
    deserialize::{self, FromSql, Queryable},
    expression::AsExpression,
    internal::derives::as_expression::Bound,
    serialize::{self, Output, ToSql},
    sql_types,
};

use crate::{Secret, Strategy, StrongSecret, ZeroizableSecret};

impl<'expr, S, I, T> AsExpression<T> for &'expr Secret<S, I>
where
    T: sql_types::SingleValue,
    I: Strategy<S>,
{
    type Expression = Bound<T, Self>;
        /// Converts the current type into its corresponding expression type.
    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<'expr2, 'expr, S, I, T> AsExpression<T> for &'expr2 &'expr Secret<S, I>
where
    T: sql_types::SingleValue,
    I: Strategy<S>,
{
    type Expression = Bound<T, Self>;
    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<S, I, T, DB> ToSql<T, DB> for Secret<S, I>
where
    DB: Backend,
    S: ToSql<T, DB>,
    I: Strategy<S>,
{
        /// Converts the inner secret value to its SQL representation and writes it to the specified output.
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
            ToSql::<T, DB>::to_sql(&self.inner_secret, out)
        }
}

impl<DB, S, T, I> FromSql<T, DB> for Secret<S, I>
where
    DB: Backend,
    S: FromSql<T, DB>,
    I: Strategy<S>,
{
        /// This method converts a raw value from the database into a value of type `Self` using the `from_sql` method of the associated type `S`. It then maps the raw value to `Self` using the `into` method.
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        S::from_sql(bytes).map(|raw| raw.into())
    }
}

impl<S, I, T> AsExpression<T> for Secret<S, I>
where
    T: sql_types::SingleValue,
    I: Strategy<S>,
{
    type Expression = Bound<T, Self>;
    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<ST, DB, S, I> Queryable<ST, DB> for Secret<S, I>
where
    DB: Backend,
    I: Strategy<S>,
    ST: sql_types::SingleValue,
    Self: FromSql<ST, DB>,
{
    type Row = Self;
        /// Builds a new instance of Self using the provided `row` and returns a Result containing the new instance.
    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}

impl<'expr, S, I, T> AsExpression<T> for &'expr StrongSecret<S, I>
where
    T: sql_types::SingleValue,
    S: ZeroizableSecret,
    I: Strategy<S>,
{
    type Expression = Bound<T, Self>;
    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<'expr2, 'expr, S, I, T> AsExpression<T> for &'expr2 &'expr StrongSecret<S, I>
where
    T: sql_types::SingleValue,
    S: ZeroizableSecret,
    I: Strategy<S>,
{
    type Expression = Bound<T, Self>;
    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<S, I, DB, T> ToSql<T, DB> for StrongSecret<S, I>
where
    DB: Backend,
    S: ToSql<T, DB> + ZeroizableSecret,
    I: Strategy<S>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        ToSql::<T, DB>::to_sql(&self.inner_secret, out)
    }
}

impl<DB, S, I, T> FromSql<T, DB> for StrongSecret<S, I>
where
    DB: Backend,
    S: FromSql<T, DB> + ZeroizableSecret,
    I: Strategy<S>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        S::from_sql(bytes).map(|raw| raw.into())
    }
}

impl<S, I, T> AsExpression<T> for StrongSecret<S, I>
where
    T: sql_types::SingleValue,
    S: ZeroizableSecret,
    I: Strategy<S>,
{
    type Expression = Bound<T, Self>;
    fn as_expression(self) -> Self::Expression {
        Bound::new(self)
    }
}

impl<ST, DB, S, I> Queryable<ST, DB> for StrongSecret<S, I>
where
    I: Strategy<S>,
    DB: Backend,
    S: ZeroizableSecret,
    ST: sql_types::SingleValue,
    Self: FromSql<ST, DB>,
{
    type Row = Self;
    fn build(row: Self::Row) -> deserialize::Result<Self> {
        Ok(row)
    }
}
