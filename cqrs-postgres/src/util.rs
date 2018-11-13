use postgres::types::{FromSql, ToSql, Type, IsNull, JSON, JSONB, BYTEA};
use cqrs_core::{PersistableAggregate, SerializableEvent};
use std::{error::Error, fmt, io::{self, Read, Seek}, str};

#[derive(Debug)]
pub struct RawJson(pub Vec<u8>);

impl FromSql for RawJson {
    fn from_sql(ty: &Type, mut raw: &[u8]) -> Result<RawJson, Box<Error + Sync + Send>> {
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

    accepts!(JSON, JSONB, BYTEA);
}

impl ToSql for RawJson {
    fn to_sql(&self, ty: &Type, out: &mut Vec<u8>) -> Result<IsNull, Box<Error + Sync + Send>> {
        if *ty == JSONB {
            out.push(1);
        }
        if self.0.is_empty() {
            out.extend_from_slice("null".as_ref());
        } else {
            out.extend_from_slice(&self.0);
        }
        Ok(IsNull::No)
    }

    accepts!(JSON, JSONB, BYTEA);
    to_sql_checked!();
}

impl fmt::Display for RawJson {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Ok(data) = str::from_utf8(&self.0) {
            f.write_str(data)
        } else {
            f.write_str("<invalid UTF-8>")
        }
    }
}

#[derive(Debug)]
pub struct PostgresEventView<'e, E: SerializableEvent + fmt::Debug + 'e>(pub &'e E);

impl<'e, E: SerializableEvent + fmt::Debug + 'e> ToSql for PostgresEventView<'e, E> {
    fn to_sql(&self, ty: &Type, out: &mut Vec<u8>) -> Result<IsNull, Box<Error + Sync + Send>> {
        if *ty == JSONB {
            out.push(1);
        }

        let mut cursor = io::Cursor::new(out);
        cursor.seek(io::SeekFrom::End(0))?;
        self.0.serialize_to_writer(&mut cursor)?;

        Ok(IsNull::No)
    }

    accepts!(JSON, JSONB, BYTEA);
    to_sql_checked!();
}

#[derive(Debug)]
pub struct PostgresSnapshotView<'a, A: PersistableAggregate + fmt::Debug + 'a>(pub &'a A);

impl<'a, A: PersistableAggregate + fmt::Debug + 'a> ToSql for PostgresSnapshotView<'a, A> {
    fn to_sql(&self, ty: &Type, out: &mut Vec<u8>) -> Result<IsNull, Box<Error + Sync + Send>> {
        if *ty == JSONB {
            out.push(1);
        }

        let mut cursor = io::Cursor::new(out);
        cursor.seek(io::SeekFrom::End(0))?;
        self.0.snapshot_to_writer(&mut cursor)?;

        Ok(IsNull::No)
    }

    accepts!(JSON, JSONB, BYTEA);
    to_sql_checked!();
}
