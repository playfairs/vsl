use crate::{
    error::Error,
    format::{MAGIC, version::CURRENT},
};

pub const HEADER_LEN: usize = 48;

pub fn encode(payload_len: u64, digest: [u8; 32]) -> Vec<u8> {
    let mut header = Vec::with_capacity(HEADER_LEN);
    header.extend_from_slice(&MAGIC);
    header.extend_from_slice(&CURRENT.to_be_bytes());
    header.extend_from_slice(&0u16.to_be_bytes());
    header.extend_from_slice(&payload_len.to_be_bytes());
    header.extend_from_slice(&digest);
    header
}

pub fn decode(data: &[u8]) -> Result<(u16, u64, [u8; 32]), Error> {
    if data.len() < HEADER_LEN {
        return Err(Error::HeaderTooShort);
    }
    if data[..4] != MAGIC {
        return Err(Error::InvalidMagic);
    }
    let version = u16::from_be_bytes(
        data[4..6]
            .try_into()
            .map_err(|_| Error::VersionTruncated)?,
    );
    let flags = u16::from_be_bytes(
        data[6..8]
            .try_into()
            .map_err(|_| Error::FlagsTruncated)?,
    );
    if flags != 0 {
        return Err(Error::MalformedVessel("unsupported flags are set".into()));
    }
    let payload_len = u64::from_be_bytes(
        data[8..16]
            .try_into()
            .map_err(|_| Error::PayloadLengthTruncated)?,
    );
    let digest: [u8; 32] = data[16..48]
        .try_into()
        .map_err(|_| Error::DigestTruncated)?;
    if version != CURRENT {
        return Err(Error::UnsupportedVersion(version));
    }
    Ok((version, payload_len, digest))
}
