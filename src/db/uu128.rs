extern crate uuid;

use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Uuid;
use std::io::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, FromSqlRow, AsExpression, Hash)]
#[sql_type = "Uuid"]
pub struct Uu128(uuid::Uuid);

impl From<u128> for Uu128 {
    fn from(value: u128) -> Uu128 {
        Uu128(uuid::Uuid::from_bytes(value.to_be_bytes()))
    }
}

impl From<Uu128> for u128 {
    fn from(value: Uu128) -> u128 {
        // Introduced in the next version of the uuid crate, v0.8
        let bytes = value.0.as_bytes();
        u128::from(bytes[0]) << 120
            | u128::from(bytes[1]) << 112
            | u128::from(bytes[2]) << 104
            | u128::from(bytes[3]) << 96
            | u128::from(bytes[4]) << 88
            | u128::from(bytes[5]) << 80
            | u128::from(bytes[6]) << 72
            | u128::from(bytes[7]) << 64
            | u128::from(bytes[8]) << 56
            | u128::from(bytes[9]) << 48
            | u128::from(bytes[10]) << 40
            | u128::from(bytes[11]) << 32
            | u128::from(bytes[12]) << 24
            | u128::from(bytes[13]) << 16
            | u128::from(bytes[14]) << 8
            | u128::from(bytes[15])
    }
}

impl FromSql<Uuid, Pg> for Uu128 {
    fn from_sql(bytes: Option<&[u8]>) -> deserialize::Result<Self> {
        let bytes = not_none!(bytes);

        match uuid::Uuid::from_slice(bytes) {
            Ok(v) => Ok(Uu128(v)),
            Err(e) => Err(e.into()),
        }
    }
}

impl ToSql<Uuid, Pg> for Uu128 {
    fn to_sql<W: Write>(&self, out: &mut Output<W, Pg>) -> serialize::Result {
        out.write_all(self.0.as_bytes())
            .map(|_| IsNull::No)
            .map_err(Into::into)
    }
}
