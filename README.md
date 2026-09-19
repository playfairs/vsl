# VSL

VSL is the binary payload format layer. It defines the serialized `envelope` for payload bytes, including header metadata, versioning, payload length, and the digest used to validate content.

This repository is intentionally independent of image storage and CLI concerns.

```text
VSL
├── normal payload
└── VSLBND01
      └── macOS .app bundle
```

## Relationship to veSL

- VSL is the payload format.
- veSL is the image carrier used to carry a payload inside a PNG.
- Vessel is the consumer / orchestrator that may eventually decide how to run an extracted payload.

The important distinction is:

- `veSL` is not the payload format.
- `VSL` is not the image carrier.
- `Vessel` is not the format itself.

## VSLBND01

VSLBND01 is the specialized bundle payload type for macOS application bundles. It serializes a directory tree in a way that preserves relative bundle structure, executable paths, and relevant metadata so the reconstructed bundle can retain the relationships expected by the host system.

This repository currently exposes the bundle pack/unpack behavior directly, without depending on PNG functionality or CLI orchestration.

## Development

This project uses Nox as the primary workflow interface.

```bash
nox validate
nox build
nox test
nox check
nox task fmt-check
nox task clippy
```