# Project Makefile.
#
# Targets are documented inline with `## ` comments — the `help`
# target greps for them, so adding a new target with a `## ` comment
# automatically lists it in `make help`.

.DEFAULT_GOAL := help

.PHONY: help init fmt fmt-check build test lint clean

help: ## Show this help.
	@awk 'BEGIN {FS = ":.*?## "; printf "Usage: make <target>\n\nTargets:\n"} \
		/^[a-zA-Z_-]+:.*?## / {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}' \
		$(MAKEFILE_LIST)

# ─── Bootstrap ────────────────────────────────────────────────────
# `make init` brings a fresh checkout into a working state: git
# repo, beads issue tracker, and a link between the two. Each step
# is idempotent — running `make init` again is safe.

init: ## Initialize repo: git init, ensure bd installed, bd init, link bd to git origin.
	@set -e; \
	if [ ! -d .git ]; then \
		echo "[init] git init"; \
		git init; \
	else \
		echo "[init] git: already initialized"; \
	fi; \
	if ! command -v bd >/dev/null 2>&1; then \
		echo "[init] bd (beads) not found on PATH."; \
		echo "       Install bd and ensure it is on PATH, then re-run 'make init'."; \
		echo "       See your beads distribution for install instructions."; \
		exit 1; \
	else \
		echo "[init] bd: $$(command -v bd)"; \
	fi; \
	if [ ! -d .beads ]; then \
		echo "[init] bd init"; \
		bd init; \
	else \
		echo "[init] bd: already initialized (.beads/ present)"; \
	fi; \
	if git remote get-url origin >/dev/null 2>&1; then \
		url=$$(git remote get-url origin); \
		echo "[init] bd config set sync.remote git+$$url"; \
		bd config set sync.remote "git+$$url"; \
	else \
		echo "[init] git has no 'origin' remote; skipping bd sync.remote"; \
	fi

# ─── Quality gates ────────────────────────────────────────────────
# Stubs — replace the bodies with this project's real toolchain.
# AGENTS.md references these target names; keep them named the same
# so the documented workflow stays accurate.

fmt: ## Format all source files in place.
	@echo "fmt: not yet configured — edit Makefile" && exit 1

fmt-check: ## Check formatting without modifying files.
	@echo "fmt-check: not yet configured — edit Makefile" && exit 1

build: ## Build the project.
	@echo "build: not yet configured — edit Makefile" && exit 1

test: ## Run the test suite.
	@echo "test: not yet configured — edit Makefile" && exit 1

lint: ## Run static analysis / linters.
	@echo "lint: not yet configured — edit Makefile" && exit 1

clean: ## Remove build artifacts.
	@echo "clean: not yet configured — edit Makefile" && exit 1
