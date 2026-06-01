# Project Makefile.
#
# Targets are documented inline with `## ` comments. The `help` target
# prints any target with a `## ` comment.

.DEFAULT_GOAL := help

BACKLOG_NPM_PACKAGE := https://github.com/lesserevil/backlog.md/archive/HEAD.tar.gz
BACKLOG_PREFIX := .tools/backlog
BACKLOG_CLI := $(BACKLOG_PREFIX)/bin/backlog

export PATH := $(abspath $(BACKLOG_PREFIX)/bin):$(PATH)

.PHONY: help init ensure-backlog fmt fmt-check build test lint clean

help: ## Show this help.
	@awk 'BEGIN {FS = ":.*?## "; printf "Usage: make <target>\n\nTargets:\n"} \
		/^[a-zA-Z_-]+:.*?## / {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}' \
		$(MAKEFILE_LIST)

init: ensure-backlog ## Initialize git and Backlog.md metadata for this checkout.
	@set -e; \
	if [ ! -d .git ]; then \
		echo "[init] git init"; \
		git init; \
	else \
		echo "[init] git: already initialized"; \
	fi; \
	if [ ! -f backlog/config.yml ]; then \
		echo "[init] backlog init"; \
		$(BACKLOG_CLI) init Rogers --defaults --backlog-dir backlog --config-location folder; \
	else \
		echo "[init] backlog: already initialized"; \
	fi

ensure-backlog: ## Install the Backlog.md CLI from lesserevil/backlog.md into .tools/backlog.
	@set -e; \
	if [ ! -x "$(BACKLOG_CLI)" ]; then \
		mkdir -p "$(BACKLOG_PREFIX)"; \
		npm install --global --prefix "$(BACKLOG_PREFIX)" --ignore-scripts --no-audit --no-fund "$(BACKLOG_NPM_PACKAGE)"; \
	fi; \
	echo "Backlog.md CLI available: $$($(BACKLOG_CLI) --version)"

fmt: ## Format all source files in place.
	@cargo fmt --all

fmt-check: ## Check formatting without modifying files.
	@cargo fmt --all --check

build: ## Build the project.
	@cargo build --all

test: ## Run the test suite.
	@cargo test --all

lint: ## Run static analysis / linters.
	@cargo clippy --all -- -D warnings

clean: ## Remove build artifacts.
	@cargo clean
