"""Tests of the review-bundle data elision (human directive, 2026-11-17).

The reviewer's context is bounded: data files (benchmark models, deposited
results, images) and oversized diff sections are elided from the bundle
with an explicit marker — never silently. Code files are kept in full.
"""

from scripts.review import (
    ELISION_MARKER,
    MAX_DIFF_LINES_PER_FILE,
    elide_data_files,
    section_file_path,
    split_diff_sections,
)

RS_DIFF = """diff --git a/crates/x/src/lib.rs b/crates/x/src/lib.rs
index 1111111..2222222 100644
--- a/crates/x/src/lib.rs
+++ b/crates/x/src/lib.rs
@@ -1,1 +1,2 @@
 pub fn f() {}
+pub fn g() {}
"""

PNML_PATH = "crates/lisage-bench/models/SwimmingPool-1.pnml"
PNML_DIFF = (
    f"diff --git a/{PNML_PATH} b/{PNML_PATH}\n"
    "new file mode 100644\n"
    "index 0000000..3333333\n"
    "--- /dev/null\n"
    f"+++ b/{PNML_PATH}\n"
    "@@ -0,0 +1,3 @@\n"
    '+<?xml version="1.0"?>\n'
    '+<pnml><net id="SwimmingPool-PT-01"/></pnml>\n'
    "+<page/>\n"
)


def make_oversized_diff(path: str = "crates/x/src/big.rs") -> str:
    """A code file whose diff section exceeds the per-file line cap."""
    body = "".join(f"+line {i}\n" for i in range(MAX_DIFF_LINES_PER_FILE + 10))
    return (
        f"diff --git a/{path} b/{path}\nindex 1111111..2222222 100644\n"
        f"--- a/{path}\n+++ b/{path}\n@@ -1,1 +1,1 @@\n-old\n{body}"
    )


def test_data_extension_is_elided_with_marker() -> None:
    """A .pnml model file is replaced by an explicit marker, the code file kept."""
    diff = RS_DIFF + PNML_DIFF
    result = elide_data_files(diff_text=diff)
    assert ELISION_MARKER in result
    assert "SwimmingPool-1.pnml" in result
    assert "<pnml>" not in result
    assert "pub fn g() {}" in result
    assert "## ELIDED FILES" in result


def test_small_diff_is_unchanged() -> None:
    """A diff with only small code files passes through untouched."""
    assert elide_data_files(diff_text=RS_DIFF) == RS_DIFF


def test_oversized_code_file_is_elided() -> None:
    """Even a code file over the line cap is elided (bounded context)."""
    result = elide_data_files(diff_text=make_oversized_diff())
    assert ELISION_MARKER in result
    assert "big.rs" in result
    assert "line 399" not in result


def test_split_and_path_helpers() -> None:
    """The diff splitter and path extractor handle additions and deletions."""
    sections = split_diff_sections(diff_text=RS_DIFF + PNML_DIFF)
    assert len(sections) == 2
    assert section_file_path(section=sections[0]) == "crates/x/src/lib.rs"
    assert section_file_path(section=sections[1]) == PNML_PATH
    deletion = "diff --git a/old.rs b/old.rs\n--- a/old.rs\n+++ /dev/null\n@@ -1,1 +0,0 @@\n-x\n"
    assert section_file_path(section=deletion) == "old.rs"
