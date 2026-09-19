use crate::{
    digest::sha256,
    error::Error,
    format::{VSL_TYPE, header},
};

pub fn encode(payload: &[u8], digest: [u8; 32]) -> Vec<u8> {
    let mut data = header::encode(payload.len() as u64, digest);
    data.extend_from_slice(payload);
    data
}

pub fn decode(data: &[u8]) -> Result<&[u8], Error> {
    let (_, declared, digest) = header::decode(data)?;
    let available = data
        .len()
        .checked_sub(header::HEADER_LEN)
        .ok_or(Error::MalformedVessel("payload is absent".into()))?;
    let expected = usize::try_from(declared).map_err(|_| Error::PayloadLength {
        declared,
        available,
    })?;
    if expected != available {
        return Err(Error::PayloadLength {
            declared,
            available,
        });
    }
    let payload = &data[header::HEADER_LEN..];
    if sha256(payload) != digest {
        return Err(Error::PayloadDigest);
    }
    Ok(payload)
}
