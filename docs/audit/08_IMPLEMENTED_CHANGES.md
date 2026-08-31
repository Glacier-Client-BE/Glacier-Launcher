# 08 — Implemented Changes Log

**No source code changes were made during this audit pass.**

## Why
The audit's safety constraint permits only changes that are "safe,
deterministic, and clearly preserve behavior" (genuinely unused imports,
confirmed-dead/unreachable branches, confirmed-orphaned assets). This pass
specifically checked for each of these categories:

- **Unused imports (Android Kotlin):** every `import` line in
  `android/app/src/main/java/xyz/glacierclient/launcher/GlacierApp.kt`,
  `MainActivity.kt`, `service/ClientInjectionService.kt`, and
  `service/JavaEditionBridge.kt` was cross-checked against usage in each
  file's body. All imports are referenced — none found unused.
- **Orphaned assets:** every file under
  `android/app/src/main/assets/www/` was checked against references from
  `index.html` and the `js/*.js` files. All CSS, font, icon, and image files
  are referenced from at least one loaded file — none found orphaned.
- **Dead/unreachable branches:** no confidently-dead branch was identified
  in the files read during this pass (`MainActivity.kt`,
  `ClientInjectionService.kt`, `JavaEditionBridge.kt`, `GlacierApp.kt`); the
  much larger C# codebase (17,100+ lines across `Pages/`+`Services/`+`Models/`)
  was surveyed by size/structure rather than line-by-line in the time
  available for this pass, so no C# dead-code claim is made with the
  confidence this audit's own safety bar requires.
- **`ClientInjectionService.kt` doc-comment staleness** (README flags this
  explicitly as outdated re: "needs root") was considered but treated as a
  **content** change (in-app Settings → Clients copy is user-facing prose,
  not indisputably-dead code) and left undone here — tracked instead as
  `03_MISSING_FEATURES.md` item 4 / `04_CODE_AUDIT.md`'s last row, for the
  calling session or a follow-up pass to apply deliberately alongside the
  actual injection-architecture work it describes.

## Files created
9 new files under `docs/audit/`:
- `docs/audit/01_ARCHITECTURE.md`
- `docs/audit/02_FEATURE_PARITY.md`
- `docs/audit/03_MISSING_FEATURES.md`
- `docs/audit/04_CODE_AUDIT.md`
- `docs/audit/05_REFACTOR_PLAN.md`
- `docs/audit/06_PERFORMANCE_REPORT.md`
- `docs/audit/07_UI_PARITY.md`
- `docs/audit/08_IMPLEMENTED_CHANGES.md` (this file)
- `docs/audit/TODO.md`

## Files modified
None.
</content>
