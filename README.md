# Truco Engine

The authoritative rules engine for Baixada's two-player Brazilian Truco.

This repository deliberately contains three implementations at one contract
boundary:

- the production Rust engine;
- a thin Python binding over the Rust API;
- an independent TypeScript implementation used to detect behavioral drift.

Rules, schemas, and executable fixtures live in
[`baixada-cards/truco-spec`](https://github.com/baixada-cards/truco-spec).
This repository records an exact contract revision in
[`spec.lock.json`](spec.lock.json), verifies every manifested file, and runs
the same fixture corpus through Rust and TypeScript.

## Layout

| Path | Responsibility |
| --- | --- |
| `crates/truco-engine` | Rules, state transitions, public/player views, fixture runner, and exploration resolution |
| `crates/truco-engine-py` | PyO3 boundary exposing the serde-friendly Rust API |
| `python/truco_engine` | Small Python development wrapper and native-loader helper |
| `typescript` | Independent TypeScript engine and conformance tests |
| `spec.lock.json` | Immutable `truco-spec` version, revision, and manifest digest |
| `scripts/sync_spec.py` | Safe materialization and verification of the pinned contract |

HTTP sessions, gameplay bots, CFR solving, deployment, and product UI are
intentionally outside this repository.

## Development

Prerequisites:

- stable Rust with Clippy and rustfmt;
- Python 3.12+ and `uv`;
- Node.js 24 and pnpm 10;
- [Socket Firewall Free](https://docs.socket.dev/docs/socket-firewall-free).

Install dependencies and materialize the pinned specification:

```sh
make sync
make sync-spec
```

To use an already verified local `truco-spec` checkout without fetching it:

```sh
TRUCO_SPEC_SOURCE=/path/to/truco-spec make sync-spec
```

Run the complete repository gate:

```sh
make check
```

The contract checkout is generated under `.cache/truco-spec` and is never
committed.

## Direct commands

```sh
cargo test -p truco-engine --all-targets --locked
UV_CACHE_DIR=.uv-cache uv run --no-sync pytest tests/python
pnpm --dir typescript test
```

Build and stage the Python extension for local imports:

```sh
uv run python -m truco_engine._native_loader --build
```

## Consuming the Rust engine

During the multi-repository release-candidate phase, downstream Rust projects
pin this repository by signed tag or full commit:

```toml
truco-engine = {
  git = "https://github.com/baixada-cards/truco-engine",
  rev = "<full-commit-sha>"
}
```

Moving branch dependencies are not supported. Package-registry publication
will be considered only when a real external consumer benefits from it.

## Versioning

Repository releases follow Semantic Versioning. A release records the exact
`truco-spec` revision it conforms to. A contract change is not complete until
the relevant fixture suite passes in every maintained implementation.

## License

MIT. See [`LICENSE`](LICENSE).
