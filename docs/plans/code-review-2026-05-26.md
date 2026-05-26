# Code Review & Remediation Plan — LineEndings

**Date:** 2026-05-26
**Reviewer:** Claude Code (fresh-eyes pass)
**Scope:** Full codebase — `src/`, `tests/`, `Cargo.toml`
**Version reviewed:** 1.1.4 (commit `7952ed7`)
**Supersedes:** `code-review-2026-02-28.md` (most of its findings are now fixed — see "Status of prior review")

---

## Summary

The codebase is in good shape. Module boundaries are clean, the
parallel-analyse / sequential-report structure is sound, atomic replacement via
`NamedTempFile::persist` is correct, and `#![forbid(unsafe_code)]` is enforced.
The earlier review's high-priority issues (string-match binary detection,
`BomType::None`, extensionless backup path, dead debug code) have been
remediated.

This pass focuses only on **substantial** issues. The headline finding is that
the two destructive code paths (`rewrite_file_with_line_ending` and
`remove_bom_from_file`) silently discard file metadata and can interact badly
with pre-existing `.bak` files — both are data-integrity problems for a tool
whose whole job is mutating user files in place.

Priority table is at the bottom.

---

## Issues

### 1. HIGH — File permissions and ownership are silently lost on every rewrite

**Files:** `src/processing.rs:156-203` (`rewrite_file_with_line_ending`),
`src/processing.rs:329-361` (`remove_bom_from_file`)

Both functions create a `NamedTempFile::new_in(parent)` and `persist()` it over
the original. On Unix `mkstemp` creates the temp file with mode `0600` and the
running user's ownership. `persist` renames it into place, so **the replaced
file ends up with `0600` and whatever uid/gid the process runs as**, discarding
the original file's mode and group.

Concrete consequences:

- An executable script (`#!/bin/sh`, mode `0755`) loses its `+x` bit and stops
  being runnable after a line-ending fix.
- A world-readable config (`0644`) becomes `0600`, breaking other users/services.
- Run under `sudo` on a file owned by another user → ownership flips to root.

There is no `set_permissions` call anywhere in the codebase (confirmed by grep),
so this affects 100% of rewrites and BOM removals. The backup (`fs::copy`)
preserves content but does **not** preserve mode either, so even restoring from
`.bak` won't bring the bit back.

**Fix:** Capture `std::fs::metadata(input_path)?.permissions()` before writing
and apply it to the temp file (via the handle, before `persist`, or to the path
after). Behind `#[cfg(unix)]`, also consider preserving the mode bits explicitly
with `PermissionsExt`. Ownership generally cannot be restored without elevated
privileges — document that the tool should not be run as a different user than
the file owner, or `chown` back where possible.

**Tests:** Add integration tests (Unix-gated) asserting that a `0755` file
retains `0755` after `-l`/`-w` conversion and after `--remove-bom`.

---

### 2. HIGH — Pre-existing `.bak` files cause stale backups and unrelated data loss

**Files:** `src/processing.rs:125-133` (`create_backup_if_needed`),
`src/processing.rs:368-403` (`trash_backup_files`), `src/main.rs:165-168`

Two design decisions combine badly:

1. `create_backup_if_needed` only copies **if the `.bak` does not already
   exist**. The intent (idempotency across runs) is reasonable, but if a `.bak`
   already exists from any source, the function trusts it blindly.
2. After a successful run, `trash_backup_files` computes the backup path for
   **every** analysed file and moves it to the trash if it exists.

Failure scenarios:

- **Unrelated user file destroyed.** A user keeps a hand-written
  `notes.txt.bak` next to `notes.txt`. They run `line_endings -l notes.txt`.
  Step 1 sees the `.bak` exists and skips creating a real backup. Step 2 moves
  the user's `notes.txt.bak` to the trash. Their file is gone and was never a
  backup of anything this tool wrote.
- **Stale backup gives false safety.** If a `.bak` lingers from a previous
  aborted run (e.g. trashing failed, or `--no-trash` was used last time), the
  current run rewrites the file but the `.bak` still reflects the *older*
  content — so the backup no longer protects the change just made.

**Fix options (pick one, document the contract):**

- Generate a unique, namespaced backup name unlikely to collide with user files
  (e.g. `<file>.lineendings.bak` or a temp-dir backup keyed by run), and only
  trash backups this run actually created. Track created-backup paths in the
  `RewriteResult`/`BomRemovalResult` rather than re-deriving them in
  `trash_backup_files`.
- Or: always overwrite the backup at the start of a run (so it reflects
  pre-mutation state), and only trash backups whose creation this run recorded.

The key invariant: **only ever trash a `.bak` that this run created**, and
**never let a stale `.bak` stand in for a fresh one**.

**Tests:** (a) pre-existing unrelated `.bak` is not trashed and not treated as a
backup; (b) backup content always equals pre-run content even when a `.bak`
already existed.

---

### 3. HIGH — Rewrite fails on valid non-UTF-8 text files (and is inconsistent with analysis)

**File:** `src/processing.rs:176-194`

Analysis counts line endings at the **byte** level (`count_line_endings`), so it
correctly handles any encoding. But `rewrite_file_with_line_ending` reconstructs
the file via `reader.lines()`, which yields `io::Result<String>` and returns
`ErrorKind::InvalidData` on the first non-UTF-8 byte.

A Latin-1 / Windows-1252 file (e.g. containing `0xE9` for `é`) has no null bytes
and stays under the 30% non-printable threshold, so `is_binary_file` classifies
it as **text**. Analysis succeeds and reports its line endings. The user then
runs `-l`/`-w`, and the rewrite **errors out** on that file — after the backup
has been created, and the early-bail on errors means trashing is skipped, so the
user is left with a `.bak` and an unconverted original.

So the tool will analyse a file it then refuses to fix, with no upfront warning.

**Fix:** Rewrite at the byte level instead of via UTF-8 `lines()`. Scan the byte
stream, normalise `\r\n` / `\n` (and decide on lone `\r`, see issue 6) to the
target ending, and stream bytes to the temp file. This also removes the
"whole line loaded into a `String`" memory cost for files with very long lines,
and makes the rewrite encoding-agnostic, matching the analysis stage.

**Tests:** A file with invalid UTF-8 bytes plus CRLF endings converts to LF with
the non-UTF-8 bytes preserved.

---

### 4. MEDIUM — Overlapping globs process the same file twice in parallel (corruption risk)

**Files:** `src/utils.rs:11-74`, `src/processing.rs:38-41`, `src/main.rs:112-115`

`get_paths_matching_glob` de-duplicates only *within* a single pattern, not
across patterns. Invoking `line_endings -l "*.txt" test_linux.txt` (or any two
overlapping patterns) yields the same path twice in `expanded_paths`.

The rewrite stage runs over these results with `par_iter()`. Two Rayon threads
can call `rewrite_file_with_line_ending` on the **same path concurrently** —
two temp files, two `persist` races, and `create_backup_if_needed` doing a
check-then-copy on the same `.bak`. The atomic `persist` prevents a torn file,
but the interleaving is undefined and wasteful, and the TOCTOU on the backup is
real.

The previous review flagged this as Low; combined with parallel execution it is
better treated as a correctness issue.

**Fix:** De-duplicate `expanded_paths` (canonicalised) before analysis/rewrite.
A `BTreeSet`/`IndexSet` over canonical paths preserves determinism and removes
the hazard entirely.

**Tests:** Two overlapping patterns matching one file → the file is processed
exactly once; exactly one backup created.

---

### 5. MEDIUM — Each file is opened 3–5 times per run

**Files:** `src/analysis.rs:22-91` (analysis), `src/processing.rs:156-220`

`analyze_file` opens the file separately for `is_binary_file`, `detect_bom`, and
`count_line_endings_in_file` — up to three `open()` syscalls per file. The
rewrite path then opens it again for `check_trailing_newline` (with a `seek`)
and once more for the read pass.

For a tool whose value proposition is bulk parallel processing, this is 3–5×
the syscall traffic needed, and each extra open widens the TOCTOU window between
"analysed" and "rewritten". A single read of the leading
`max(BINARY_CHECK_SIZE, …)` bytes can satisfy binary detection, BOM detection,
and feed the line-ending count in one pass; the trailing-newline check can be
folded into the rewrite read instead of a separate seek.

**Fix:** Read the head buffer once in `analyze_file` and derive binary/BOM from
it, streaming the remainder for the line count. In the rewrite path, track the
last byte written instead of pre-seeking for the trailing newline. This is a
performance/robustness refactor, not a behavioural change — keep the existing
tests green.

---

### 6. MEDIUM — UTF-16/UTF-32 BOM removal is unreachable; CR-only endings silently skipped

**Files:** `src/analysis.rs:24-47`, `src/analysis.rs:107-139`, `src/help.rs`

Two scope gaps the user is never told about (both carried over from the prior
review, still open):

- **UTF-16/UTF-32 BOMs can never be removed.** Those encodings contain `\x00`
  bytes, so `is_binary_file` classifies the file as binary and `analyze_file`
  returns before BOM detection. The `BomType::Utf16*/Utf32*` removal sizes in
  `process_file_for_bom_removal` are dead in the normal flow. `--remove-bom`
  silently does nothing for exactly the files most likely to carry those BOMs.
- **CR-only (classic Mac) line endings are invisible.** `count_line_endings`
  counts only `LF` and `CRLF`; a lone `\r` increments nothing. Such a file
  reports "None" (identical to an empty file) and is silently skipped even when
  the user asked for a conversion.

Neither needs full support, but the behaviour must be **honest**. Decide and
document: either detect-and-warn ("skipped: UTF-16 file, BOM removal not
supported" / "skipped: CR-only line endings"), or state the limitation in
`help.rs` and the README. Reporting a CR-only file as "None" is misleading.

**Fix:** Add an explicit "unsupported/skipped" classification surfaced in
output, and update help text. If UTF-16/32 support is desired later, make binary
detection BOM-aware (check for a multibyte BOM before the null-byte test and
exempt those files).

---

### 7. LOW — Rewrite/BOM removal follow and clobber symlinks

**File:** `src/processing.rs:156-203`, `src/processing.rs:329-361`

`persist(input_path)` replaces the path with a regular file. If `input_path` is
a symlink, the link is replaced by a real file and the link target is left
untouched — usually not what the user expects when "fixing" a symlinked config.
Worth a deliberate decision: resolve and write through to the target, or skip
symlinks with a notice. At minimum, document the behaviour.

---

### 8. LOW — No dry-run / preview mode for destructive operations

**Files:** `src/config.rs`, `src/main.rs`, `src/help.rs`

The tool mutates files in place with no way to preview what `-l`/`-w`/`-m` would
do. A `--dry-run` flag that runs analysis and prints the would-rewrite /
would-remove-BOM list without touching disk is low-risk to add (the rewrite
decision logic in `process_file_for_rewrite` is already separated from the write)
and materially safer for first-time users. Suggested, not required.

---

## Status of prior review (`code-review-2026-02-28.md`)

| Prior finding | Status |
|---|---|
| Binary detection via `error.contains("Binary file detected")` | **Fixed** — `is_binary: bool` field added |
| `BomType::None` double-null | **Fixed** — replaced by `bom_checked` + `Option<BomType>` |
| Extensionless backup path `Makefile..bak` | **Fixed** — explicit no-extension branch + unit tests |
| Commented-out debug code in `main.rs` | **Fixed** — gone |
| `rewrite_files` aborts on first error, hides output | **Fixed** — errors now collected, all results printed |
| Misleading `test_bom_strings_are_static` | **Fixed** — replaced by clearer tests |
| `BomType::Utf16/32` dead code | **Open** — see issue 6 |
| TOCTOU in `create_backup_if_needed` | **Open / widened** — see issues 2 and 4 |
| CR-only line endings silently pass through | **Open** — see issue 6 |

---

## Priority summary

| Priority | Issue | File(s) |
|----------|-------|---------|
| High | File permissions/ownership lost on every rewrite & BOM removal | `processing.rs:156-203, 329-361` |
| High | Pre-existing `.bak` → stale backup + unrelated file trashed | `processing.rs:125-133, 368-403` |
| High | Rewrite fails on valid non-UTF-8 text (inconsistent with analysis) | `processing.rs:176-194` |
| Medium | Overlapping globs process same file twice in parallel | `utils.rs:11-74`, `main.rs:112-115` |
| Medium | File opened 3–5× per run (syscalls + TOCTOU) | `analysis.rs:22-91`, `processing.rs` |
| Medium | UTF-16/32 BOM removal unreachable; CR-only silently skipped | `analysis.rs`, `help.rs` |
| Low | Symlinks followed and clobbered | `processing.rs` |
| Low | No `--dry-run` mode | `config.rs`, `main.rs` |

---

## Suggested sequencing

1. **Issues 1–3 first** — they are data-integrity bugs in the destructive paths
   and each is independently testable. Write the failing test, then fix.
2. **Issue 4** — trivial de-dup, removes a parallel hazard.
3. **Issue 6** — honesty about scope; cheap and prevents silent no-ops.
4. **Issue 5** — performance refactor; do after correctness is locked so the
   existing suite guards the behaviour.
5. **Issues 7–8** — decide policy, document, optionally implement.

All changes must keep the commit gate green:
`cargo build --all-targets`, `cargo clippy --all-targets --all-features -- -D clippy::all -D clippy::pedantic -F unsafe_code`, `cargo fmt`, `cargo nextest run`.
