.PHONY: release audit audit-parity audit-crates audit-runaway-procs audit-disk-budget quality-lint quality-tools quality-coverage quality-coverage-html quality-metrics quality-dead-code quality-unused-deps quality-duplicates quality-report rebuild-with-quality

QUALITY_DIR := target/quality
QUALITY_COVERAGE_DIR := $(QUALITY_DIR)/coverage
QUALITY_METRICS_DIR := $(QUALITY_DIR)/metrics
QUALITY_CLIPPY_FILE := $(QUALITY_DIR)/clippy/messages.jsonl
QUALITY_MACHETE_FILE := $(QUALITY_DIR)/machete/report.txt
QUALITY_JSCPD_FILE := $(QUALITY_DIR)/jscpd/jscpd-report.json
QUALITY_REPORT_DIR := $(QUALITY_DIR)/reports

release:
	cargo build --release -p maenifold-cli
	cargo build --release -p maenifold-pipeline-driver

audit: audit-parity audit-crates audit-runaway-procs audit-disk-budget

audit-parity:
	@./scripts/audit-parity.sh

audit-crates:
	@./scripts/audit-crates.sh

audit-runaway-procs:
	@bash scripts/audit-runaway-procs.sh

audit-disk-budget:
	@bash scripts/audit-disk-budget.sh

quality-lint:
	cargo clippy --workspace --lib --bins -- -A clippy::all -W clippy::correctness -W clippy::suspicious -W clippy::perf -W dead_code -W unused -D warnings -A clippy::too-many-lines -A clippy::cognitive-complexity -A clippy::too-many-arguments

quality-tools:
	@echo "Install Rust quality tools:"
	@echo "  rustup component add llvm-tools-preview"
	@echo "  cargo install cargo-llvm-cov cargo-machete rust-code-analysis-cli"
	@echo "  npm install -g jscpd"

quality-coverage:
	mkdir -p $(QUALITY_COVERAGE_DIR)
	MAENIFOLD_FULL_SUITE=1 cargo llvm-cov --workspace --lcov --summary-only --output-path $(QUALITY_COVERAGE_DIR)/lcov.info --ignore-filename-regex '(^|/)(tests|target|vendor)/'

quality-coverage-html:
	mkdir -p $(QUALITY_COVERAGE_DIR)
	MAENIFOLD_FULL_SUITE=1 cargo llvm-cov --workspace --html --output-dir $(QUALITY_COVERAGE_DIR)/html --ignore-filename-regex '(^|/)(tests|target|vendor)/'

quality-metrics:
	mkdir -p $(QUALITY_METRICS_DIR)
	rust-code-analysis-cli -m -p crates -O json -o $(QUALITY_METRICS_DIR)

quality-dead-code:
	mkdir -p $(dir $(QUALITY_CLIPPY_FILE))
	cargo clippy --workspace --lib --bins --message-format=json -- -A clippy::all -W dead_code -W unused -A clippy::too-many-lines -A clippy::cognitive-complexity -A clippy::too-many-arguments > $(QUALITY_CLIPPY_FILE)

quality-unused-deps:
	mkdir -p $(dir $(QUALITY_MACHETE_FILE))
	@cargo machete --with-metadata > $(QUALITY_MACHETE_FILE); \
	status=$$?; \
	if [ $$status -gt 1 ]; then \
		exit $$status; \
	fi

quality-duplicates:
	mkdir -p $(QUALITY_DIR)/jscpd
	npx --yes jscpd --config .jscpd.json

quality-report: quality-coverage quality-metrics quality-dead-code quality-unused-deps quality-duplicates
	mkdir -p $(QUALITY_REPORT_DIR)
	python3 scripts/quality_report.py --repo-root . --lcov $(QUALITY_COVERAGE_DIR)/lcov.info --metrics-dir $(QUALITY_METRICS_DIR) --clippy $(QUALITY_CLIPPY_FILE) --machete $(QUALITY_MACHETE_FILE) --jscpd $(QUALITY_JSCPD_FILE) --output-dir $(QUALITY_REPORT_DIR)

# Canonical "rebuild + quality" combo. ./rebuild.sh redeploys the daemon (fast,
# ~2-3 min). Then make quality-report runs the full quality pipeline including
# cargo llvm-cov --workspace under instrumentation (slow, +10-15 min). Quality
# failures are warn-only: a non-zero quality exit will NOT fail this target,
# but the warning + exit code are printed to stdout so it stays visible.
# CRAP hotspot table lands at $(QUALITY_REPORT_DIR)/summary.md.
rebuild-with-quality:
	./rebuild.sh
	@echo ""
	@echo "═══ Quality report (warn-only) ═══"
	@$(MAKE) quality-report; \
		status=$$?; \
		if [ $$status -ne 0 ]; then \
			echo ""; \
			echo "  ⚠ make quality-report exited $$status (non-fatal; daemon is up)"; \
		fi
	@echo ""
	@if [ -f $(QUALITY_REPORT_DIR)/summary.md ]; then \
		echo "═══ Quality summary ═══"; \
		cat $(QUALITY_REPORT_DIR)/summary.md; \
	else \
		echo "  ⚠ no summary at $(QUALITY_REPORT_DIR)/summary.md"; \
	fi
