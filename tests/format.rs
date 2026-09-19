use std::fs;

use tempfile::tempdir;
use vsl::{
    bundle::{pack, unpack},
    digest::sha256,
    format::{chunk, header},
};

#[test]
fn vsl_chunk_round_trips_payload() {
    let payload = b"hello from VSL";
    let digest = sha256(payload);
    let encoded = chunk::encode(payload, digest);
    let decoded = chunk::decode(&encoded).unwrap();
    assert_eq!(decoded, payload);
}

#[test]
fn vsl_rejects_invalid_digest() {
    let payload = b"hello from VSL";
    let mut encoded = chunk::encode(payload, sha256(payload));
    encoded[header::HEADER_LEN + 1] ^= 0xFF;
    let err = chunk::decode(&encoded).unwrap_err();
    assert!(matches!(err, vsl::error::Error::PayloadDigest));
}

#[test]
fn vsl_bundle_round_trips_app_tree() {
    let dir = tempdir().unwrap();
    let app = dir.path().join("Demo.app");
    let bin_dir = app.join("Contents/MacOS");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::write(bin_dir.join("Demo"), b"#!/bin/sh\necho demo\n").unwrap();
    let mode = 0o755u32;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(bin_dir.join("Demo")).unwrap().permissions();
        perms.set_mode(mode);
        fs::set_permissions(bin_dir.join("Demo"), perms).unwrap();
    }

    let payload = pack(&app).unwrap();
    let out = dir.path().join("restored");
    let entrypoint = unpack(&payload, &out).unwrap();
    let restored = fs::read(entrypoint).unwrap();
    assert_eq!(restored, b"#!/bin/sh\necho demo\n");
    assert!(out.join("Contents/MacOS/Demo").exists());
}
