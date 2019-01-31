use postgres::types::{FromSql, IsNull, ToSql, Type, BYTEA, INT8, JSON, JSONB};
use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct BorrowedJson<'a, T>(pub &'a T);

impl<'a, T> ToSql for BorrowedJson<'a, T>
where
    T: Serialize + fmt::Debug + 'a,
{
    postgres::accepts!(JSON, JSONB, BYTEA);

    postgres::to_sql_checked!();

    fn to_sql(&self, ty: &Type, out: &mut Vec<u8>) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        if *ty == JSONB {
            out.push(1);
        }
        serde_json::to_writer(out, self.0)?;

        Ok(IsNull::No)
    }
}

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Json<T>(pub T);

impl<T> FromSql for Json<T>
where
    T: DeserializeOwned,
{
    postgres::accepts!(JSON, JSONB, BYTEA);

    fn from_sql(ty: &Type, mut raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        use std::io::Read;
        if *ty == JSONB {
            let mut b = [0; 1];
            raw.read_exact(&mut b)?;
            // We only support version 1 of the jsonb binary format
            if b[0] != 1 {
                return Err("unsupported JSONB encoding version".into());
            }
        }
        serde_json::from_slice(raw).map(Json).map_err(From::from)
    }
}

impl<T> ToSql for Json<T>
where
    T: Serialize + fmt::Debug,
{
    postgres::accepts!(JSON, JSONB, BYTEA);

    postgres::to_sql_checked!();

    fn to_sql(&self, ty: &Type, out: &mut Vec<u8>) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        if *ty == JSONB {
            out.push(1);
        }
        serde_json::to_writer(out, &self.0)?;

        Ok(IsNull::No)
    }
}

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct RawJsonPersist<'a>(pub &'a [u8]);

impl<'a> ToSql for RawJsonPersist<'a> {
    postgres::accepts!(JSON, JSONB, BYTEA);

    postgres::to_sql_checked!();

    fn to_sql(&self, ty: &Type, out: &mut Vec<u8>) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        if *ty == JSONB {
            out.push(1);
        }
        out.extend_from_slice(self.0);

        Ok(IsNull::No)
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct RawJsonRead(pub Vec<u8>);

impl FromSql for RawJsonRead {
    postgres::accepts!(JSON, JSONB, BYTEA);

    fn from_sql(ty: &Type, mut raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        use std::io::Read;
        if *ty == JSONB {
            let mut b = [0; 1];
            raw.read_exact(&mut b)?;
            // We only support version 1 of the jsonb binary format
            if b[0] != 1 {
                return Err("unsupported JSONB encoding version".into());
            }
        }
        Ok(RawJsonRead(Vec::from(raw)))
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Sequence(pub cqrs_core::EventNumber);

impl FromSql for Sequence {
    postgres::accepts!(INT8);

    fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        let value = i64::from_sql(ty, raw)?;
        if value < 0 {
            return Err("Invalid event sequence number, negative values are not allowed".into());
        }
        let event_number = cqrs_core::EventNumber::new(value as u64)
            .ok_or_else(|| "Invalid event sequence number, zero is not a valid sequence number")?;
        Ok(Sequence(event_number))
    }
}
