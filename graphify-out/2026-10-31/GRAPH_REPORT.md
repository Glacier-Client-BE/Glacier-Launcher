# Graph Report - MinecraftLauncher  (2026-10-30)

## Corpus Check
- 296 files · ~588,847 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 3935 nodes · 9461 edges · 169 communities (154 shown, 6 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 169 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `1a553949`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- InstanceService
- launcher.rs
- launch.rs
- ShaderService
- ImportLauncherView.svelte
- gl-auth/src/lib.rs
- utilities.rs
- gl-nbt/src/lib.rs
- java_loaders.rs
- resume.rs
- BedrockInstanceService
- themes.rs
- GitHubApiCache
- sync_packs.rs
- gl-injection/src/lib.rs
- versions.rs
- ImportCandidate
- App.svelte
- worlds.rs
- tests.rs
- levilamina.rs
- gl-settings/src/lib.rs
- mod_analyzer.rs
- UtilitiesView.svelte
- modpack.rs
- CLAUDE.md
- server_ping.rs
- gl-clients/src/hash.rs
- lunar_badlion.rs
- releases.rs
- frontend/package.json
- clients.rs
- mmc.rs
- src/notifications.rs
- tauri.conf.json
- BedrockOptionsService
- wu.rs
- export/curseforge.rs
- logs.rs
- PARITY.md (parity ledger)
- skin_library.rs
- src/bedrock.rs
- horizon.rs
- gl-integrations/src/models.rs
- SyncReport
- CurseForgeClient
- LoaderClient
- JavaInstancesView.svelte
- flarial.rs
- bedrock_addons.rs
- commands/notifications.rs
- Release.tsx
- java_packs.rs
- src/tweaks.rs
- DownloadQueue
- ModrinthClient
- ServersView.svelte
- collector
- version_service.rs
- jar_inspect.rs
- queueEntry
- DownloadHandle
- paths.rs
- gl-clients/src/models.rs
- gl-download/src/download.rs
- ClientLibraryView.svelte
- gl-persistence/src/lib.rs
- commands/bedrock.rs
- gdk.rs
- RuntimeDownloadService
- launcher_import.rs
- modpack_transfer.rs
- java_mods.rs
- queue.rs
- src/client_library.rs
- src/skins.rs
- src/java.rs
- gl-win32chrome/src/lib.rs
- commands/java.rs
- system.rs
- gl-java/src/lib.rs
- src/glacier_client.rs
- DiscordRpcService
- mod_provenance.rs
- SkinViewer.svelte
- compilerOptions
- AppState
- integrations.rs
- io_err
- SyncTarget
- latite.rs
- throwaway_host.rs
- HorizonRelease
- commands/levilamina_mods.rs
- commands/skins.rs
- commands/tweaks.rs
- commands/client_library.rs
- SyncRegistry
- compilerOptions
- glacier-launcher
- effective.rs
- BedrockScreenshotsView.svelte
- commands/glacier_client.rs
- InstallState
- Path
- atlauncher.rs
- settings.ts
- interop.js
- BedrockError
- oderso.rs
- OnboardingModal.svelte
- .parse
- DownloadError
- modrinth_app.rs
- java_screenshots.rs
- compilerOptions
- GlacierClientVersion
- check
- detach
- package.json
- presets.rs
- setup_tray
- parse_version_db
- default.json
- set_settings
- JavaInstance
- ModpackError
- inject_bedrock_client
- ClientsError
- IntegrationError
- read_instance
- gl-update/src/error.rs
- tsconfig.json
- detect.rs
- interop.d.ts
- ai.rs
- java_process.rs
- DownloadRequest
- commands/mod.rs
- loader_map.rs
- Prompt for the next chat
- list_jar_names
- update.rs
- version.rs
- GitHubRelease
- UpdateChecker
- DownloadClient
- MrVersion
- launchers.rs
- reuse.rs
- .new
- mod_loader.rs
- resolve_modrinth_node
- report.rs
- UnifiedMod
- OptionsDocument
- []
- ModpacksView.svelte
- HorizonLayout
- HorizonBusy

## God Nodes (most connected - your core abstractions)
1. `AppState` - 280 edges
2. `InstanceService` - 41 edges
3. `DownloadQueue` - 36 edges
4. `BedrockInstanceService` - 26 edges
5. `SyncReport` - 26 edges
6. `io_err()` - 25 edges
7. `ThemeDefinition` - 25 edges
8. `ImportCandidate` - 23 edges
9. `ProcessRegistry` - 22 edges
10. `DownloadRequest` - 21 edges

## Surprising Connections (you probably didn't know these)
- `frontend/README.md (Svelte+Vite template doc)` --semantically_similar_to--> `Glacier Launcher (product)`  [INFERRED] [semantically similar]
  frontend/README.md → README.md
- `ensure_server_jar()` --calls--> `hashes_equal()`  [INFERRED]
  src-tauri/src/commands/launchers.rs → crates/gl-clients/src/hash.rs
- `scratch.txt (per-file special-character scratch dump)` --references--> `PARITY.md (parity ledger)`  [AMBIGUOUS]
  scratch.txt → PARITY.md
- `scratch.txt (per-file special-character scratch dump)` --references--> `README.md (Glacier Launcher overview)`  [AMBIGUOUS]
  scratch.txt → README.md
- `glacier-launcher` --depends_on--> `gl-auth`  [EXTRACTED]
  src-tauri/Cargo.toml → crates/gl-auth/Cargo.toml

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **No path to install a current Bedrock build (GdkArchiveService + StoreInstallService + WU cap)** — concept_gdkarchiveservice, concept_storeinstallservice, concept_wu_rs [EXTRACTED 1.00]
- **CI/build verification gates for the cutover commit** — github_workflows_ci, github_workflows_build, concept_cutover_commit [EXTRACTED 1.00]
- **Transcription ledger and its supporting line-range inventories** — parity, docs_inventory_shell, docs_inventory_bedrock_panels, docs_inventory_bedrock_services [EXTRACTED 1.00]

## Communities (169 total, 6 thin omitted)

### Community 0 - "InstanceService"
Cohesion: 0.08
Nodes (41): count_files_with_ext(), count_mods(), count_worlds(), dir_size_bytes(), Path, copy_dir_recursive(), import_many(), import_one() (+33 more)

### Community 1 - "launcher.rs"
Cohesion: 0.26
Nodes (19): build_classpath(), build_game_args(), build_jvm_args(), collect_args(), features_match(), is_jvm_arg_compatible(), LaunchOptions, md5() (+11 more)

### Community 2 - "launch.rs"
Cohesion: 0.11
Nodes (48): available_ram_mb(), cancel_launch(), console_content_title(), console_stop_game(), ConsoleExitPayload, ConsoleLogPayload, ConsoleStatusPayload, emit_java_log_line() (+40 more)

### Community 3 - "ShaderService"
Cohesion: 0.13
Nodes (28): AppliedShader, Into, Path, PathBuf, Result, Self, String, Vec (+20 more)

### Community 4 - "ImportLauncherView.svelte"
Cohesion: 0.14
Nodes (6): goToPreview(), openLauncher(), pickCustomRoot(), runDetection(), runImport(), toast()

### Community 5 - "gl-auth/src/lib.rs"
Cohesion: 0.05
Nodes (68): B, authenticate_with_minecraft(), AuthError, DisplayClaims, exchange_code(), exchange_code_against_live_endpoint(), exchange_code_for_live_token(), exchange_for_xbl_token() (+60 more)

### Community 6 - "utilities.rs"
Cohesion: 0.06
Nodes (80): blank_entry(), decide_update(), decide_update_detects_matching_and_changed_hash(), hash_and_size(), head_matches_recorded_size(), index_path(), kind_for_extension(), library_dir() (+72 more)

### Community 7 - "gl-nbt/src/lib.rs"
Cohesion: 0.06
Nodes (50): apply_summary_to_root(), find_byte(), find_int(), gzip_compress(), gzip_decompress(), gzip_helpers_round_trip(), level_dat_rejects_bad_header(), level_dat_round_trip_with_header() (+42 more)

### Community 8 - "java_loaders.rs"
Cohesion: 0.30
Nodes (17): get_instance_loader(), install_loader(), InstanceLoaderDto, java_root_dir(), list_loader_versions(), LoaderInstallProgress, parse_loader(), AppHandle (+9 more)

### Community 9 - "resume.rs"
Cohesion: 0.32
Nodes (12): clear_pending_marker(), clear_pending_marker_is_idempotent(), marker_path(), marker_pointing_at_missing_file_is_ignored(), marker_pointing_at_real_file_is_returned_and_consumed(), no_marker_returns_none(), Option, Path (+4 more)

### Community 10 - "BedrockInstanceService"
Cohesion: 0.06
Nodes (56): BackupDiff, BedrockBackupService, BedrockRoot, HashMap, Into, Option, Path, PathBuf (+48 more)

### Community 11 - "themes.rs"
Cohesion: 0.07
Nodes (48): css_vars_cover_every_variable_the_original_emits(), duplicated_gets_a_fresh_id(), exports_dir(), fmt3(), light_text_uses_subtler_item_overlays(), lighten(), new_id(), overlay_strength_scales_the_overlay_alphas() (+40 more)

### Community 12 - "GitHubApiCache"
Cohesion: 0.22
Nodes (13): CachedResponse, Entry, expires_after_ttl(), GitHubApiCache, hit_after_store(), miss_when_empty(), HashMap, Instant (+5 more)

### Community 13 - "sync_packs.rs"
Cohesion: 0.35
Nodes (19): apply_sync_pack(), clear_sync_conflict_override(), create_sync_pack(), delete_sync_pack(), detach_sync_pack(), get_sync_report(), instance_dir(), instance_dirs() (+11 more)

### Community 14 - "gl-injection/src/lib.rs"
Cohesion: 0.08
Nodes (37): c_void, activate_application(), enable_debug_privilege(), find_process_by_name(), find_processes_by_name(), grant_uwp_access(), inject_dll(), inject_rejects_missing_dll_before_touching_win32() (+29 more)

### Community 15 - "versions.rs"
Cohesion: 0.20
Nodes (29): WuVersion, available_free_space(), delete_latite_version(), delete_vanilla_bedrock_version(), download_latite_version(), download_vanilla_bedrock_version(), ensure_disk_space(), installed_bedrock_version() (+21 more)

### Community 16 - "ImportCandidate"
Cohesion: 0.19
Nodes (14): atlauncher_adapter_parses_neoforge_instance(), curseforge_app_adapter_parses_forge_instance(), DetectedLauncher, ImportCandidate, mmc_adapter_parses_a_fabric_instance(), modrinth_app_adapter_parses_quilt_profile(), pipeline_imports_and_copies_files(), ModLoaderKind (+6 more)

### Community 17 - "App.svelte"
Cohesion: 0.13
Nodes (3): i(), lang, ViewMode

### Community 18 - "worlds.rs"
Cohesion: 0.08
Nodes (66): copy_dir_recursive(), delete_pack(), dir_size(), import_a_loose_shaderpack_folder(), import_list_delete_a_zip_pack(), import_pack(), importing_a_missing_source_is_an_error(), list_packs() (+58 more)

### Community 19 - "tests.rs"
Cohesion: 0.25
Nodes (14): dir_link_apply_with_keep_existing_then_detach_materialize_round_trips_no_data_loss(), dir_link_conflict_requires_explicit_adopt_strategy(), file_copy_apply_and_detach_discard(), instance_dirs(), make_pack(), mods_target_reports_missing_and_present(), options_txt_merge_preserves_key_order_and_untouched_local_keys(), preview_never_touches_the_filesystem() (+6 more)

### Community 20 - "levilamina.rs"
Cohesion: 0.06
Nodes (48): detects_not_found_payload(), falls_back_to_first_zip_when_no_windows_asset(), get_text_cached(), GitHubAsset, GitHubRelease, mod_asset_prefers_zip_over_other_files(), parse_release(), parses_release_and_picks_windows_zip() (+40 more)

### Community 21 - "gl-settings/src/lib.rs"
Cohesion: 0.18
Nodes (21): cache_dir(), default_settings_round_trip_through_persistence(), JavaAccount, launcher_root(), LauncherSettings, legacy_settings_path(), load(), load_from_disk() (+13 more)

### Community 22 - "mod_analyzer.rs"
Cohesion: 0.19
Nodes (26): analyze(), analyze_dir(), analyzes_fabric_mod_json(), analyzes_forge_mods_toml(), built_ins(), capture(), delete_mod(), is_disabled_jar() (+18 more)

### Community 23 - "UtilitiesView.svelte"
Cohesion: 0.12
Nodes (4): optionIsOn(), optionValue(), setOptionValue(), toggleOption()

### Community 24 - "modpack.rs"
Cohesion: 0.09
Nodes (51): errors_when_manifest_missing(), loader_profile_id(), parse_curseforge_manifest(), parse_loader_id(), parse_modrinth_index(), parses_curseforge_manifest_fixture(), parses_modrinth_index_fixture_and_skips_unsupported_client_files(), read_zip_json() (+43 more)

### Community 26 - "server_ping.rs"
Cohesion: 0.08
Nodes (50): extract_between(), LanGame, parse_lan_packet(), parses_well_formed_lan_packet(), Option, Result, String, Vec (+42 more)

### Community 27 - "gl-clients/src/hash.rs"
Cohesion: 0.24
Nodes (6): AsRef, Path, Result, String, sha256_hex_bytes(), sha256_hex_file()

### Community 28 - "lunar_badlion.rs"
Cohesion: 0.21
Nodes (19): detect(), detects_lunar_exe_at_known_path(), DirectLaunchFiles, find_badlion(), find_lunar(), find_lunar_jre(), list_version_dirs(), lists_offline_versions_sorted_descending() (+11 more)

### Community 29 - "releases.rs"
Cohesion: 0.21
Nodes (14): asset_prefers_exe_over_other_files(), fixture(), no_update_when_current_is_latest(), pick_asset(), pick_update(), picks_newest_non_draft_with_newer_version(), recent_releases(), recent_releases_skips_drafts_and_caps_at_max() (+6 more)

### Community 30 - "frontend/package.json"
Cohesion: 0.05
Nodes (40): dependencies, skinview3d, @tauri-apps/api, @tauri-apps/plugin-dialog, @tauri-apps/plugin-log, @tauri-apps/plugin-notification, devDependencies, esbuild (+32 more)

### Community 31 - "clients.rs"
Cohesion: 0.16
Nodes (42): OderSoDllEntry, active_oderso_dll(), check_flarial_dll_up_to_date(), check_levilamina_up_to_date(), check_oderso_up_to_date(), delete_flarial_client(), delete_levilamina_client(), delete_oderso_client() (+34 more)

### Community 32 - "mmc.rs"
Cohesion: 0.38
Nodes (11): MmcComponent, MmcPack, parse_instance_cfg_name(), read_instance(), Option, Path, Result, String (+3 more)

### Community 33 - "src/notifications.rs"
Cohesion: 0.14
Nodes (22): AppNotification, detached(), ids_are_unique_and_guid_n_shaped(), items_come_back_newest_first(), mark_all_read_clears_the_count(), new_id(), note(), notifications_path() (+14 more)

### Community 34 - "tauri.conf.json"
Cohesion: 0.06
Nodes (35): debugApplicationIdSuffix, app, security, trayIcon, windows, enable, scope, build (+27 more)

### Community 35 - "BedrockOptionsService"
Cohesion: 0.13
Nodes (18): append_ext(), BedrockOptionsService, find_meta(), infer_kind(), known_keys(), Meta, prettify(), HashMap (+10 more)

### Community 36 - "wu.rs"
Cohesion: 0.17
Nodes (24): extract_appx(), fetch_updates(), find_text(), generic_version(), get_auth_cookie(), get_download_url(), is_dotted_number(), list_versions() (+16 more)

### Community 37 - "export/curseforge.rs"
Cohesion: 0.09
Nodes (43): add_dir_to_zip(), add_file_to_zip(), export_curseforge(), loader_id(), Path, PathBuf, Result, SimpleFileOptions (+35 more)

### Community 38 - "logs.rs"
Cohesion: 0.14
Nodes (38): chrono_like_day(), civil_from_days(), CrashReportInfo, day_to_iso(), days_from_civil(), export_support_bundle(), get_stats_summary(), latest_java_crash_report() (+30 more)

### Community 39 - "PARITY.md (parity ledger)"
Cohesion: 0.13
Nodes (25): Bedrock-first, then Java (order of work), BedrockContentView merged-panels defect (Worlds/Packs/Backups/Screenshots), BedrockTabs (bottom tab strip, Home.BigFeatures.cs:54-75), Missing com.mojang root resolution (highest-risk finding), Console window rendered main app (console.html missing bug), rust-tauri-cutover branch cutover commit, GdkArchiveService (Bedrock GDK version source), gl-injection crate (Win32 DLL injection port) (+17 more)

### Community 40 - "skin_library.rs"
Cohesion: 0.17
Nodes (25): add_from_file(), add_from_username(), base64_decode(), base64_roundtrip_matches_known_vector(), delete_skin(), list_skins(), MojangProfileByName, resolve_skin_texture() (+17 more)

### Community 41 - "src/bedrock.rs"
Cohesion: 0.20
Nodes (24): activate_via_com(), BedrockPackageInfo, connect_external_server(), escape_data_string(), extract_application_id(), find_game_process(), find_game_process_for(), installed_version() (+16 more)

### Community 42 - "horizon.rs"
Cohesion: 0.08
Nodes (45): attr_value(), blocks_become_lines_and_items_bullets(), build_commit(), build_id(), build_id_is_slugged_and_digest_suffixed(), commit_of_release(), decode_entities(), decode_entity() (+37 more)

### Community 43 - "gl-integrations/src/models.rs"
Cohesion: 0.16
Nodes (17): get_announcement(), Client, Option, Announcement, AppNotification, default_icon(), default_kind(), new_id() (+9 more)

### Community 44 - "SyncReport"
Cohesion: 0.29
Nodes (9): HashMap, Path, PathBuf, Result, Self, String, SyncEngine, Vec (+1 more)

### Community 45 - "CurseForgeClient"
Cohesion: 0.16
Nodes (15): CurseForgeClient, html_decode(), parse_addon(), parse_file(), parses_search_addon_fixture(), Client, Into, Option (+7 more)

### Community 46 - "LoaderClient"
Cohesion: 0.18
Nodes (13): extract_xml_tag_values(), extracts_xml_versions(), is_pre_release(), LoaderClient, Client, Result, Self, String (+5 more)

### Community 47 - "JavaInstancesView.svelte"
Cohesion: 0.12
Nodes (11): cloneJavaInstance(), commitGroup(), commitRenameInstance(), deleteJavaInstance(), importInstanceZip(), importOfficialMinecraft(), newJavaInstance(), onInstancePresetChange() (+3 more)

### Community 48 - "flarial.rs"
Cohesion: 0.26
Nodes (9): fetch_remote_hash(), local_hash(), local_hash_matches_known_file_contents(), parse_remote_hash(), parses_release_hash_from_fixture(), Client, Path, Result (+1 more)

### Community 49 - "bedrock_addons.rs"
Cohesion: 0.25
Nodes (26): BedrockAddonInstallResult, check_bedrock_pack_conflicts(), copy_dir_recursive(), default_dir_name_for_class(), extract_to_temp(), find_pack_conflicts(), install_bedrock_addon(), install_into_class_dir() (+18 more)

### Community 50 - "commands/notifications.rs"
Cohesion: 0.29
Nodes (12): AppNotification, add_notification(), clear_notifications(), list_notifications(), mark_all_notifications_read(), mark_notification_read(), notification_unread_count(), remove_notification() (+4 more)

### Community 51 - "Release.tsx"
Cohesion: 0.05
Nodes (61): react, react-dom, remotion, @remotion/cli, @remotion/google-fonts, @remotion/transitions, @types/react, dependencies (+53 more)

### Community 52 - "java_packs.rs"
Cohesion: 0.30
Nodes (22): delete_java_resourcepack(), delete_java_shaderpack(), get_mod_versions(), import_java_resourcepack(), import_java_shaderpack(), install_curseforge_mod(), install_planned_file(), install_resolved_mods() (+14 more)

### Community 53 - "src/tweaks.rs"
Cohesion: 0.19
Nodes (22): BedrockManifest, BedrockManifestHeader, BedrockManifestModule, BedrockTweaksIndex, build_bedrock_manifest(), builds_manifest_for_single_pack(), flatten_vanilla(), flattens_vanilla_categories_recursively() (+14 more)

### Community 54 - "DownloadQueue"
Cohesion: 0.16
Nodes (9): DownloadQueue, Duration, Mutex, Result, Sender, String, Semaphore, AppHandle (+1 more)

### Community 55 - "ModrinthClient"
Cohesion: 0.20
Nodes (13): html_decode(), ModrinthClient, parse_project(), parse_version(), parses_latest_version_and_prefers_primary_file(), parses_search_project_fixture(), Client, Option (+5 more)

### Community 56 - "ServersView.svelte"
Cohesion: 0.10
Nodes (11): closeServerModal(), deleteServer(), #each(), getServerIconStyle(), isServerIconImage(), openServers(), parseTags(), saveServerModal() (+3 more)

### Community 57 - "collector"
Cohesion: 0.18
Nodes (18): collector(), enough_time_but_not_enough_delta_is_suppressed(), first_update_always_reported(), reports_once_interval_and_delta_both_satisfied(), Duration, FnMut, Instant, Option (+10 more)

### Community 58 - "version_service.rs"
Cohesion: 0.21
Nodes (19): annotate_installed(), cache_entries(), CachedVersion, dirs_home(), load_cache_file(), ManifestLatest, ManifestRoot, ManifestVersionEntry (+11 more)

### Community 59 - "jar_inspect.rs"
Cohesion: 0.17
Nodes (22): analyze(), analyze_flags_duplicate_ids_and_missing_deps(), find_toml_string(), is_built_in(), parse_fabric(), parse_forge(), parses_fabric_mod_json_fixture(), parses_forge_mods_toml_fixture_and_extracts_required_deps() (+14 more)

### Community 60 - "queueEntry"
Cohesion: 0.67
Nodes (3): cancelFor(), progressFor(), queueEntry()

### Community 61 - "DownloadHandle"
Cohesion: 0.15
Nodes (14): AtomicBool, DownloadEntry, DownloadHandle, DownloadStatus, RegistryEntry, Arc, CancellationToken, Option (+6 more)

### Community 62 - "paths.rs"
Cohesion: 0.06
Nodes (69): copy_dir_recursive(), extract_to_temp(), import_content(), import_packs(), import_world(), ImportOutcome, imports_mcaddon_bundle_routes_by_module_type(), imports_mcpack_by_resource_module_type() (+61 more)

### Community 63 - "gl-clients/src/models.rs"
Cohesion: 0.15
Nodes (11): ClientDownloadState, ClientInfo, ClientLibraryPack, ClientType, default_kind(), download_state_lifecycle(), GlacierLauncherMeta, GlacierManifest (+3 more)

### Community 64 - "gl-download/src/download.rs"
Cohesion: 0.18
Nodes (21): backoff_for(), commit(), content_range_total(), download_file(), download_once(), download_with_retry(), DownloadOutcome, has_real_hash() (+13 more)

### Community 65 - "ClientLibraryView.svelte"
Cohesion: 0.11
Nodes (13): app, deleteScreenshot(), installClientPack(), isClientPackExpanded(), loadClientLibrary(), packKey(), refreshInstalled(), toggleClientPack() (+5 more)

### Community 66 - "gl-persistence/src/lib.rs"
Cohesion: 0.18
Nodes (18): append_extension(), Envelope, load(), no_leftover_temp_file_after_write(), PersistError, quarantine_corrupt(), round_trips_through_atomic_write_and_load(), Error (+10 more)

### Community 67 - "commands/bedrock.rs"
Cohesion: 0.07
Nodes (68): BedrockBackup, BedrockInstance, BedrockPack, BedrockWorld, OptionEntry, OptionKind, PackKind, Option (+60 more)

### Community 68 - "gdk.rs"
Cohesion: 0.18
Nodes (13): a_missing_or_odd_digest_yields_no_hash_rather_than_a_bad_one(), an_error_body_is_an_empty_list_not_a_failure(), list_versions(), maps_release_fields_and_skips_drafts(), parse_asset_name(), parse_releases(), Client, Option (+5 more)

### Community 69 - "RuntimeDownloadService"
Cohesion: 0.21
Nodes (14): AdoptiumBinary, AdoptiumPackage, AdoptiumRelease, extract_zip(), find_first_named(), Client, FnMut, Option (+6 more)

### Community 70 - "launcher_import.rs"
Cohesion: 0.32
Nodes (11): detect_external_launchers(), import_instances(), ImportOutcome, preview_import(), JavaInstance, Option, Result, State (+3 more)

### Community 71 - "modpack_transfer.rs"
Cohesion: 0.17
Nodes (22): add_dir_to_zip(), copy_dir_recursive(), detect_multimc_root(), err(), export_instance(), export_modpack(), extract_overrides(), import_modpack() (+14 more)

### Community 72 - "java_mods.rs"
Cohesion: 0.21
Nodes (38): check_java_mod_updates(), client_mod_prefix(), delete_java_mod(), diff_java_instances(), download_forge_installer(), download_neoforge_installer(), ensure_client_mod_installed(), install_fabric_loader() (+30 more)

### Community 73 - "queue.rs"
Cohesion: 0.29
Nodes (10): clear_finished_also_drops_queued_entries(), new_entry_starts_queued_with_zero_progress(), now_unix_ms(), pausing_a_download_that_finished_before_the_pause_request_is_a_no_op(), queue_caps_concurrency_at_configured_limit(), remove_and_clear_notify_subscribers(), resume_is_a_no_op_for_an_entry_that_isnt_paused(), Vec (+2 more)

### Community 74 - "src/client_library.rs"
Cohesion: 0.19
Nodes (16): encode_path(), index_url(), media_url(), parse_index(), parses_and_groups_by_version_and_name(), PathEntry, raw_url(), Result (+8 more)

### Community 75 - "src/skins.rs"
Cohesion: 0.21
Nodes (15): CapeInfo, ApiErrorBody, extract_api_message(), get_capes(), hide_cape(), ProfileResponse, RawCape, reset_skin() (+7 more)

### Community 76 - "src/java.rs"
Cohesion: 0.23
Nodes (16): AuthValues, component_matches_major(), find_javaw_under(), JavaLaunchRequest, launch(), LaunchedJava, resolve_java_runtime(), ResolvedAuth (+8 more)

### Community 77 - "gl-win32chrome/src/lib.rs"
Cohesion: 0.17
Nodes (12): apply_chrome(), hit_test(), hittest_from_lparam(), hittest_subclass_proc(), HitTestResult, install_resize_hittest(), Option, WinResult (+4 more)

### Community 78 - "commands/java.rs"
Cohesion: 0.17
Nodes (37): JvmPreset, create_java_instance(), create_java_instance_from_template(), delete_java_instance(), duplicate_java_instance(), export_java_instance(), get_effective_launch_config(), get_instance_settings() (+29 more)

### Community 79 - "system.rs"
Cohesion: 0.15
Nodes (34): auto_export_settings_if_due(), close_launcher_window(), ConfigProfile, copy_file(), delete_all_data(), delete_config_profile(), export_settings(), exports_dir() (+26 more)

### Community 80 - "gl-java/src/lib.rs"
Cohesion: 0.22
Nodes (18): read_version_profile(), build_classpath_excludes_natives_and_dedupes(), build_game_args_fullscreen_takes_priority_over_custom_resolution(), build_game_args_substitutes_and_appends_resolution_and_server(), build_jvm_args_clamps_ram_to_documented_bounds(), build_jvm_args_honors_quoted_values_with_spaces(), build_jvm_args_substitutes_and_filters_classpath_placeholder(), inherited_profile_without_jar_uses_vanilla_root_jar() (+10 more)

### Community 81 - "src/glacier_client.rs"
Cohesion: 0.17
Nodes (14): fetch_download_counts(), fetch_manifest(), glacier_dir(), installed_size_bytes(), parse_manifest(), parses_manifest_camel_case_fields(), require_installed(), Client (+6 more)

### Community 82 - "DiscordRpcService"
Cohesion: 0.23
Nodes (7): DiscordRpcService, now_unix(), Default, Drop, Option, Self, DiscordIpcClient

### Community 83 - "mod_provenance.rs"
Cohesion: 0.30
Nodes (18): corrupt_store_falls_back_to_empty_rather_than_panicking(), load(), missing_store_yields_empty_default(), ModProvenance, ModSource, ProvenanceStore, record(), record_load_remove_roundtrip() (+10 more)

### Community 84 - "SkinViewer.svelte"
Cohesion: 0.08
Nodes (16): bodyRenderUrl, capeLabel, capeMode, cleanUuid, displayName, failed, fallbackRenderUrl, model (+8 more)

### Community 85 - "compilerOptions"
Cohesion: 0.12
Nodes (16): compilerOptions, allowImportingTsExtensions, erasableSyntaxOnly, lib, module, moduleDetection, noEmit, noFallthroughCasesInSwitch (+8 more)

### Community 86 - "AppState"
Cohesion: 0.18
Nodes (22): JavaInstanceService, JavaVersionService, cancel_download(), clear_finished_downloads(), DownloadEntryDto, list_downloads(), pause_download(), remove_download() (+14 more)

### Community 87 - "integrations.rs"
Cohesion: 0.14
Nodes (33): NewsPost, cached_news(), decode(), decode_entity(), decode_posts_decodes_exactly_once(), fetch_client_changelog(), get_news(), html_decode() (+25 more)

### Community 88 - "io_err"
Cohesion: 0.22
Nodes (20): io_err(), Error, Path, PathBuf, String, SyncError, apply(), copy_dir_recursive() (+12 more)

### Community 89 - "SyncTarget"
Cohesion: 0.21
Nodes (10): AdoptStrategy, MergeFormat, Option, String, Vec, SyncTarget, VersionOverride, resolve() (+2 more)

### Community 90 - "latite.rs"
Cohesion: 0.28
Nodes (14): dll_file_name(), falls_back_to_name_and_date_for_non_standard_tags(), format_display_name(), format_published_at(), LatiteVersion, list_versions(), parse_latite_tag(), parse_versions() (+6 more)

### Community 91 - "throwaway_host.rs"
Cohesion: 0.25
Nodes (12): HostGuard, injecting_into_a_dead_process_fails_cleanly(), injects_a_real_dll_into_a_throwaway_process(), missing_dll_is_reported_before_touching_the_process(), process_lookup_finds_a_spawned_host_and_loses_it_after_exit(), Child, Drop, PathBuf (+4 more)

### Community 92 - "HorizonRelease"
Cohesion: 0.19
Nodes (16): HorizonRelease, missing_windows_zip_is_an_error(), not_found_payload_is_reported(), parse_release(), parse_releases(), pick_sums_asset(), pick_zip_asset(), picks_exact_windows_zip_not_other_platforms() (+8 more)

### Community 93 - "commands/levilamina_mods.rs"
Cohesion: 0.29
Nodes (15): delete_levilamina_mod(), download_levilamina_mod(), extract_zip(), fetch_levilamina_mods(), levilamina_mod_installed(), mod_install_dir(), mods_root(), open_levilamina_mods_folder() (+7 more)

### Community 94 - "commands/skins.rs"
Cohesion: 0.33
Nodes (15): add_skin_from_file(), add_skin_from_username(), apply_skin(), delete_saved_skin(), get_capes(), hide_cape(), list_saved_skins(), reset_skin() (+7 more)

### Community 95 - "commands/tweaks.rs"
Cohesion: 0.33
Nodes (15): BedrockTweaksIndexDto, build_bedrock_tweaks_pack(), build_vanilla_tweaks_pack(), fetch_image_bytes(), list_bedrock_tweaks(), list_vanilla_tweaks(), new_uuid_v4(), HashMap (+7 more)

### Community 96 - "commands/client_library.rs"
Cohesion: 0.31
Nodes (14): ClientLibraryDownload, client_library_site_url(), client_library_versions(), get_client_library_packs(), install_client_library_download(), installed_path(), is_client_library_download_installed(), resource_packs_dir() (+6 more)

### Community 97 - "SyncRegistry"
Cohesion: 0.20
Nodes (9): SyncPack, Mutex, Option, Path, PathBuf, Result, Self, Vec (+1 more)

### Community 98 - "compilerOptions"
Cohesion: 0.13
Nodes (14): compilerOptions, allowArbitraryExtensions, allowJs, checkJs, module, moduleDetection, noEmit, target (+6 more)

### Community 99 - "glacier-launcher"
Cohesion: 0.17
Nodes (16): gl-auth, gl-bedrock, gl-clients, gl-download, gl-injection, gl-integrations, gl-java, gl-launch (+8 more)

### Community 100 - "effective.rs"
Cohesion: 0.27
Nodes (13): all_none_falls_back_to_defaults(), effective(), EffectiveLaunchConfig, explicit_memory_override_wins(), GlobalJavaDefaults, instance_with_settings(), java_path_override(), legacy_ram_fields_still_override_when_no_new_settings() (+5 more)

### Community 101 - "BedrockScreenshotsView.svelte"
Cohesion: 0.11
Nodes (12): formatSize(), closeLightbox(), copyScreenshotToClipboard(), formatSize(), lightboxKeydown(), lightboxNext(), lightboxPrev(), screenshotUrl() (+4 more)

### Community 102 - "commands/glacier_client.rs"
Cohesion: 0.27
Nodes (14): get_glacier_download_counts(), get_glacier_manifest(), glacier_installed_size_bytes(), install_glacier_client(), launch_glacier_client(), AppHandle, HashMap, Option (+6 more)

### Community 103 - "InstallState"
Cohesion: 0.20
Nodes (8): hashes_equal(), BuildRecord, digest_matches(), InstallState, is_sha256_hex(), promote_keeps_previous_and_returns_dropped_build(), rollback_swaps_and_needs_a_previous_build(), build_info()

### Community 104 - "Path"
Cohesion: 0.25
Nodes (15): classpath_separator(), detect_java_major(), find_first_javaw_under(), find_javaw_in_registry(), heuristic_major_from_path(), LibraryEntry, maven_coord_to_path(), render_command_line() (+7 more)

### Community 105 - "atlauncher.rs"
Cohesion: 0.40
Nodes (9): AtInstance, AtLauncherBlock, AtLoaderVersion, read_instance(), Option, Path, String, Vec (+1 more)

### Community 106 - "settings.ts"
Cohesion: 0.13
Nodes (17): canAdvance(), create(), ensureVersionInstalled(), loadVersions(), next(), toast(), BedrockInstance, EffectiveLaunchConfig (+9 more)

### Community 107 - "interop.js"
Cohesion: 0.24
Nodes (8): comboFromEvent(), ensureTabCyclers(), matchesAction(), pageSizeFor(), REBINDABLE_DEFAULTS, refreshCyclerButtons(), tabsOf(), tauriWindow

### Community 108 - "BedrockError"
Cohesion: 0.23
Nodes (10): BedrockError, IoContext, Error, Into, PathBuf, Result, String, T (+2 more)

### Community 109 - "oderso.rs"
Cohesion: 0.25
Nodes (13): DllEntry, list_dlls(), parse_dll_list(), parses_and_sorts_dll_entries_newest_first(), raw_url(), Client, Result, String (+5 more)

### Community 110 - "OnboardingModal.svelte"
Cohesion: 0.14
Nodes (10): LanguageOption, LANGUAGES, t(), Table, currentStepKey, finish(), handleKeydown(), lastStep (+2 more)

### Community 111 - ".parse"
Cohesion: 0.21
Nodes (9): MavenCoordinate, parses_basic_coordinate(), parses_classifier_and_extension(), parses_extension_without_classifier(), Option, Result, Self, String (+1 more)

### Community 112 - "DownloadError"
Cohesion: 0.27
Nodes (7): DownloadError, Box, Error, Into, PathBuf, Self, String

### Community 113 - "modrinth_app.rs"
Cohesion: 0.40
Nodes (9): MrLoaderVersion, MrMetadata, MrProfile, read_profile(), Option, Path, String, Vec (+1 more)

### Community 114 - "java_screenshots.rs"
Cohesion: 0.29
Nodes (14): delete_java_screenshot(), entry(), iso_from_systemtime(), list_java_screenshots(), open_java_screenshots_folder(), Option, Path, PathBuf (+6 more)

### Community 115 - "compilerOptions"
Cohesion: 0.18
Nodes (10): compilerOptions, esModuleInterop, jsx, module, moduleResolution, noEmit, skipLibCheck, strict (+2 more)

### Community 116 - "GlacierClientVersion"
Cohesion: 0.46
Nodes (3): GlacierClientVersion, Path, PathBuf

### Community 117 - "check"
Cohesion: 0.33
Nodes (9): check(), PrelaunchWarning, Into, JavaInstance, Option, Path, String, Vec (+1 more)

### Community 118 - "detach"
Cohesion: 0.26
Nodes (10): apply(), detach(), Path, Result, apply(), detach(), Option, Path (+2 more)

### Community 119 - "package.json"
Cohesion: 0.25
Nodes (7): description, name, private, scripts, build, dev, version

### Community 120 - "presets.rs"
Cohesion: 0.38
Nodes (6): args_for(), find(), JvmPreset, Option, String, Vec

### Community 121 - "setup_tray"
Cohesion: 0.38
Nodes (6): apply_native_window_chrome(), AppHandle, Result, run(), setup_tray(), WebviewWindow

### Community 122 - "parse_version_db"
Cohesion: 0.31
Nodes (9): first_seen_wins_on_duplicate_key(), list_versions(), parse_version_db(), Client, Result, Vec, skips_section_headers_eappx_and_non_x64(), sorts_newest_first() (+1 more)

### Community 123 - "default.json"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 124 - "set_settings"
Cohesion: 0.57
Nodes (6): get_settings(), keep_newer_java_token(), LauncherSettings, Result, String, set_settings()

### Community 125 - "JavaInstance"
Cohesion: 0.22
Nodes (9): InstanceSettings, JavaInstance, JavaVersion, ModLoaderKind, InstanceSettings, ModLoaderKind, Option, String (+1 more)

### Community 126 - "ModpackError"
Cohesion: 0.50
Nodes (4): ModpackError, Error, String, ZipError

### Community 127 - "inject_bedrock_client"
Cohesion: 0.40
Nodes (3): inject_bedrock_client(), Result, String

### Community 128 - "ClientsError"
Cohesion: 0.67
Nodes (3): ClientsError, Error, String

### Community 129 - "IntegrationError"
Cohesion: 0.67
Nodes (3): IntegrationError, Error, String

### Community 130 - "read_instance"
Cohesion: 0.39
Nodes (8): CfLoader, CfManifest, read_instance(), Option, Path, String, Vec, scan()

### Community 133 - "detect.rs"
Cohesion: 0.36
Nodes (7): appdata(), detect_all(), instances_dir(), Option, Path, PathBuf, Vec

### Community 135 - "ai.rs"
Cohesion: 0.33
Nodes (9): Candidate, ContentBody, explain_crash_log(), GenerateContentResponse, Part, Result, State, String (+1 more)

### Community 141 - "java_process.rs"
Cohesion: 0.07
Nodes (51): kill_errors_for_untracked_instance(), kill_process_tree(), mark_exited_removes_from_list_and_drops_buffer(), pid_of_reflects_registration_and_removal(), ProcessEntry, ProcessRegistry, push_log_line_is_noop_for_unknown_instance(), register_push_and_read_roundtrip() (+43 more)

### Community 142 - "DownloadRequest"
Cohesion: 0.36
Nodes (6): DownloadRequest, Into, PathBuf, Self, String, validate_rejects_empty_url()

### Community 145 - "commands/mod.rs"
Cohesion: 0.29
Nodes (8): ensure_direct_child(), ensure_direct_child_accepts_child_and_rejects_outside(), Path, PathBuf, Result, String, safe_file_name(), scratch()

### Community 146 - "loader_map.rs"
Cohesion: 0.62
Nodes (6): from_atlauncher_loader_type(), from_curseforge_loader_name(), from_mmc_component_uid(), from_modrinth_loader(), ModLoaderKind, Option

### Community 149 - "Prompt for the next chat"
Cohesion: 0.29
Nodes (6): CSS housekeeping, Established patterns to reuse, not reinvent, Known open issue (not yet verified live), Prompt for the next chat, Suggested next targets, roughly in priority order, Verification

### Community 150 - "list_jar_names"
Cohesion: 0.60
Nodes (5): list_jar_names(), mod_filenames(), Path, String, Vec

### Community 151 - "update.rs"
Cohesion: 0.35
Nodes (14): check_launcher_update(), download_launcher_update(), install_launcher_update(), launcher_version(), list_client_releases(), list_glacier_client_releases(), list_launcher_releases(), resume_pending_launcher_update() (+6 more)

### Community 152 - "version.rs"
Cohesion: 0.20
Nodes (5): is_newer_version(), normalize_tag(), parse_version(), Option, String

### Community 153 - "GitHubRelease"
Cohesion: 0.31
Nodes (8): GitHubAsset, GitHubRelease, LauncherRelease, LauncherUpdateInfo, GitHubAsset, Option, String, Vec

### Community 154 - "UpdateChecker"
Cohesion: 0.39
Nodes (5): Client, Result, Self, String, UpdateChecker

### Community 155 - "DownloadClient"
Cohesion: 0.31
Nodes (4): DownloadClient, Client, Result, Self

### Community 156 - "MrVersion"
Cohesion: 0.22
Nodes (17): CfAddon, CfSearchResult, CurseForgeFileRef, ModInfo, ModLoader, ModpackFile, ModpackPlan, ModReport (+9 more)

### Community 157 - "launchers.rs"
Cohesion: 0.14
Nodes (51): begin_horizon_op(), build_status(), detect_lunar_badlion(), DirectLaunchFilesDto, dirs_local_app_data(), dirs_user_profile(), emit_horizon_progress(), ensure_java_25() (+43 more)

### Community 158 - "reuse.rs"
Cohesion: 0.07
Nodes (54): AssetJob, collect_libraries(), DownloadJob, extract_native_jar(), InstallProgress, InstallService, library_paths_escaping_libraries_dir_are_skipped(), Arc (+46 more)

### Community 159 - ".new"
Cohesion: 0.50
Nodes (3): Result, Self, String

### Community 160 - "mod_loader.rs"
Cohesion: 0.05
Nodes (75): download_to_file(), get_string(), Client, Path, Result, String, io_err(), JavaError (+67 more)

### Community 162 - "resolve_modrinth_node"
Cohesion: 0.21
Nodes (17): InstallPlan, PlannedFile, resolve_curseforge_plan(), resolve_modrinth_node(), resolve_modrinth_node_inner(), resolve_modrinth_plan(), Box, HashMap (+9 more)

### Community 164 - "report.rs"
Cohesion: 0.21
Nodes (8): preview(), Path, String, ReportAction, ReportEntry, Into, Self, String

### Community 165 - "UnifiedMod"
Cohesion: 0.35
Nodes (10): fetch_versions(), Provider, From, Option, Result, String, Vec, search_mods() (+2 more)

### Community 166 - "OptionsDocument"
Cohesion: 0.22
Nodes (7): OptionsDocument, Option, Path, Result, Self, String, Vec

### Community 168 - "ModpacksView.svelte"
Cohesion: 0.47
Nodes (3): onSearchKey(), search(), setSource()

### Community 169 - "HorizonLayout"
Cohesion: 0.35
Nodes (3): HorizonLayout, OfficialInstall, PathBuf

## Ambiguous Edges - Review These
- `inventory-shell.md` → `scratch.txt (per-file special-character scratch dump)`  [AMBIGUOUS]
  scratch.txt · relation: references
- `PARITY.md (parity ledger)` → `scratch.txt (per-file special-character scratch dump)`  [AMBIGUOUS]
  scratch.txt · relation: references
- `README.md (Glacier Launcher overview)` → `scratch.txt (per-file special-character scratch dump)`  [AMBIGUOUS]
  scratch.txt · relation: references

## Knowledge Gaps
- **192 isolated node(s):** `gl-auth`, `gl-bedrock`, `gl-clients`, `gl-integrations`, `gl-modpack` (+187 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 803 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **6 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **What is the exact relationship between `inventory-shell.md` and `scratch.txt (per-file special-character scratch dump)`?**
  _Edge tagged AMBIGUOUS (relation: references) - confidence is low._
- **What is the exact relationship between `PARITY.md (parity ledger)` and `scratch.txt (per-file special-character scratch dump)`?**
  _Edge tagged AMBIGUOUS (relation: references) - confidence is low._
- **What is the exact relationship between `README.md (Glacier Launcher overview)` and `scratch.txt (per-file special-character scratch dump)`?**
  _Edge tagged AMBIGUOUS (relation: references) - confidence is low._
- **Why does `AppState` connect `AppState` to `launch.rs`, `gl-auth/src/lib.rs`, `utilities.rs`, `ai.rs`, `java_loaders.rs`, `BedrockInstanceService`, `themes.rs`, `java_process.rs`, `sync_packs.rs`, `versions.rs`, `worlds.rs`, `update.rs`, `modpack.rs`, `launchers.rs`, `clients.rs`, `.new`, `src/notifications.rs`, `logs.rs`, `SyncReport`, `bedrock_addons.rs`, `commands/notifications.rs`, `java_packs.rs`, `DownloadQueue`, `commands/bedrock.rs`, `launcher_import.rs`, `java_mods.rs`, `commands/java.rs`, `DiscordRpcService`, `integrations.rs`, `commands/levilamina_mods.rs`, `commands/skins.rs`, `commands/tweaks.rs`, `commands/client_library.rs`, `commands/glacier_client.rs`, `java_screenshots.rs`?**
  _High betweenness centrality (0.492) - this node is a cross-community bridge._
- **Why does `BedrockInstanceService` connect `BedrockInstanceService` to `AppState`, `paths.rs`?**
  _High betweenness centrality (0.076) - this node is a cross-community bridge._
- **Why does `DownloadQueue` connect `DownloadQueue` to `mod_loader.rs`, `queue.rs`, `AppState`, `DownloadClient`, `DownloadHandle`?**
  _High betweenness centrality (0.054) - this node is a cross-community bridge._
- **What connects `gl-auth`, `gl-bedrock`, `gl-clients` to the rest of the system?**
  _192 weakly-connected nodes found - possible documentation gaps or missing edges._