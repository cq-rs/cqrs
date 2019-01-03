use postgres::types::{FromSql, ToSql, Type, IsNull, JSON, JSONB, BYTEA};
use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error, fmt};

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Json<T>(pub T);

impl<T> FromSql for Json<T>
where T: DeserializeOwned
{
    fn from_sql(ty: &Type, mut raw: &[u8]) -> Result<Json<T>, Box<dyn Error + Sync + Send>> {
        use std::io::Read;
        if *ty == JSONB {
            let mut b = [0; 1];
            raw.read_exact(&mut b)?;
            // We only support version 1 of the jsonb binary format
            if b[0] != 1 {
                return Err("unsupported JSONB encoding version".into());
            }
        }
        serde_json::from_slice(raw)
            .map(Json)
            .map_err(From::from)
    }

    postgres::accepts!(JSON, JSONB, BYTEA);
}

impl<T> ToSql for Json<T>
where T: Serialize + fmt::Debug
{
    fn to_sql(&self, ty: &Type, out: &mut Vec<u8>) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        if *ty == JSONB {
            out.push(1);
        }
        serde_json::to_writer(out, &self.0)?;

        Ok(IsNull::No)
    }

    postgres::accepts!(JSON, JSONB, BYTEA);
    postgres::to_sql_checked!();
}
