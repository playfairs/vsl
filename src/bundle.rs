use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

use crate::error::Error;

const MAGIC: &[u8; 8] = b"VSLBND01";
const VERSION: u16 = 1;
const FILE: u8 = 1;
const DIRECTORY: u8 = 2;
const SYMLINK: u8 = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleEntry {
    pub path: String,
    pub kind: BundleEntryKind,
    pub mode: u32,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BundleEntryKind {
    File,
    Directory,
    Symlink,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bundle {
    pub entrypoint: String,
    pub entries: Vec<BundleEntry>,
}

pub fn pack(bundle_root: &Path) -> Result<Vec<u8>, Error> {
    if !bundle_root.is_dir() {
        return Err(Error::MalformedBundle("bundle source is not a directory".into()));
    }
    let name = bundle_root
        .file_stem()
        .ok_or_else(|| Error::MalformedBundle("bundle has no name".into()))?;
    let entrypoint = format!("Contents/MacOS/{}", name.to_string_lossy());
    if !bundle_root.join(&entrypoint).is_file() {
        return Err(Error::MalformedBundle("bundle entrypoint is missing".into()));
    }
    let mut entries = Vec::new();
    collect(bundle_root, Path::new(""), &mut entries)?;
    let mut output = Vec::new();
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&VERSION.to_be_bytes());
    output.extend_from_slice(&(entries.len() as u32).to_be_bytes());
    output.extend_from_slice(&(entrypoint.len() as u16).to_be_bytes());
    output.extend_from_slice(entrypoint.as_bytes());

    for entry in entries {
        let kind = match entry.kind {
            BundleEntryKind::File => FILE,
            BundleEntryKind::Directory => DIRECTORY,
            BundleEntryKind::Symlink => SYMLINK,
        };
        let path_len = u32::try_from(entry.path.len()).map_err(|_| Error::MalformedBundle("bundle path is too long".into()))?;
        let data_len = u64::try_from(entry.data.len()).map_err(|_| Error::MalformedBundle("bundle entry is too large".into()))?;
        output.push(kind);
        output.extend_from_slice(&entry.mode.to_be_bytes());
        output.extend_from_slice(&path_len.to_be_bytes());
        output.extend_from_slice(&data_len.to_be_bytes());
        output.extend_from_slice(entry.path.as_bytes());
        output.extend_from_slice(&entry.data);
    }
    Ok(output)
}

pub fn unpack(payload: &[u8], destination: &Path) -> Result<PathBuf, Error> {
    let mut cursor = 0;
    if take(payload, &mut cursor, MAGIC.len())? != MAGIC {
        return Err(Error::MalformedBundle("bundle magic is invalid".into()));
    }
    let version = read_u16(payload, &mut cursor)?;
    if version != VERSION {
        return Err(Error::UnsupportedVersion(version));
    }
    let count = usize::try_from(read_u32(payload, &mut cursor)?).map_err(|_| Error::MalformedBundle("bundle entry count is invalid".into()))?;
    let entrypoint_len = usize::from(read_u16(payload, &mut cursor)?);
    let entrypoint = String::from_utf8(take(payload, &mut cursor, entrypoint_len)?.to_vec()).map_err(|_| Error::MalformedBundle("bundle entrypoint is not UTF-8".into()))?;
    validate_relative(Path::new(&entrypoint))?;
    for _ in 0..count {
        let kind = take(payload, &mut cursor, 1)?[0];
        if ![FILE, DIRECTORY, SYMLINK].contains(&kind) {
            return Err(Error::InvalidBundleEntryType);
        }
        let mode = read_u32(payload, &mut cursor)?;
        let path_len = usize::try_from(read_u32(payload, &mut cursor)?).map_err(|_| Error::MalformedBundle("bundle path length is invalid".into()))?;
        let data_len = usize::try_from(read_u64(payload, &mut cursor)?).map_err(|_| Error::MalformedBundle("bundle data length is invalid".into()))?;
        let path = String::from_utf8(take(payload, &mut cursor, path_len)?.to_vec()).map_err(|_| Error::MalformedBundle("bundle path is not UTF-8".into()))?;
        let relative = Path::new(&path);
        validate_relative(relative)?;
        let data = take(payload, &mut cursor, data_len)?;
        let target = destination.join(relative);
        match kind {
            DIRECTORY => {
                fs::create_dir_all(&target).map_err(|source| crate::error::io_error(&target, source))?;
                set_mode(&target, mode)?;
            }
            FILE => {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|source| crate::error::io_error(parent, source))?;
                }
                fs::write(&target, data).map_err(|source| crate::error::io_error(&target, source))?;
                set_mode(&target, mode)?;
            }
            SYMLINK => {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|source| crate::error::io_error(parent, source))?;
                }
                let link = String::from_utf8(data.to_vec()).map_err(|_| Error::MalformedBundle("bundle symlink target is not UTF-8".into()))?;
                validate_relative(Path::new(&link))?;
                #[cfg(unix)]
                std::os::unix::fs::symlink(link, &target).map_err(|source| crate::error::io_error(&target, source))?;
                #[cfg(not(unix))]
                return Err(Error::MalformedBundle("bundle symlinks are unsupported on this platform".into()));
            }
            _ => unreachable!(),
        }
    }
    if cursor != payload.len() {
        return Err(Error::TrailingBundleData);
    }
    Ok(destination.join(entrypoint))
}

pub fn is_bundle(payload: &[u8]) -> bool {
    payload.starts_with(MAGIC)
}

fn collect(root: &Path, relative: &Path, entries: &mut Vec<BundleEntry>) -> Result<(), Error> {
    let directory = root.join(relative);
    let mut children = fs::read_dir(&directory)
        .map_err(|source| crate::error::io_error(&directory, source))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| crate::error::io_error(&directory, source))?;
    children.sort_by_key(|entry| entry.file_name());
    for child in children {
        let child_relative = relative.join(child.file_name());
        let metadata = fs::symlink_metadata(child.path()).map_err(|source| crate::error::io_error(&child.path(), source))?;
        let mode = file_mode(&metadata);
        let path = child_relative.to_string_lossy().into_owned();
        if metadata.is_dir() {
            collect(root, &child_relative, entries)?;
            entries.push(BundleEntry {
                path: path.clone(),
                kind: BundleEntryKind::Directory,
                mode,
                data: Vec::new(),
            });
        } else if metadata.file_type().is_symlink() {
            let target = fs::read_link(child.path()).map_err(|source| crate::error::io_error(&child.path(), source))?;
            let target = target.to_string_lossy().into_owned();
            validate_relative(Path::new(&target))?;
            entries.push(BundleEntry {
                path,
                kind: BundleEntryKind::Symlink,
                mode,
                data: target.into_bytes(),
            });
        } else if metadata.is_file() {
            let data = fs::read(child.path()).map_err(|source| crate::error::io_error(&child.path(), source))?;
            entries.push(BundleEntry {
                path,
                kind: BundleEntryKind::File,
                mode,
                data,
            });
        } else {
            return Err(Error::MalformedBundle("bundle contains unsupported filesystem entry".into()));
        }
    }
    Ok(())
}

fn validate_relative(path: &Path) -> Result<(), Error> {
    if path.as_os_str().is_empty() || path.is_absolute() || path.components().any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))) {
        return Err(Error::InvalidBundlePath);
    }
    Ok(())
}

fn take<'a>(input: &'a [u8], cursor: &mut usize, length: usize) -> Result<&'a [u8], Error> {
    let end = cursor.checked_add(length).ok_or(Error::MalformedBundle("bundle offset overflow".into()))?;
    let bytes = input.get(*cursor..end).ok_or(Error::MalformedBundle("bundle is truncated".into()))?;
    *cursor = end;
    Ok(bytes)
}

fn read_u16(input: &[u8], cursor: &mut usize) -> Result<u16, Error> {
    Ok(u16::from_be_bytes(take(input, cursor, 2)?.try_into().unwrap()))
}

fn read_u32(input: &[u8], cursor: &mut usize) -> Result<u32, Error> {
    Ok(u32::from_be_bytes(take(input, cursor, 4)?.try_into().unwrap()))
}

fn read_u64(input: &[u8], cursor: &mut usize) -> Result<u64, Error> {
    Ok(u64::from_be_bytes(take(input, cursor, 8)?.try_into().unwrap()))
}

#[cfg(unix)]
fn file_mode(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode()
}

#[cfg(not(unix))]
fn file_mode(_metadata: &fs::Metadata) -> u32 {
    0o700
}

fn set_mode(path: &Path, mode: u32) -> Result<(), Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|source| crate::error::io_error(path, source))?;
    }
    Ok(())
}
