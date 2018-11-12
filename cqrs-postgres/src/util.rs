use postgres::types::{FromSql, ToSql, Type, IsNull, JSON, JSONB};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::ser::Serializer;
use std::{error::Error, fmt, io::Cursor};

#[derive(Debug)]
pub struct Json<T>(pub T);

impl<T> FromSql for Json<T>
    where T: DeserializeOwned
{
    fn from_sql(ty: &Type, mut raw: &[u8]) -> Result<Json<T>, Box<Error + Sync + Send>> {
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

    accepts!(JSON, JSONB);
}

impl<T> ToSql for Json<T>
where T: Serialize + fmt::Debug
{
    fn to_sql(&self, ty: &Type, out: &mut Vec<u8>) -> Result<IsNull, Box<Error + Sync + Send>> {
        if *ty == JSONB {
            out.push(1);
        }
        let mut serializer = Serializer::new(Cursor::new(out));
        self.0.serialize(&mut serializer)?;
        Ok(IsNull::No)
    }

    accepts!(JSON, JSONB);
    to_sql_checked!();
}

#[derive(Debug)]
pub struct RawJson(pub Vec<u8>);

impl FromSql for RawJson
{
    fn from_sql(ty: &Type, mut raw: &[u8]) -> Result<RawJson, Box<Error + Sync + Send>> {
        use std::io::Read;
        if *ty == JSONB {
            let mut b = [0; 1];
            raw.read_exact(&mut b)?;
            // We only support version 1 of the jsonb binary format
            if b[0] != 1 {
                return Err("unsupported JSONB encoding version".into());
            }
        }
        Ok(RawJson(raw.into()))
    }

    accepts!(JSON, JSONB);
}

impl ToSql for RawJson {
    fn to_sql(&self, ty: &Type, out: &mut Vec<u8>) -> Result<IsNull, Box<Error + Sync + Send>> {
        if *ty == JSONB {
            out.push(1);
        }
        if self.0.is_empty() {
            out.extend_from_slice(&['{' as u8, '}' as u8]);
        } else {
            out.extend_from_slice(&self.0);
        }
        Ok(IsNull::No)
    }

    accepts!(JSON, JSONB);
    to_sql_checked!();
}

impl fmt::Display for RawJson {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Ok(data) = ::std::str::from_utf8(&self.0) {
            f.write_str(data)
        } else {
            f.write_str("<invalid UTF-8>")
        }
    }
}
