use diesel::{backend::Backend, deserialize::FromSql, serialize::ToSql, sql_types, AsExpression};

#[derive(Debug, AsExpression)]
#[diesel(sql_type = diesel::sql_types::Binary)]
#[repr(transparent)]
pub struct Encryption {
    inner: Vec<u8>,
}

impl Encryption {
    pub fn new(item: Vec<u8>) -> Self {
        Self { inner: item }
    }

    #[inline]
    pub fn into_inner(self) -> Vec<u8> {
        self.inner
    }

    #[inline]
    pub fn get_inner(&self) -> &Vec<u8> {
        &self.inner
    }
}

impl<DB> FromSql<sql_types::Binary, DB> for Encryption
where
    DB: Backend,
    Vec<u8>: FromSql<sql_types::Binary, DB>,
{
    fn from_sql(bytes: diesel::backend::RawValue<'_, DB>) -> diesel::deserialize::Result<Self> {
        <Vec<u8>>::from_sql(bytes).map(Self::new)
    }
}

impl<DB> ToSql<sql_types::Binary, DB> for Encryption
where
    DB: Backend,
    Vec<u8>: ToSql<sql_types::Binary, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        self.get_inner().to_sql(out)
    }
}
