# Project Makefile - task runner. `make help` lists every target.
# The day cycle targets (day-start, day-report, smoke-test) implement
# docs/branching.md; the quality targets implement ADR-0007/0008.

DAY_TAG := daily-$(shell date +%F)

.DEFAULT_GOAL := help
.PHONY: help lint format typecheck test check security build \
        day-start day-report smoke-test mutate review merge-pr pr claim \
        claim-pr update-pr wait-ci night review-bundle sync-daily clean

help: ## List available targets with their description.
	@grep --extended-regexp '^[a-z-]+:.*?## ' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  %-14s %s\n", $$1, $$2}'

lint: ## Lint every file (clippy -D warnings, ADR-0004/0008).
	cargo clippy --all-targets -- -D warnings

format: ## Format every file in place (rustfmt).
	cargo fmt --all

format-check: ## Check formatting without modifying (rustfmt --check).
	cargo fmt --all -- --check

typecheck: ## Alias for clippy (Rust's strict checker, ADR-0006).
	cargo clippy --all-targets -- -D warnings

test: ## Run the full test suite (cargo nextest, ADR-0007).
	cargo nextest run --all-targets

check: format-check lint test ## Full quality gate: run before reporting work.

security: ## Secret scan of the whole history (gitleaks, ADR-0008).
	gitleaks git --no-banner --redact .

audit: ## Dependency audit (cargo-audit).
	cargo audit

deny: ## License and dependency policy (cargo-deny).
	cargo deny check

build: ## Build the workspace (ADR-0001).
	cargo build --workspace

day-start: ## Tag the morning restore point on `daily` (docs/branching.md).
	git tag "$(DAY_TAG)"

day-report: ## Print the end-of-day report since the morning tag.
	@git log --oneline "$(DAY_TAG)..daily"

smoke-test: ## Verify the workspace compiles and tests pass.
	cargo nextest run --all-targets

mutate: ## Run mutation testing (ADR-0012; EOD on daily).
	cargo install cargo-mutate --locked && cargo mutate

review: ## Run the local reviewer session on pull request PR=N (ADR-0010).
	@echo "Review driver not yet ported to Rust; see docs/agents/developer.md"

merge-pr: ## Merge pull request PR=N into daily after review and CI (ADR-0009).
	@echo "Merge driver not yet ported to Rust; see docs/agents/developer.md"

pr: ## Open the pull request from the current branch: TITLE=... [BODY_FILE=...]
	@echo "PR driver not yet ported to Rust; see docs/agents/developer.md"

claim: ## Claim an issue before working it: ISSUE=N (ADR-0017).
	@echo "Claim driver not yet ported to Rust; see docs/agents/developer.md"

claim-pr: ## Claim a pull request before resuming it: PR=N (ADR-0017).
	@echo "Claim driver not yet ported to Rust; see docs/agents/developer.md"

update-pr: ## Replace a pull request body from a file: PR=N BODY_FILE=... (ADR-0019).
	@echo "Update-PR driver not yet ported to Rust; see docs/agents/developer.md"

sync-daily: ## Rebase daily onto origin/main, push with lease (ADR-0020).
	@echo "Sync driver not yet ported to Rust; see docs/agents/developer.md"

wait-ci: ## Actively wait for CI on the pull request head: PR=N (ADR-0018).
	@echo "Wait-CI driver not yet ported to Rust; see docs/agents/developer.md"

night: ## Run the unattended loop (ADR-0017).
	@echo "Night driver not yet ported to Rust; see docs/agents/developer.md"

review-bundle: ## Print the reviewer bundle against origin/daily (ADR-0010).
	@echo "Review bundle not yet ported to Rust; see docs/agents/developer.md"

clean: ## Remove caches and build outputs.
	rm -rf target .nextest .review .night
