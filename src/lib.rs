pub mod bundle;
pub mod digest;
pub mod error;
pub mod format;

pub use format::{MAGIC, VSL_TYPE};

pub const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
