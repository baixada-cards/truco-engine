.PHONY: check sync sync-spec rust python typescript

sync:
	sfw cargo fetch --locked
	sfw uv sync --frozen --group dev
	sfw pnpm install --frozen-lockfile

sync-spec:
	python3 scripts/sync_spec.py

rust: sync-spec
	cargo fmt --all --check
	cargo clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
	cargo test --workspace --all-targets --locked --offline

python:
	UV_CACHE_DIR=.uv-cache uv run --no-sync ruff format --check python tests/python scripts
	UV_CACHE_DIR=.uv-cache uv run --no-sync ruff check python tests/python scripts
	UV_CACHE_DIR=.uv-cache uv run --no-sync pytest tests/python

typescript: sync-spec
	pnpm --dir typescript typecheck
	pnpm --dir typescript test

check: sync-spec
	python3 scripts/check_action_pins.py
	python3 scripts/sync_spec.py --check
	$(MAKE) rust
	$(MAKE) python
	$(MAKE) typescript
