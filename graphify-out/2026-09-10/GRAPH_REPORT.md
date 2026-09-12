# Graph Report - MinecraftLauncher  (2026-09-10)

## Corpus Check
- 223 files · ~218,094 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 2904 nodes · 6700 edges · 119 communities (103 shown, 6 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 112 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `9b5d697e`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- InstanceService
- launcher.rs
- launch.rs
- ShaderService
- releases.rs
- gl-auth/src/lib.rs
- utilities.rs
- gl-nbt/src/lib.rs
- io_err
- resume.rs
- BedrockInstanceService
- themes.rs
- GitHubApiCache
- version.rs
- gl-injection/src/lib.rs
- versions.rs
- LauncherRelease
- App.svelte
- worlds.rs
- UpdateChecker
- levilamina.rs
- gl-settings/src/lib.rs
- mod_analyzer.rs
- gdk.rs
- .new
- CLAUDE.md
- server_ping.rs
- gl-clients/src/hash.rs
- lunar_badlion.rs
- parse_version_db
- frontend/package.json
- clients.rs
- java_screenshots.rs
- src/notifications.rs
- tauri.conf.json
- BedrockOptionsService
- wu.rs
- sha1_hex_file
- logs.rs
- PARITY.md (parity ledger)
- skin_library.rs
- src/bedrock.rs
- VersionsView.svelte
- LoaderClient
- []
- flarial.rs
- modpack.rs
- AppState
- DownloadQueue
- paths.rs
- src/tweaks.rs
- bedrock_addons.rs
- update.rs
- []
- collector
- VersionService
- ClientLibraryView.svelte
- import.rs
- gl-clients/src/models.rs
- gl-download/src/download.rs
- gl-persistence/src/lib.rs
- commands/bedrock.rs
- queue.rs
- RuntimeDownloadService
- java_mods.rs
- src/client_library.rs
- src/skins.rs
- gl-win32chrome/src/lib.rs
- commands/java.rs
- system.rs
- src/glacier_client.rs
- DiscordRpcService
- SkinViewer.svelte
- compilerOptions
- downloads.rs
- integrations.rs
- latite.rs
- throwaway_host.rs
- gl-integrations/src/models.rs
- commands/levilamina_mods.rs
- commands/skins.rs
- commands/tweaks.rs
- commands/client_library.rs
- compilerOptions
- glacier-launcher
- BedrockScreenshotsView.svelte
- commands/glacier_client.rs
- DownloadRequest
- settings.ts
- interop.js
- BedrockError
- oderso.rs
- ThemeStudioView.svelte
- DownloadError
- DownloadClient
- GlacierClientVersion
- ModpacksView.svelte
- package.json
- setup_tray
- default.json
- get_settings
- JavaInstance
- ModpackError
- inject_bedrock_client
- ClientsError
- IntegrationError
- gl-update/src/error.rs
- tsconfig.json
- interop.d.ts

## God Nodes (most connected - your core abstractions)
1. `AppState` - 205 edges
2. `[]` - 43 edges
3. `[]` - 40 edges
4. `InstanceService` - 30 edges
5. `BedrockInstanceService` - 26 edges
6. `ThemeDefinition` - 25 edges
7. `DownloadQueue` - 23 edges
8. `DownloadRequest` - 19 edges
9. `io_err()` - 18 edges
10. `read_version_profile()` - 18 edges

## Surprising Connections (you probably didn't know these)
- `frontend/README.md (Svelte+Vite template doc)` --semantically_similar_to--> `Glacier Launcher (product)`  [INFERRED] [semantically similar]
  frontend/README.md → README.md
- `scratch.txt (per-file special-character scratch dump)` --references--> `PARITY.md (parity ledger)`  [AMBIGUOUS]
  scratch.txt → PARITY.md
- `scratch.txt (per-file special-character scratch dump)` --references--> `README.md (Glacier Launcher overview)`  [AMBIGUOUS]
  scratch.txt → README.md
- `glacier-launcher` --depends_on--> `gl-auth`  [EXTRACTED]
  src-tauri/Cargo.toml → crates/gl-auth/Cargo.toml
- `glacier-launcher` --depends_on--> `gl-bedrock`  [EXTRACTED]
  src-tauri/Cargo.toml → crates/gl-bedrock/Cargo.toml

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **No path to install a current Bedrock build (GdkArchiveService + StoreInstallService + WU cap)** — concept_gdkarchiveservice, concept_storeinstallservice, concept_wu_rs [EXTRACTED 1.00]
- **CI/build verification gates for the cutover commit** — github_workflows_ci, github_workflows_build, concept_cutover_commit [EXTRACTED 1.00]
- **Transcription ledger and its supporting line-range inventories** — parity, docs_inventory_shell, docs_inventory_bedrock_panels, docs_inventory_bedrock_services [EXTRACTED 1.00]

## Communities (119 total, 6 thin omitted)

### Community 0 - "InstanceService"
Cohesion: 0.13
Nodes (23): copy_dir_recursive(), ensure_layout(), format_unix_secs(), InstanceService, load_index(), now_iso(), JavaInstance, Mutex (+15 more)

### Community 1 - "launcher.rs"
Cohesion: 0.06
Nodes (78): AuthValues, build_classpath(), build_game_args(), build_jvm_args(), collect_args(), detect_java_major(), features_match(), find_first_javaw_under() (+70 more)

### Community 2 - "launch.rs"
Cohesion: 0.14
Nodes (39): available_ram_mb(), cancel_launch(), chrono_expired(), console_content_title(), console_stop_game(), ConsoleExitPayload, ConsoleLogPayload, ConsoleStatusPayload (+31 more)

### Community 3 - "ShaderService"
Cohesion: 0.13
Nodes (28): AppliedShader, Into, Path, PathBuf, Result, Self, String, Vec (+20 more)

### Community 4 - "releases.rs"
Cohesion: 0.22
Nodes (15): asset_falls_back_to_first_when_no_exe(), asset_prefers_exe_over_other_files(), fixture(), no_update_when_current_is_latest(), pick_asset(), pick_update(), picks_newest_non_draft_with_newer_version(), recent_releases() (+7 more)

### Community 5 - "gl-auth/src/lib.rs"
Cohesion: 0.06
Nodes (67): B, authenticate_with_minecraft(), AuthError, DisplayClaims, exchange_code(), exchange_code_against_live_endpoint(), exchange_code_for_live_token(), exchange_for_xbl_token() (+59 more)

### Community 6 - "utilities.rs"
Cohesion: 0.06
Nodes (79): blank_entry(), decide_update(), decide_update_detects_matching_and_changed_hash(), hash_and_size(), head_matches_recorded_size(), index_path(), kind_for_extension(), library_dir() (+71 more)

### Community 7 - "gl-nbt/src/lib.rs"
Cohesion: 0.06
Nodes (51): apply_summary_to_root(), find_byte(), find_int(), gzip_compress(), gzip_decompress(), gzip_helpers_round_trip(), level_dat_rejects_bad_header(), level_dat_round_trip_with_header() (+43 more)

### Community 8 - "io_err"
Cohesion: 0.05
Nodes (76): download_to_file(), get_string(), Client, Path, Result, String, io_err(), JavaError (+68 more)

### Community 9 - "resume.rs"
Cohesion: 0.32
Nodes (12): clear_pending_marker(), clear_pending_marker_is_idempotent(), marker_path(), marker_pointing_at_missing_file_is_ignored(), marker_pointing_at_real_file_is_returned_and_consumed(), no_marker_returns_none(), Option, Path (+4 more)

### Community 10 - "BedrockInstanceService"
Cohesion: 0.06
Nodes (53): BedrockBackupService, BedrockRoot, Into, Option, Path, PathBuf, Result, Self (+45 more)

### Community 11 - "themes.rs"
Cohesion: 0.07
Nodes (48): css_vars_cover_every_variable_the_original_emits(), duplicated_gets_a_fresh_id(), exports_dir(), fmt3(), light_text_uses_subtler_item_overlays(), lighten(), new_id(), overlay_strength_scales_the_overlay_alphas() (+40 more)

### Community 12 - "GitHubApiCache"
Cohesion: 0.22
Nodes (13): CachedResponse, Entry, expires_after_ttl(), GitHubApiCache, hit_after_store(), miss_when_empty(), HashMap, Instant (+5 more)

### Community 13 - "version.rs"
Cohesion: 0.20
Nodes (5): is_newer_version(), normalize_tag(), parse_version(), Option, String

### Community 14 - "gl-injection/src/lib.rs"
Cohesion: 0.06
Nodes (47): c_void, activate_application(), enable_debug_privilege(), find_process_by_name(), find_processes_by_name(), grant_uwp_access(), inject_dll(), inject_rejects_missing_dll_before_touching_win32() (+39 more)

### Community 15 - "versions.rs"
Cohesion: 0.21
Nodes (28): WuVersion, available_free_space(), delete_latite_version(), delete_vanilla_bedrock_version(), download_latite_version(), download_vanilla_bedrock_version(), ensure_disk_space(), latite_dir() (+20 more)

### Community 16 - "LauncherRelease"
Cohesion: 0.31
Nodes (8): GitHubAsset, GitHubRelease, LauncherRelease, LauncherUpdateInfo, GitHubAsset, Option, String, Vec

### Community 18 - "worlds.rs"
Cohesion: 0.10
Nodes (51): add_dir_to_zip(), backup_of_a_missing_world_is_an_error(), backup_then_restore_round_trips_a_world(), backup_world(), datapacks_dir(), datapacks_list_files_and_folders(), delete_backup(), delete_backup_never_panics_on_a_missing_file() (+43 more)

### Community 19 - "UpdateChecker"
Cohesion: 0.39
Nodes (5): Client, Result, Self, String, UpdateChecker

### Community 20 - "levilamina.rs"
Cohesion: 0.07
Nodes (42): detects_not_found_payload(), falls_back_to_first_zip_when_no_windows_asset(), GitHubAsset, GitHubRelease, mod_asset_prefers_zip_over_other_files(), parse_release(), parses_release_and_picks_windows_zip(), pick_mod_asset() (+34 more)

### Community 21 - "gl-settings/src/lib.rs"
Cohesion: 0.21
Nodes (16): cache_dir(), default_settings_round_trip_through_persistence(), JavaAccount, launcher_root(), LauncherSettings, legacy_settings_path(), load(), Default (+8 more)

### Community 22 - "mod_analyzer.rs"
Cohesion: 0.09
Nodes (48): analyze(), analyze_dir(), analyzes_fabric_mod_json(), analyzes_forge_mods_toml(), built_ins(), capture(), delete_mod(), is_disabled_jar() (+40 more)

### Community 23 - "gdk.rs"
Cohesion: 0.18
Nodes (13): a_missing_or_odd_digest_yields_no_hash_rather_than_a_bad_one(), an_error_body_is_an_empty_list_not_a_failure(), list_versions(), maps_release_fields_and_skips_drafts(), parse_asset_name(), parse_releases(), Client, Option (+5 more)

### Community 24 - ".new"
Cohesion: 0.50
Nodes (3): Result, Self, String

### Community 26 - "server_ping.rs"
Cohesion: 0.08
Nodes (49): extract_between(), LanGame, parse_lan_packet(), parses_well_formed_lan_packet(), Option, Result, String, Vec (+41 more)

### Community 27 - "gl-clients/src/hash.rs"
Cohesion: 0.21
Nodes (6): AsRef, Path, Result, String, sha256_hex_bytes(), sha256_hex_file()

### Community 28 - "lunar_badlion.rs"
Cohesion: 0.12
Nodes (34): detect(), detects_lunar_exe_at_known_path(), DirectLaunchFiles, find_badlion(), find_lunar(), find_lunar_jre(), list_version_dirs(), lists_offline_versions_sorted_descending() (+26 more)

### Community 29 - "parse_version_db"
Cohesion: 0.31
Nodes (9): first_seen_wins_on_duplicate_key(), list_versions(), parse_version_db(), Client, Result, Vec, skips_section_headers_eappx_and_non_x64(), sorts_newest_first() (+1 more)

### Community 30 - "frontend/package.json"
Cohesion: 0.05
Nodes (38): dependencies, skinview3d, @tauri-apps/api, @tauri-apps/plugin-dialog, @tauri-apps/plugin-log, devDependencies, esbuild, svelte (+30 more)

### Community 31 - "clients.rs"
Cohesion: 0.17
Nodes (39): OderSoDllEntry, active_oderso_dll(), check_flarial_dll_up_to_date(), check_levilamina_up_to_date(), check_oderso_up_to_date(), delete_flarial_client(), delete_levilamina_client(), delete_oderso_client() (+31 more)

### Community 32 - "java_screenshots.rs"
Cohesion: 0.13
Nodes (24): ScreenshotFile, BedrockScreenshotService, Into, Option, Path, PathBuf, Result, Self (+16 more)

### Community 33 - "src/notifications.rs"
Cohesion: 0.14
Nodes (22): AppNotification, detached(), ids_are_unique_and_guid_n_shaped(), items_come_back_newest_first(), mark_all_read_clears_the_count(), new_id(), note(), notifications_path() (+14 more)

### Community 34 - "tauri.conf.json"
Cohesion: 0.06
Nodes (31): debugApplicationIdSuffix, app, security, trayIcon, windows, enable, scope, build (+23 more)

### Community 35 - "BedrockOptionsService"
Cohesion: 0.13
Nodes (18): append_ext(), BedrockOptionsService, find_meta(), infer_kind(), known_keys(), Meta, prettify(), HashMap (+10 more)

### Community 36 - "wu.rs"
Cohesion: 0.17
Nodes (24): extract_appx(), fetch_updates(), find_text(), generic_version(), get_auth_cookie(), get_download_url(), is_dotted_number(), list_versions() (+16 more)

### Community 37 - "sha1_hex_file"
Cohesion: 0.43
Nodes (7): hex_encode(), known_sha1_of_abc(), AsRef, Path, Result, String, sha1_hex_file()

### Community 38 - "logs.rs"
Cohesion: 0.18
Nodes (28): chrono_like_day(), civil_from_days(), CrashReportInfo, day_to_iso(), days_from_civil(), get_stats_summary(), latest_java_crash_report(), list_logs() (+20 more)

### Community 39 - "PARITY.md (parity ledger)"
Cohesion: 0.13
Nodes (25): Bedrock-first, then Java (order of work), BedrockContentView merged-panels defect (Worlds/Packs/Backups/Screenshots), BedrockTabs (bottom tab strip, Home.BigFeatures.cs:54-75), Missing com.mojang root resolution (highest-risk finding), Console window rendered main app (console.html missing bug), rust-tauri-cutover branch cutover commit, GdkArchiveService (Bedrock GDK version source), gl-injection crate (Win32 DLL injection port) (+17 more)

### Community 40 - "skin_library.rs"
Cohesion: 0.17
Nodes (25): add_from_file(), add_from_username(), base64_decode(), base64_roundtrip_matches_known_vector(), delete_skin(), list_skins(), MojangProfileByName, resolve_skin_texture() (+17 more)

### Community 41 - "src/bedrock.rs"
Cohesion: 0.19
Nodes (25): activate_via_com(), BedrockPackageInfo, connect_external_server(), escape_data_string(), extract_application_id(), find_game_process(), find_game_process_for(), installed_version() (+17 more)

### Community 42 - "VersionsView.svelte"
Cohesion: 0.10
Nodes (17): cancelFor(), currentSortIcon, displayName(), filteredOderSo, filteredVersions, handleOderSoDownload(), isPinned(), loadOderSoVersions() (+9 more)

### Community 46 - "LoaderClient"
Cohesion: 0.18
Nodes (13): extract_xml_tag_values(), extracts_xml_versions(), is_pre_release(), LoaderClient, Client, Result, Self, String (+5 more)

### Community 47 - "[]"
Cohesion: 0.07
Nodes (26): [], activeCapeUrl, activeInstance, cloneJavaInstance(), commitGroup(), commitRenameInstance(), confirmDeleteInstanceId, deleteJavaInstance() (+18 more)

### Community 48 - "flarial.rs"
Cohesion: 0.26
Nodes (9): fetch_remote_hash(), local_hash(), local_hash_matches_known_file_contents(), parse_remote_hash(), parses_release_hash_from_fixture(), Client, Path, Result (+1 more)

### Community 49 - "modpack.rs"
Cohesion: 0.06
Nodes (67): errors_when_manifest_missing(), loader_profile_id(), parse_curseforge_manifest(), parse_loader_id(), parse_modrinth_index(), parses_curseforge_manifest_fixture(), parses_modrinth_index_fixture_and_skips_unsupported_client_files(), read_zip_json() (+59 more)

### Community 50 - "AppState"
Cohesion: 0.20
Nodes (18): AppNotification, JavaInstanceService, JavaVersionService, add_notification(), clear_notifications(), list_notifications(), mark_all_notifications_read(), mark_notification_read() (+10 more)

### Community 51 - "DownloadQueue"
Cohesion: 0.16
Nodes (14): Arc, clear_finished_also_drops_queued_entries(), DownloadQueue, new_entry_starts_queued_with_zero_progress(), queue_caps_concurrency_at_configured_limit(), remove_and_clear_notify_subscribers(), Duration, Mutex (+6 more)

### Community 52 - "paths.rs"
Cohesion: 0.22
Nodes (22): com_mojang_root(), counts_directories_as_entries(), empty_subfolder_never_wins(), falls_back_to_shared_when_no_account_has_the_subfolder(), falls_back_to_shared_when_users_root_is_absent(), live_dir_for(), live_root_for(), minecraft_pe_dir() (+14 more)

### Community 53 - "src/tweaks.rs"
Cohesion: 0.19
Nodes (22): BedrockManifest, BedrockManifestHeader, BedrockManifestModule, BedrockTweaksIndex, build_bedrock_manifest(), builds_manifest_for_single_pack(), flatten_vanilla(), flattens_vanilla_categories_recursively() (+14 more)

### Community 54 - "bedrock_addons.rs"
Cohesion: 0.10
Nodes (43): CurseForgeClient, html_decode(), parse_addon(), parse_file(), parses_search_addon_fixture(), Client, Into, Option (+35 more)

### Community 55 - "update.rs"
Cohesion: 0.35
Nodes (14): check_launcher_update(), download_launcher_update(), install_launcher_update(), launcher_version(), list_client_releases(), list_glacier_client_releases(), list_launcher_releases(), resume_pending_launcher_update() (+6 more)

### Community 56 - "[]"
Cohesion: 0.08
Nodes (22): [], closeServerModal(), _confirmDeleteServerId, deleteServer(), getServerIconStyle(), isServerIconImage(), lanError, lanScanning (+14 more)

### Community 57 - "collector"
Cohesion: 0.18
Nodes (18): collector(), enough_time_but_not_enough_delta_is_suppressed(), first_update_always_reported(), reports_once_interval_and_delta_both_satisfied(), Duration, FnMut, Instant, Option (+10 more)

### Community 58 - "VersionService"
Cohesion: 0.21
Nodes (15): CachedVersion, dirs_home(), ManifestLatest, ManifestRoot, ManifestVersionEntry, Client, JavaVersion, Option (+7 more)

### Community 60 - "ClientLibraryView.svelte"
Cohesion: 0.10
Nodes (14): app, _clError, _clFilter, clFiltered, _clLoading, _clProgress, _clVersion, isClientPackExpanded() (+6 more)

### Community 62 - "import.rs"
Cohesion: 0.25
Nodes (20): copy_dir_recursive(), extract_to_temp(), import_content(), import_packs(), import_world(), ImportOutcome, imports_mcaddon_bundle_routes_by_module_type(), imports_mcpack_by_resource_module_type() (+12 more)

### Community 63 - "gl-clients/src/models.rs"
Cohesion: 0.15
Nodes (11): ClientDownloadState, ClientInfo, ClientLibraryPack, ClientType, default_kind(), download_state_lifecycle(), GlacierLauncherMeta, GlacierManifest (+3 more)

### Community 64 - "gl-download/src/download.rs"
Cohesion: 0.24
Nodes (17): backoff_for(), commit(), download_file(), download_once(), download_with_retry(), DownloadOutcome, is_destination_busy(), CancellationToken (+9 more)

### Community 66 - "gl-persistence/src/lib.rs"
Cohesion: 0.18
Nodes (18): append_extension(), Envelope, load(), no_leftover_temp_file_after_write(), PersistError, quarantine_corrupt(), round_trips_through_atomic_write_and_load(), Error (+10 more)

### Community 67 - "commands/bedrock.rs"
Cohesion: 0.06
Nodes (78): backup_create_and_restore_roundtrip(), backup_with_nothing_present_errors(), instance_create_list_rename_delete_roundtrip(), instance_ids_deduplicate_on_slug_collision(), instance_switch_swaps_live_folders(), options_service_load_save_and_restore(), pack_service_lists_by_manifest_name(), restore_never_overwrites_without_a_prior_backup_safety_copy() (+70 more)

### Community 68 - "queue.rs"
Cohesion: 0.21
Nodes (11): DownloadEntry, DownloadHandle, DownloadStatus, RegistryEntry, CancellationToken, Option, Result, String (+3 more)

### Community 69 - "RuntimeDownloadService"
Cohesion: 0.21
Nodes (14): AdoptiumBinary, AdoptiumPackage, AdoptiumRelease, extract_zip(), find_first_named(), Client, FnMut, Option (+6 more)

### Community 72 - "java_mods.rs"
Cohesion: 0.22
Nodes (35): check_java_mod_updates(), delete_java_mod(), diff_java_instances(), download_forge_installer(), download_neoforge_installer(), install_fabric_loader(), install_local_mod_jar(), install_modrinth_mod() (+27 more)

### Community 74 - "src/client_library.rs"
Cohesion: 0.19
Nodes (16): encode_path(), index_url(), media_url(), parse_index(), parses_and_groups_by_version_and_name(), PathEntry, raw_url(), Result (+8 more)

### Community 75 - "src/skins.rs"
Cohesion: 0.21
Nodes (15): CapeInfo, ApiErrorBody, extract_api_message(), get_capes(), hide_cape(), ProfileResponse, RawCape, reset_skin() (+7 more)

### Community 77 - "gl-win32chrome/src/lib.rs"
Cohesion: 0.17
Nodes (12): apply_chrome(), hit_test(), hittest_from_lparam(), hittest_subclass_proc(), HitTestResult, install_resize_hittest(), Option, WinResult (+4 more)

### Community 78 - "commands/java.rs"
Cohesion: 0.23
Nodes (27): JvmPreset, create_java_instance(), create_java_instance_from_template(), delete_java_instance(), duplicate_java_instance(), export_java_instance(), import_java_instance_zip(), import_official_minecraft() (+19 more)

### Community 79 - "system.rs"
Cohesion: 0.23
Nodes (16): close_launcher_window(), export_settings(), exports_dir(), get_launcher_root(), import_latest_settings(), open_folder(), open_path(), open_url() (+8 more)

### Community 81 - "src/glacier_client.rs"
Cohesion: 0.20
Nodes (11): fetch_manifest(), glacier_dir(), installed_size_bytes(), parse_manifest(), parses_manifest_camel_case_fields(), require_installed(), Client, Path (+3 more)

### Community 82 - "DiscordRpcService"
Cohesion: 0.23
Nodes (7): DiscordRpcService, now_unix(), Default, Drop, Option, Self, DiscordIpcClient

### Community 84 - "SkinViewer.svelte"
Cohesion: 0.12
Nodes (11): bodyRenderUrl, capeLabel, capeMode, cleanUuid, displayName, failed, fallbackRenderUrl, model (+3 more)

### Community 85 - "compilerOptions"
Cohesion: 0.12
Nodes (16): compilerOptions, allowImportingTsExtensions, erasableSyntaxOnly, lib, module, moduleDetection, noEmit, noFallthroughCasesInSwitch (+8 more)

### Community 86 - "downloads.rs"
Cohesion: 0.18
Nodes (15): cancel_download(), clear_finished_downloads(), DownloadEntryDto, list_downloads(), remove_download(), AppHandle, From, Option (+7 more)

### Community 87 - "integrations.rs"
Cohesion: 0.17
Nodes (27): NewsPost, decode(), decode_entity(), fetch_client_changelog(), get_news(), html_decode(), parses_fixture_response(), Client (+19 more)

### Community 90 - "latite.rs"
Cohesion: 0.28
Nodes (14): dll_file_name(), falls_back_to_name_and_date_for_non_standard_tags(), format_display_name(), format_published_at(), LatiteVersion, list_versions(), parse_latite_tag(), parse_versions() (+6 more)

### Community 91 - "throwaway_host.rs"
Cohesion: 0.25
Nodes (12): HostGuard, injecting_into_a_dead_process_fails_cleanly(), injects_a_real_dll_into_a_throwaway_process(), missing_dll_is_reported_before_touching_the_process(), process_lookup_finds_a_spawned_host_and_loses_it_after_exit(), Child, Drop, PathBuf (+4 more)

### Community 92 - "gl-integrations/src/models.rs"
Cohesion: 0.16
Nodes (17): get_announcement(), Client, Option, Announcement, AppNotification, default_icon(), default_kind(), new_id() (+9 more)

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

### Community 98 - "compilerOptions"
Cohesion: 0.13
Nodes (14): compilerOptions, allowArbitraryExtensions, allowJs, checkJs, module, moduleDetection, noEmit, target (+6 more)

### Community 99 - "glacier-launcher"
Cohesion: 0.16
Nodes (15): gl-auth, gl-bedrock, gl-clients, gl-download, gl-injection, gl-integrations, gl-java, gl-launch (+7 more)

### Community 102 - "commands/glacier_client.rs"
Cohesion: 0.30
Nodes (12): get_glacier_manifest(), glacier_installed_size_bytes(), install_glacier_client(), launch_glacier_client(), AppHandle, Option, PathBuf, Result (+4 more)

### Community 104 - "DownloadRequest"
Cohesion: 0.29
Nodes (8): DownloadRequest, has_real_hash(), Into, Option, PathBuf, Self, String, validate_rejects_empty_url()

### Community 106 - "settings.ts"
Cohesion: 0.11
Nodes (9): accessibilityMode, BedrockInstance, JavaAccount, JavaInstance, JavaVersion, JvmPreset, LauncherSettings, PrelaunchWarning (+1 more)

### Community 107 - "interop.js"
Cohesion: 0.22
Nodes (9): comboFromEvent(), ensureTabCyclers(), matchesAction(), pageSizeFor(), TODO: swap for @tauri-apps/plugin-opener once that plugin is added to, REBINDABLE_DEFAULTS, refreshCyclerButtons(), tabsOf() (+1 more)

### Community 108 - "BedrockError"
Cohesion: 0.23
Nodes (10): BedrockError, IoContext, Error, Into, PathBuf, Result, String, T (+2 more)

### Community 109 - "oderso.rs"
Cohesion: 0.32
Nodes (10): DllEntry, list_dlls(), parse_dll_list(), parses_and_sorts_dll_entries_newest_first(), raw_url(), Client, Result, String (+2 more)

### Community 110 - "ThemeStudioView.svelte"
Cohesion: 0.20
Nodes (5): LanguageOption, LANGUAGES, t(), Table, FONT_SUGGESTIONS

### Community 112 - "DownloadError"
Cohesion: 0.27
Nodes (7): Box, DownloadError, Error, Into, PathBuf, Self, String

### Community 115 - "DownloadClient"
Cohesion: 0.31
Nodes (4): DownloadClient, Client, Result, Self

### Community 116 - "GlacierClientVersion"
Cohesion: 0.46
Nodes (3): GlacierClientVersion, Path, PathBuf

### Community 118 - "ModpacksView.svelte"
Cohesion: 0.32
Nodes (5): onSearchKey(), search(), setModpacksViewMode(), setSource(), saveViewMode()

### Community 119 - "package.json"
Cohesion: 0.25
Nodes (7): description, name, private, scripts, build, dev, version

### Community 121 - "setup_tray"
Cohesion: 0.38
Nodes (6): apply_native_window_chrome(), AppHandle, Result, run(), setup_tray(), WebviewWindow

### Community 123 - "default.json"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 124 - "get_settings"
Cohesion: 0.60
Nodes (5): get_settings(), LauncherSettings, Result, String, set_settings()

### Community 125 - "JavaInstance"
Cohesion: 0.33
Nodes (5): JavaInstance, JavaVersion, Option, String, Vec

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

## Ambiguous Edges - Review These
- `inventory-shell.md` → `scratch.txt (per-file special-character scratch dump)`  [AMBIGUOUS]
  scratch.txt · relation: references
- `PARITY.md (parity ledger)` → `scratch.txt (per-file special-character scratch dump)`  [AMBIGUOUS]
  scratch.txt · relation: references
- `README.md (Glacier Launcher overview)` → `scratch.txt (per-file special-character scratch dump)`  [AMBIGUOUS]
  scratch.txt · relation: references

## Knowledge Gaps
- **171 isolated node(s):** `gl-auth`, `gl-bedrock`, `gl-clients`, `gl-download`, `gl-integrations` (+166 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 624 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **6 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **What is the exact relationship between `inventory-shell.md` and `scratch.txt (per-file special-character scratch dump)`?**
  _Edge tagged AMBIGUOUS (relation: references) - confidence is low._
- **What is the exact relationship between `PARITY.md (parity ledger)` and `scratch.txt (per-file special-character scratch dump)`?**
  _Edge tagged AMBIGUOUS (relation: references) - confidence is low._
- **What is the exact relationship between `README.md (Glacier Launcher overview)` and `scratch.txt (per-file special-character scratch dump)`?**
  _Edge tagged AMBIGUOUS (relation: references) - confidence is low._
- **Why does `AppState` connect `AppState` to `launch.rs`, `gl-auth/src/lib.rs`, `utilities.rs`, `BedrockInstanceService`, `themes.rs`, `versions.rs`, `worlds.rs`, `.new`, `clients.rs`, `java_screenshots.rs`, `src/notifications.rs`, `logs.rs`, `modpack.rs`, `DownloadQueue`, `bedrock_addons.rs`, `update.rs`, `commands/bedrock.rs`, `java_mods.rs`, `commands/java.rs`, `DiscordRpcService`, `downloads.rs`, `integrations.rs`, `commands/levilamina_mods.rs`, `commands/skins.rs`, `commands/tweaks.rs`, `commands/client_library.rs`, `commands/glacier_client.rs`?**
  _High betweenness centrality (0.450) - this node is a cross-community bridge._
- **Why does `BedrockInstanceService` connect `BedrockInstanceService` to `AppState`, `commands/bedrock.rs`?**
  _High betweenness centrality (0.082) - this node is a cross-community bridge._
- **Why does `ServerStatus` connect `server_ping.rs` to `gl-integrations/src/models.rs`?**
  _High betweenness centrality (0.048) - this node is a cross-community bridge._
- **Are the 4 inferred relationships involving `[]` (e.g. with `cancelDeleteInstance()` and `downloadPlayerSkin()`) actually correct?**
  _`[]` has 4 INFERRED edges - model-reasoned connections that need verification._