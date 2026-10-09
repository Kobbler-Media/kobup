.PHONY: sqlx-check all quick web fmt fmt-api fmt-web lint lint-api lint-web check check-api check-web build build-api build-web test test-api sweep install dev docker-push sqlx-prepare

all: fmt lint check build test

# Quick build - skips tests
quick: fmt lint check build

# Web-only checks
web: fmt-web lint-web check-web build-web

fmt: fmt-api fmt-web
lint: lint-api lint-web
check: check-api check-web
build: build-api build-web

fmt-api:
	cargo fmt

lint-api:
	SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings

check-api:
	SQLX_OFFLINE=true cargo check

build-api:
	SQLX_OFFLINE=true RUSTFLAGS="-D warnings" cargo build

fmt-web:
	@if [ -d apps/kobup-web/node_modules ]; then npm --prefix apps/kobup-web run format; else echo "web: skipping format (no node_modules)"; fi

lint-web:
	@if [ -d apps/kobup-web/node_modules ]; then npm --prefix apps/kobup-web run lint; else echo "web: skipping lint (no node_modules)"; fi

check-web:
	@if [ -d apps/kobup-web/node_modules ]; then npm --prefix apps/kobup-web run check; else echo "web: skipping check (no node_modules)"; fi

build-web:
	@if [ -d apps/kobup-web/node_modules ]; then npm --prefix apps/kobup-web run build; else echo "web: skipping build (no node_modules)"; fi

test: test-api

test-api:
	SQLX_OFFLINE=true cargo test

# Optional disk-space housekeeping. Needs `cargo install cargo-sweep`.
sweep:
	@command -v cargo-sweep >/dev/null 2>&1 && cargo sweep --maxsize 10GB . || \
		echo "sweep: cargo-sweep not installed, skipping (cargo install cargo-sweep)"

install:
	npm --prefix apps/kobup-web ci

dev:
	@echo "Start in separate terminals:"
	@echo "  cargo run -p kobup-service"
	@echo "  cd apps/kobup-web && npm run dev"

# Publish release images. Maintainers only — needs push rights on REGISTRY.
# Contributors never need this target.
REGISTRY ?= ghcr.io/kobbler-media

docker-push:
	docker build --no-cache --target prod -t $(REGISTRY)/kobup:latest -f docker/api.Dockerfile .
	docker build --no-cache --target prod -t $(REGISTRY)/kobup-web:latest apps/kobup-web
	docker push $(REGISTRY)/kobup:latest
	docker push $(REGISTRY)/kobup-web:latest
	@echo ""
	@echo "=== Pushed digests ==="
	@docker inspect $(REGISTRY)/kobup:latest --format '  api: {{index .RepoDigests 0}}' 2>/dev/null || true
	@docker inspect $(REGISTRY)/kobup-web:latest --format '  web: {{index .RepoDigests 0}}' 2>/dev/null || true
	@echo ""
	docker image prune -f --filter "dangling=true"

SQLX_TMP_DB := /tmp/kobup-sqlx-prepare
SQLX_DB_URL := sqlite:$(SQLX_TMP_DB)/kobup.db?mode=rwc

define sqlx_preflight
	@sqlx --version >/dev/null 2>&1 || { echo "sqlx-cli not installed. cargo install sqlx-cli --no-default-features --features postgres,sqlite,rustls"; exit 1; }
	@DATABASE_URL="sqlite:/tmp/kobup-sqlx-driver-probe.db?mode=rwc" sqlx database create >/dev/null 2>&1 || { \
		echo "sqlx-cli lacks the sqlite driver. Reinstall additively (keeps postgres for other kob projects):"; \
		echo "  cargo install sqlx-cli --no-default-features --features postgres,sqlite,rustls --force"; \
		exit 1; }
	@rm -f /tmp/kobup-sqlx-driver-probe.db
	@rm -rf $(SQLX_TMP_DB) && mkdir -p $(SQLX_TMP_DB)
	@DATABASE_URL="$(SQLX_DB_URL)" cargo sqlx database create
	@DATABASE_URL="$(SQLX_DB_URL)" cargo sqlx migrate run --source crates/kobup-service/migrations
endef

sqlx-prepare:
	$(sqlx_preflight)
	@DATABASE_URL="$(SQLX_DB_URL)" cargo sqlx prepare --workspace
	@rm -rf $(SQLX_TMP_DB)

# sqlx-cli never clears its comparison cache, so if the target dir is shared with
# another sqlx project its queries make --check report ours missing.
sqlx-check:
	@rm -rf "$${CARGO_TARGET_DIR:-target}/sqlx-prepare-check"
	$(sqlx_preflight)
	@DATABASE_URL="$(SQLX_DB_URL)" cargo sqlx prepare --workspace --check
	@rm -rf $(SQLX_TMP_DB)
