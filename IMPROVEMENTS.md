# Potential Improvements

## 1. Binary file detection ✅ COMPLETED
Currently the tool will try to process binary files if they match the glob pattern. Should skip or warn about binary files to avoid corrupting them.

**Implementation approach:**
- Read first 8KB of file and check for null bytes or high percentage of non-printable characters
- Skip binary files and report them separately
- Add optional `--force` flag to override binary detection

**Status:** Implemented in commit 112689d. Binary files are now automatically detected and skipped, with a summary count displayed.

## 2. Empty file handling ✅ COMPLETED
The `check_trailing_newline()` function seeks to -1 on empty files, which could fail. Should handle explicitly.

**Implementation approach:**
- Check file size first, return early for empty files
- Add test cases for empty files

**Status:** Already implemented correctly. File size is checked before seeking (line 202-204 in processing.rs).

## 3. Dry-run mode
Add `--dry-run` flag to preview what would be changed without modifying files.

**Implementation approach:**
- Add `dry_run` boolean to `ConfigSettings`
- Skip actual file writes but show what would be done
- Useful for testing before making bulk changes

## 4. Progress indication
For large batches, show progress (e.g., "Processing 500/1000 files...").

**Implementation approach:**
- Add progress counter using indicatif crate or simple counter
- Show periodic updates during parallel processing
- Option to disable with `--quiet` flag

## 5. Summary statistics ✅ COMPLETED
At the end, show comprehensive statistics about the run.

**Implementation approach:**
- Track: total files, files changed, files skipped, total line endings converted
- Display in clear summary format
- Already partially implemented, could be expanded

**Status:** Implemented in commit 35cd920. Now displays:
- Total files processed and binary files skipped
- Files with mixed line endings count
- Total LF and CRLF line endings across all files
- Analysis time and total execution time

## 6. Backup cleanup ✅ COMPLETED
Add option to clean up `.bak` files after successful operations.

**Implementation approach:**
- Add `--remove-backups` flag
- Delete all `.bak` files created during session
- Add confirmation prompt for safety

**Status:** Implemented in commit d5ba37c. Added `--delete-backups` (-d) flag that:
- Deletes .bak backup files for processed files
- Can be combined with other operations (rewrite, BOM removal)
- Reports count of deleted backups
- No confirmation prompt needed as it only deletes backups for explicitly specified files

## 7. File size limits
Add optional max file size limit to avoid accidentally processing huge files.

**Implementation approach:**
- Add `--max-size <BYTES>` option
- Check file size before processing
- Skip files over limit with warning

## 8. Error handling improvements
Currently exits on first error during rewrite. Could collect errors and continue processing other files.

**Implementation approach:**
- Already partially implemented with parallel error collection
- Continue processing all files even if some fail
- Show all errors at the end

## 9. Configuration file support
Allow `.lineendings.toml` config file for common settings.

**Implementation approach:**
- Add serde and toml crates
- Read config from `.lineendings.toml` or `lineendings.toml`
- Command line args override config file settings

## 10. Exclude patterns
Add `--exclude` to skip certain patterns (e.g., `--exclude "*.min.js"`).

**Implementation approach:**
- Add exclude patterns to `ConfigSettings`
- Filter matched files against exclude patterns
- Support multiple exclude patterns
