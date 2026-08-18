# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2] - 2026-08-18

### Added

- Windows binaries. Releases now carry `hl7test-<version>-x86_64-pc-windows-msvc.zip`
  alongside the macOS and Linux archives.

### Changed

- Every release binary except the cross-compiled Intel macOS one is started on
  its own runner before being packaged.

## [0.1.1] - 2026-08-18

### Fixed

- The Linux binaries are now static musl builds. The 0.1.0 ones were linked
  against the build runner's glibc 2.39 and would not start on Debian 12,
  Ubuntu 22.04, RHEL 8 or 9, or Amazon Linux, which report
  `version 'GLIBC_2.39' not found`. The new binaries have no libc dependency
  and run on any Linux, including Alpine.
- Release checksum files record only the archive name, so `shasum -a 256 -c`
  works in the directory a user downloads into.
- The Intel macOS binary is cross-compiled on an Apple silicon runner, which no
  longer leaves a release waiting on a scarce Intel runner.
- The release job reports why creating a release failed instead of silently
  trying to upload to a release that does not exist.

### Added

- A demo GIF and screenshots in the README, generated from real output by
  `docs/tools/render_media.py`.

## [0.1.0] - 2026-08-17

First release.

### Added

- `hl7test` command that decodes an HL7 v2 message into named fields and
  reports what a receiving system would reject.
- Parser for segments, fields, repetitions, components and subcomponents, with
  support for custom delimiters, escape sequences, CR/LF/CRLF line endings,
  MLLP framing bytes, HL7 batch wrappers and several messages per file.
- Dictionary of segment fields, HL7 code tables and abstract message structures
  covering ADT, ORU, ORM/OML, ACK, SIU, MDM, VXU, DFT, BAR, RDE and QRY.
- Validation across five groups: structure, required fields, data types, code
  tables and cross-field consistency.
- Plain-language decoding of names, dates, coded values, identifiers,
  addresses and patient locations.
- Interactive viewer (`--tui`) with segment, field and validation panels.
- JSON reports (`--json`), single-value queries (`--field`), quiet mode
  (`--quiet`), segment filters (`--segment`) and `--strict` exit codes.
- Homebrew formula and prebuilt binaries for macOS and Linux.

[Unreleased]: https://github.com/sudhi001/hl7probe/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/sudhi001/hl7probe/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/sudhi001/hl7probe/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/sudhi001/hl7probe/releases/tag/v0.1.0
