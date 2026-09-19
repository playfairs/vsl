use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("VSL payload cannot be empty")]
    EmptyPayload,
    #[error("VSL payload is malformed: {0}")]
    MalformedVessel(String),
    #[error("VSL format version {0} is unsupported")]
    UnsupportedVersion(u16),
    #[error("VSL payload length is invalid: declared {declared}, available {available}")]
    PayloadLength { declared: u64, available: usize },
    #[error("VSL payload digest does not match")]
    PayloadDigest,
    #[error("VSL header is shorter than 48 bytes")]
    HeaderTooShort,
    #[error("VSL magic is invalid")]
    InvalidMagic,
    #[error("VSL version is truncated")]
    VersionTruncated,
    #[error("VSL flags are truncated")]
    FlagsTruncated,
    #[error("VSL payload length is truncated")]
    PayloadLengthTruncated,
    #[error("VSL digest is truncated")]
    DigestTruncated,
    #[error("VSL bundle is malformed: {0}")]
    MalformedBundle(String),
    #[error("VSL bundle entry type is invalid")]
    InvalidBundleEntryType,
    #[error("VSL bundle path must be relative and traversal-free")]
    InvalidBundlePath,
    #[error("VSL bundle file is truncated")]
    BundleTruncated,
    #[error("VSL bundle has trailing data")]
    TrailingBundleData,
}

pub fn io_error(path: &std::path::Path, source: std::io::Error) -> Error {
    Error::MalformedBundle(format!("I/O error for {}: {}", path.display(), source))
}
