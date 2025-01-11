use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};

pub fn read<T: DeserializeOwned>(s: &[u8]) -> Result<T> {
    let v = ciborium::from_reader(s)?;
    Ok(v)
}

pub fn write<T: Serialize>(t: &T) -> Result<Vec<u8>> {
    let mut buf = vec![];
    ciborium::into_writer(t, &mut buf)?;
    Ok(buf)
}
