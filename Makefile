# coding-crew — developer targets
#
# The gate is one command: make check
#
# talita is out of scope for now; there are no targets for it.

CARGO ?= $(shell command -v cargo 2>/dev/null || echo $(HOME)/.cargo/bin/cargo)
CARGO_TARGET_DIR ?= target
RAUL_PKG := coding-crew-raul
HUGO_PKG := coding-crew-hugo
TALITA_PKG := coding-crew-talita
SKILL_SCRIPTS ?= $(HOME)/.config/opencode/skills/orchestration-flow/scripts

.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help
	@echo "coding-crew — common targets"
	@echo ""
	@grep -E '^[a-zA-Z0-9_.-]+:.*##' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*## "}; {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

.PHONY: build build-release build-raul build-hugo build-talita
build: ## Build the workspace
	$(CARGO) build

build-release: ## Build release binaries
	$(CARGO) build --release

build-raul: ## Build raul only
	$(CARGO) build -p $(RAUL_PKG)

build-hugo: ## Build hugo only
	$(CARGO) build -p $(HUGO_PKG)

build-talita: ## Build talita only
	$(CARGO) build -p $(TALITA_PKG)

.PHONY: run raul
run raul: ## Run raul (TUI) on the mock data
	$(CARGO) run -p $(RAUL_PKG) -- mock-data/coding-crew/work

.PHONY: hugo
hugo: ## Run hugo (TUI) on its mock runtime
	$(CARGO) run -p $(HUGO_PKG)

.PHONY: test test-scripts columns
test: ## Run the test suite
	$(CARGO) test --workspace

test-scripts: ## Run the shell script tests
	bash scripts/test-pty-sweep.sh

columns: ## Regenerate the terminal-column table from its rules and check it
	python3 scripts/generate-terminal-columns.py --check

.PHONY: test-flow
test-flow: ## Run the flow skill's suites when the skill is installed
	@set -e; \
	if [ -d "$(SKILL_SCRIPTS)" ]; then \
		bash "$(SKILL_SCRIPTS)/test-herdr-wake.sh"; \
		bash "$(SKILL_SCRIPTS)/test-review-snapshot.sh"; \
	else \
		echo "flow skill is not installed at $(SKILL_SCRIPTS) - skipping flow suites"; \
	fi

.PHONY: sweep
sweep: build-raul ## Sweep the plain render at every width from 40 to 120: the Overview and the Tasks tab, in both icon sets
	python3 scripts/pty-sweep.py --raul $(CARGO_TARGET_DIR)/debug/raul --work mock-data/coding-crew/work --icons nerd
	python3 scripts/pty-sweep.py --raul $(CARGO_TARGET_DIR)/debug/raul --work mock-data/coding-crew/work --icons unicode
	python3 scripts/pty-sweep.py --raul $(CARGO_TARGET_DIR)/debug/raul --work mock-data/coding-crew/work --tab tasks --icons nerd
	python3 scripts/pty-sweep.py --raul $(CARGO_TARGET_DIR)/debug/raul --work mock-data/coding-crew/work --tab tasks --icons unicode

.PHONY: fmt fmt-check
fmt: ## Format the code
	$(CARGO) fmt --all

fmt-check: ## Check formatting
	$(CARGO) fmt --all -- --check

.PHONY: lint
lint: ## Run clippy with warnings as errors
	$(CARGO) clippy --all-targets -- -D warnings

.PHONY: check ci
check: fmt-check lint test test-scripts columns test-flow sweep ## The gate: fmt, clippy, tests, script tests, column table, flow suites, width sweep
ci: check ## Alias for check

.PHONY: doc
doc: ## Build the API docs
	$(CARGO) doc --no-deps --workspace

.PHONY: clean
clean: ## Remove build artifacts
	$(CARGO) clean
