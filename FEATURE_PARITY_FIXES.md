# Feature Parity Fixes: WPF → Tauri

## Executive Summary

This document tracks all fixes applied to achieve feature parity between the WPF reference implementation and the Tauri 2.x implementation.

## Completed Fixes

### ✅ Fix 1: Video Naming - Extract Real Filenames from URLs

**Issue**: WPF extracts real filenames from URL query strings (e.g., `XcGaajaEfYcA` from `.../XcGaajaEfYcA.mp4`). Tauri was using `Video_1`, `Video_2` placeholders.

**Solution**: Modified `format_url_name` in `parser.rs` to extract filename from URL path, remove extension, and return meaningful name or fallback to `Video_N`.

**Files Modified**: `src-tauri/src/parser.rs`

---

### ✅ Fix 2: Resume Progress Accounting

**Issue**: When resuming a download, progress calculation was incorrect. The `total` field should represent final file size.