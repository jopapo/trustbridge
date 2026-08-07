# Roadmap

## v0.1 - Bootstrap (current)

- Rust CLI scaffold
- macOS keychain source scan
- sync planning engine
- rancher-desktop target stub
- foundational docs and ADRs

## v0.2 - Rancher Desktop Real Sync

- Implement certificate copy into Rancher Desktop VM
- Execute truststore update inside VM
- Add failure rollback and stronger error reporting
- Add `doctor` command for dependency checks

## v0.3 - Policy & Filtering

- Allowlist/denylist by fingerprint
- Subject/issuer filters
- expiration guardrails
- config file support

## v0.4 - Broader Targets

- Docker Desktop target
- Colima target
- better runtime autodetection

## v0.5 - Cross-platform Sources

- Windows certificate store source
- Linux host truststore source

## v1.0 - Stable OSS Foundation

- tested plugin contracts
- migration/stability guarantees
- release automation and package distribution
