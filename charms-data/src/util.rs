use anyhow::Result;
use ciborium_io::Read;
use core::fmt::Debug;
use serde::{de::DeserializeOwned, Serialize};

/// Deserialize a CBOR value from a reader (e.g. `&[u8]` or `std::io::stdin()`).
pub fn read<T, R>(s: R) -> Result<T>
where
    T: DeserializeOwned,
    R: Read,
    R::Error: Debug + Send + Sync + 'static,
{
    Ok(ciborium::from_reader(s)?)
}

/// Serialize a value to a byte vector as CBOR.
pub fn write<T>(t: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut buf = vec![];
    ciborium::into_writer(t, &mut buf)?;
    Ok(buf)
}
