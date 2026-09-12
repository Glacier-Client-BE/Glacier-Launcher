# Graph Report - MinecraftLauncher  (2026-09-12)

## Corpus Check
- 275 files · ~257,838 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 3554 nodes · 8272 edges · 158 communities (140 shown, 7 thin omitted)
- Extraction: 98% EXTRACTED · 2% INFERRED · 0% AMBIGUOUS · INFERRED: 151 edges (avg confidence: 0.85)
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
- ImportLauncherView.svelte
- gl-auth/src/lib.rs
- utilities.rs
- gl-nbt/src/lib.rs
- java_loaders.rs
- resume.rs
- BedrockInstanceService
- themes.rs
- releases.rs
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
- gdk.rs
- .new
- CLAUDE.md
- server_ping.rs
- gl-clients/src/hash.rs
- lunar_badlion.rs
- parse_version_db
- frontend/package.json
- AppState
- mmc.rs
- src/notifications.rs
- tauri.conf.json
- BedrockOptionsService
- wu.rs
- export_mrpack
- logs.rs
- PARITY.md (parity ledger)
- skin_library.rs
- src/bedrock.rs
- VersionsView.svelte
- mod_loader.rs
- SyncReport
- CurseForgeClient
- LoaderClient
- []
- flarial.rs
- modpack.rs
- commands/notifications.rs
- DownloadQueue
- paths.rs
- src/tweaks.rs
- bedrock_addons.rs
- SyncEngine
- []
- .install
- VersionService
- jar_inspect.rs
- ClientLibraryView.svelte
- ModrinthClient
- gl-bedrock/src/lib.rs
- gl-clients/src/models.rs
- gl-download/src/download.rs
- java_packs.rs
- gl-persistence/src/lib.rs
- commands/bedrock.rs
- queue.rs
- RuntimeDownloadService
- launcher_import.rs
- modpack_transfer.rs
- java_mods.rs
- gl-modpack/src/models.rs
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
- downloads.rs
- integrations.rs
- io_err
- SyncPack
- latite.rs
- throwaway_host.rs
- gl-integrations/src/models.rs
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
- OptionsDocument
- DownloadRequest
- atlauncher.rs
- settings.ts
- interop.js
- BedrockError
- oderso.rs
- ThemeStudioView.svelte
- .parse
- DownloadError
- modrinth_app.rs
- UnifiedMod
- DownloadClient
- GlacierClientVersion
- check
- ModpacksView.svelte
- package.json
- presets.rs
- setup_tray
- InstallProfile
- default.json
- get_settings
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
- collect_args
- java_process.rs
- resolve_modrinth_node
- loader_map.rs
- apply
- list_jar_names
- collector
- processors.rs
- io_err
- pipeline.rs
- get_string
- patcher.rs

## God Nodes (most connected - your core abstractions)
1. `AppState` - 250 edges
2. `[]` - 43 edges
3. `InstanceService` - 40 edges
4. `[]` - 40 edges
5. `DownloadQueue` - 28 edges
6. `BedrockInstanceService` - 26 edges
7. `ThemeDefinition` - 25 edges
8. `SyncReport` - 25 edges
9. `io_err()` - 24 edges
10. `ImportCandidate` - 23 edges

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

## Communities (158 total, 7 thin omitted)

### Community 0 - "InstanceService"
Cohesion: 0.09
Nodes (36): count_files_with_ext(), count_mods(), count_worlds(), dir_size_bytes(), Path, ensure_enough_disk_space(), ensure_no_id_collision(), free_disk_bytes() (+28 more)

### Community 1 - "launcher.rs"
Cohesion: 0.20
Nodes (27): build_classpath(), build_game_args(), build_jvm_args(), detect_java_major(), find_first_javaw_under(), find_javaw_in_registry(), heuristic_major_from_path(), is_jvm_arg_compatible() (+19 more)

### Community 2 - "launch.rs"
Cohesion: 0.12
Nodes (44): available_ram_mb(), cancel_launch(), chrono_expired(), console_content_title(), console_stop_game(), ConsoleExitPayload, ConsoleLogPayload, ConsoleStatusPayload (+36 more)

### Community 3 - "ShaderService"
Cohesion: 0.13
Nodes (28): AppliedShader, Into, Path, PathBuf, Result, Self, String, Vec (+20 more)

### Community 4 - "ImportLauncherView.svelte"
Cohesion: 0.11
Nodes (12): goToPreview(), importingIndex, loading, openLauncher(), pickCustomRoot(), runDetection(), runImport(), string (+4 more)

### Community 5 - "gl-auth/src/lib.rs"
Cohesion: 0.05
Nodes (67): B, authenticate_with_minecraft(), AuthError, DisplayClaims, exchange_code(), exchange_code_against_live_endpoint(), exchange_code_for_live_token(), exchange_for_xbl_token() (+59 more)

### Community 6 - "utilities.rs"
Cohesion: 0.06
Nodes (79): blank_entry(), decide_update(), decide_update_detects_matching_and_changed_hash(), hash_and_size(), head_matches_recorded_size(), index_path(), kind_for_extension(), library_dir() (+71 more)

### Community 7 - "gl-nbt/src/lib.rs"
Cohesion: 0.06
Nodes (51): apply_summary_to_root(), find_byte(), find_int(), gzip_compress(), gzip_decompress(), gzip_helpers_round_trip(), level_dat_rejects_bad_header(), level_dat_round_trip_with_header() (+43 more)

### Community 8 - "java_loaders.rs"
Cohesion: 0.30
Nodes (17): get_instance_loader(), install_loader(), InstanceLoaderDto, java_root_dir(), list_loader_versions(), LoaderInstallProgress, parse_loader(), AppHandle (+9 more)

### Community 9 - "resume.rs"
Cohesion: 0.32
Nodes (12): clear_pending_marker(), clear_pending_marker_is_idempotent(), marker_path(), marker_pointing_at_missing_file_is_ignored(), marker_pointing_at_real_file_is_returned_and_consumed(), no_marker_returns_none(), Option, Path (+4 more)

### Community 10 - "BedrockInstanceService"
Cohesion: 0.06
Nodes (52): BedrockBackupService, BedrockRoot, Into, Option, Path, PathBuf, Result, Self (+44 more)

### Community 11 - "themes.rs"
Cohesion: 0.07
Nodes (48): css_vars_cover_every_variable_the_original_emits(), duplicated_gets_a_fresh_id(), exports_dir(), fmt3(), light_text_uses_subtler_item_overlays(), lighten(), new_id(), overlay_strength_scales_the_overlay_alphas() (+40 more)

### Community 12 - "releases.rs"
Cohesion: 0.05
Nodes (60): CachedResponse, Entry, expires_after_ttl(), GitHubApiCache, hit_after_store(), miss_when_empty(), HashMap, Instant (+52 more)

### Community 13 - "sync_packs.rs"
Cohesion: 0.37
Nodes (17): apply_sync_pack(), create_sync_pack(), delete_sync_pack(), detach_sync_pack(), get_sync_report(), instance_dir(), instance_dirs(), list_sync_packs() (+9 more)

### Community 14 - "gl-injection/src/lib.rs"
Cohesion: 0.06
Nodes (47): c_void, activate_application(), enable_debug_privilege(), find_process_by_name(), find_processes_by_name(), grant_uwp_access(), inject_dll(), inject_rejects_missing_dll_before_touching_win32() (+39 more)

### Community 15 - "versions.rs"
Cohesion: 0.20
Nodes (29): WuVersion, available_free_space(), delete_latite_version(), delete_vanilla_bedrock_version(), download_latite_version(), download_vanilla_bedrock_version(), ensure_disk_space(), installed_bedrock_version() (+21 more)

### Community 16 - "ImportCandidate"
Cohesion: 0.19
Nodes (14): atlauncher_adapter_parses_neoforge_instance(), curseforge_app_adapter_parses_forge_instance(), DetectedLauncher, ImportCandidate, mmc_adapter_parses_a_fabric_instance(), modrinth_app_adapter_parses_quilt_profile(), pipeline_imports_and_copies_files(), ModLoaderKind (+6 more)

### Community 18 - "worlds.rs"
Cohesion: 0.08
Nodes (66): copy_dir_recursive(), delete_pack(), dir_size(), import_a_loose_shaderpack_folder(), import_list_delete_a_zip_pack(), import_pack(), importing_a_missing_source_is_an_error(), list_packs() (+58 more)

### Community 19 - "tests.rs"
Cohesion: 0.25
Nodes (14): dir_link_apply_with_keep_existing_then_detach_materialize_round_trips_no_data_loss(), dir_link_conflict_requires_explicit_adopt_strategy(), file_copy_apply_and_detach_discard(), instance_dirs(), make_pack(), mods_target_reports_missing_and_present(), options_txt_merge_preserves_key_order_and_untouched_local_keys(), preview_never_touches_the_filesystem() (+6 more)

### Community 20 - "levilamina.rs"
Cohesion: 0.07
Nodes (42): detects_not_found_payload(), falls_back_to_first_zip_when_no_windows_asset(), GitHubAsset, GitHubRelease, mod_asset_prefers_zip_over_other_files(), parse_release(), parses_release_and_picks_windows_zip(), pick_mod_asset() (+34 more)

### Community 21 - "gl-settings/src/lib.rs"
Cohesion: 0.21
Nodes (16): cache_dir(), default_settings_round_trip_through_persistence(), JavaAccount, launcher_root(), LauncherSettings, legacy_settings_path(), load(), Default (+8 more)

### Community 22 - "mod_analyzer.rs"
Cohesion: 0.19
Nodes (26): analyze(), analyze_dir(), analyzes_fabric_mod_json(), analyzes_forge_mods_toml(), built_ins(), capture(), delete_mod(), is_disabled_jar() (+18 more)

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
Nodes (37): dependencies, skinview3d, @tauri-apps/api, @tauri-apps/plugin-dialog, @tauri-apps/plugin-log, devDependencies, esbuild, svelte (+29 more)

### Community 31 - "AppState"
Cohesion: 0.16
Nodes (45): JavaInstanceService, JavaVersionService, OderSoDllEntry, active_oderso_dll(), check_flarial_dll_up_to_date(), check_levilamina_up_to_date(), check_oderso_up_to_date(), delete_flarial_client() (+37 more)

### Community 32 - "mmc.rs"
Cohesion: 0.38
Nodes (11): MmcComponent, MmcPack, parse_instance_cfg_name(), read_instance(), Option, Path, Result, String (+3 more)

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
Cohesion: 0.16
Nodes (25): extract_appx(), fetch_updates(), find_text(), generic_version(), get_auth_cookie(), get_download_url(), is_dotted_number(), list_versions() (+17 more)

### Community 37 - "export_mrpack"
Cohesion: 0.09
Nodes (40): add_dir_to_zip(), add_file_to_zip(), export_curseforge(), loader_id(), Path, PathBuf, Result, SimpleFileOptions (+32 more)

### Community 38 - "logs.rs"
Cohesion: 0.14
Nodes (36): chrono_like_day(), civil_from_days(), CrashReportInfo, day_to_iso(), days_from_civil(), get_stats_summary(), latest_java_crash_report(), leaves_version_like_numbers_alone() (+28 more)

### Community 39 - "PARITY.md (parity ledger)"
Cohesion: 0.13
Nodes (25): Bedrock-first, then Java (order of work), BedrockContentView merged-panels defect (Worlds/Packs/Backups/Screenshots), BedrockTabs (bottom tab strip, Home.BigFeatures.cs:54-75), Missing com.mojang root resolution (highest-risk finding), Console window rendered main app (console.html missing bug), rust-tauri-cutover branch cutover commit, GdkArchiveService (Bedrock GDK version source), gl-injection crate (Win32 DLL injection port) (+17 more)

### Community 40 - "skin_library.rs"
Cohesion: 0.17
Nodes (25): add_from_file(), add_from_username(), base64_decode(), base64_roundtrip_matches_known_vector(), delete_skin(), list_skins(), MojangProfileByName, resolve_skin_texture() (+17 more)

### Community 41 - "src/bedrock.rs"
Cohesion: 0.20
Nodes (24): activate_via_com(), BedrockPackageInfo, connect_external_server(), escape_data_string(), extract_application_id(), find_game_process(), find_game_process_for(), installed_version() (+16 more)

### Community 42 - "VersionsView.svelte"
Cohesion: 0.10
Nodes (17): cancelFor(), currentSortIcon, displayName(), filteredOderSo, filteredVersions, handleOderSoDownload(), isPinned(), loadOderSoVersions() (+9 more)

### Community 43 - "mod_loader.rs"
Cohesion: 0.27
Nodes (27): download_forge_installer(), download_install_libraries(), download_neoforge_installer(), download_via_queue(), install_fabric(), install_forge(), install_from_installer_jar(), install_neoforge() (+19 more)

### Community 44 - "SyncReport"
Cohesion: 0.15
Nodes (14): apply(), detach(), Path, Result, preview(), Path, String, ReportAction (+6 more)

### Community 45 - "CurseForgeClient"
Cohesion: 0.16
Nodes (15): CurseForgeClient, html_decode(), parse_addon(), parse_file(), parses_search_addon_fixture(), Client, Into, Option (+7 more)

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
Cohesion: 0.09
Nodes (51): errors_when_manifest_missing(), loader_profile_id(), parse_curseforge_manifest(), parse_loader_id(), parse_modrinth_index(), parses_curseforge_manifest_fixture(), parses_modrinth_index_fixture_and_skips_unsupported_client_files(), read_zip_json() (+43 more)

### Community 50 - "commands/notifications.rs"
Cohesion: 0.29
Nodes (12): AppNotification, add_notification(), clear_notifications(), list_notifications(), mark_all_notifications_read(), mark_notification_read(), notification_unread_count(), remove_notification() (+4 more)

### Community 51 - "DownloadQueue"
Cohesion: 0.13
Nodes (16): clear_finished_also_drops_queued_entries(), DownloadQueue, new_entry_starts_queued_with_zero_progress(), queue_caps_concurrency_at_configured_limit(), remove_and_clear_notify_subscribers(), Arc, Duration, Mutex (+8 more)

### Community 52 - "paths.rs"
Cohesion: 0.22
Nodes (22): com_mojang_root(), counts_directories_as_entries(), empty_subfolder_never_wins(), falls_back_to_shared_when_no_account_has_the_subfolder(), falls_back_to_shared_when_users_root_is_absent(), live_dir_for(), live_root_for(), minecraft_pe_dir() (+14 more)

### Community 53 - "src/tweaks.rs"
Cohesion: 0.19
Nodes (22): BedrockManifest, BedrockManifestHeader, BedrockManifestModule, BedrockTweaksIndex, build_bedrock_manifest(), builds_manifest_for_single_pack(), flatten_vanilla(), flattens_vanilla_categories_recursively() (+14 more)

### Community 54 - "bedrock_addons.rs"
Cohesion: 0.25
Nodes (26): BedrockAddonInstallResult, check_bedrock_pack_conflicts(), copy_dir_recursive(), default_dir_name_for_class(), extract_to_temp(), find_pack_conflicts(), install_bedrock_addon(), install_into_class_dir() (+18 more)

### Community 55 - "SyncEngine"
Cohesion: 0.38
Nodes (7): HashMap, Path, PathBuf, Result, Self, String, SyncEngine

### Community 56 - "[]"
Cohesion: 0.08
Nodes (22): [], closeServerModal(), _confirmDeleteServerId, deleteServer(), getServerIconStyle(), isServerIconImage(), lanError, lanScanning (+14 more)

### Community 57 - ".install"
Cohesion: 0.19
Nodes (19): AssetJob, collect_libraries(), DownloadJob, extract_native_jar(), InstallProgress, InstallService, Client, Default (+11 more)

### Community 58 - "VersionService"
Cohesion: 0.21
Nodes (15): CachedVersion, dirs_home(), ManifestLatest, ManifestRoot, ManifestVersionEntry, Client, JavaVersion, Option (+7 more)

### Community 59 - "jar_inspect.rs"
Cohesion: 0.17
Nodes (22): analyze(), analyze_flags_duplicate_ids_and_missing_deps(), find_toml_string(), is_built_in(), parse_fabric(), parse_forge(), parses_fabric_mod_json_fixture(), parses_forge_mods_toml_fixture_and_extracts_required_deps() (+14 more)

### Community 60 - "ClientLibraryView.svelte"
Cohesion: 0.10
Nodes (14): app, _clError, _clFilter, clFiltered, _clLoading, _clProgress, _clVersion, isClientPackExpanded() (+6 more)

### Community 61 - "ModrinthClient"
Cohesion: 0.21
Nodes (13): html_decode(), ModrinthClient, parse_project(), parse_version(), parses_latest_version_and_prefers_primary_file(), parses_search_project_fixture(), Client, Option (+5 more)

### Community 62 - "gl-bedrock/src/lib.rs"
Cohesion: 0.07
Nodes (47): copy_dir_recursive(), extract_to_temp(), import_content(), import_packs(), import_world(), ImportOutcome, imports_mcaddon_bundle_routes_by_module_type(), imports_mcpack_by_resource_module_type() (+39 more)

### Community 63 - "gl-clients/src/models.rs"
Cohesion: 0.15
Nodes (11): ClientDownloadState, ClientInfo, ClientLibraryPack, ClientType, default_kind(), download_state_lifecycle(), GlacierLauncherMeta, GlacierManifest (+3 more)

### Community 64 - "gl-download/src/download.rs"
Cohesion: 0.24
Nodes (17): backoff_for(), commit(), download_file(), download_once(), download_with_retry(), DownloadOutcome, is_destination_busy(), CancellationToken (+9 more)

### Community 65 - "java_packs.rs"
Cohesion: 0.30
Nodes (22): delete_java_resourcepack(), delete_java_shaderpack(), get_mod_versions(), import_java_resourcepack(), import_java_shaderpack(), install_curseforge_mod(), install_planned_file(), install_resolved_mods() (+14 more)

### Community 66 - "gl-persistence/src/lib.rs"
Cohesion: 0.18
Nodes (18): append_extension(), Envelope, load(), no_leftover_temp_file_after_write(), PersistError, quarantine_corrupt(), round_trips_through_atomic_write_and_load(), Error (+10 more)

### Community 67 - "commands/bedrock.rs"
Cohesion: 0.06
Nodes (78): BedrockBackup, BedrockInstance, BedrockPack, BedrockWorld, OptionEntry, OptionKind, PackKind, Option (+70 more)

### Community 68 - "queue.rs"
Cohesion: 0.30
Nodes (9): DownloadEntry, DownloadHandle, DownloadStatus, RegistryEntry, CancellationToken, Option, Receiver, JoinHandle (+1 more)

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
Cohesion: 0.22
Nodes (35): check_java_mod_updates(), delete_java_mod(), diff_java_instances(), download_forge_installer(), download_neoforge_installer(), install_fabric_loader(), install_local_mod_jar(), install_modrinth_mod() (+27 more)

### Community 73 - "gl-modpack/src/models.rs"
Cohesion: 0.22
Nodes (17): CfAddon, CfSearchResult, CurseForgeFileRef, ModInfo, ModLoader, ModpackFile, ModpackPlan, ModReport (+9 more)

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
Cohesion: 0.18
Nodes (36): JvmPreset, create_java_instance(), create_java_instance_from_template(), delete_java_instance(), duplicate_java_instance(), export_java_instance(), get_effective_launch_config(), get_instance_settings() (+28 more)

### Community 79 - "system.rs"
Cohesion: 0.23
Nodes (16): close_launcher_window(), export_settings(), exports_dir(), get_launcher_root(), import_latest_settings(), open_folder(), open_path(), open_url() (+8 more)

### Community 80 - "gl-java/src/lib.rs"
Cohesion: 0.23
Nodes (16): read_version_profile(), build_classpath_excludes_natives_and_dedupes(), build_game_args_fullscreen_takes_priority_over_custom_resolution(), build_game_args_substitutes_and_appends_resolution_and_server(), build_jvm_args_clamps_ram_to_documented_bounds(), build_jvm_args_substitutes_and_filters_classpath_placeholder(), is_jvm_arg_compatible_filters_java23_only_flags(), read_version_profile_errors_when_json_missing() (+8 more)

### Community 81 - "src/glacier_client.rs"
Cohesion: 0.20
Nodes (11): fetch_manifest(), glacier_dir(), installed_size_bytes(), parse_manifest(), parses_manifest_camel_case_fields(), require_installed(), Client, Path (+3 more)

### Community 82 - "DiscordRpcService"
Cohesion: 0.23
Nodes (7): DiscordRpcService, now_unix(), Default, Drop, Option, Self, DiscordIpcClient

### Community 83 - "mod_provenance.rs"
Cohesion: 0.30
Nodes (18): corrupt_store_falls_back_to_empty_rather_than_panicking(), load(), missing_store_yields_empty_default(), ModProvenance, ModSource, ProvenanceStore, record(), record_load_remove_roundtrip() (+10 more)

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

### Community 88 - "io_err"
Cohesion: 0.25
Nodes (17): io_err(), Error, Path, PathBuf, String, SyncError, apply(), copy_dir_recursive() (+9 more)

### Community 89 - "SyncPack"
Cohesion: 0.20
Nodes (12): AdoptStrategy, MergeFormat, Option, String, Vec, SyncPack, SyncTarget, VersionOverride (+4 more)

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

### Community 97 - "SyncRegistry"
Cohesion: 0.23
Nodes (7): Mutex, Path, PathBuf, Result, Self, Vec, SyncRegistry

### Community 98 - "compilerOptions"
Cohesion: 0.13
Nodes (14): compilerOptions, allowArbitraryExtensions, allowJs, checkJs, module, moduleDetection, noEmit, target (+6 more)

### Community 99 - "glacier-launcher"
Cohesion: 0.17
Nodes (16): gl-auth, gl-bedrock, gl-clients, gl-download, gl-injection, gl-integrations, gl-java, gl-launch (+8 more)

### Community 100 - "effective.rs"
Cohesion: 0.27
Nodes (13): all_none_falls_back_to_defaults(), effective(), EffectiveLaunchConfig, explicit_memory_override_wins(), GlobalJavaDefaults, instance_with_settings(), java_path_override(), legacy_ram_fields_still_override_when_no_new_settings() (+5 more)

### Community 102 - "commands/glacier_client.rs"
Cohesion: 0.30
Nodes (12): get_glacier_manifest(), glacier_installed_size_bytes(), install_glacier_client(), launch_glacier_client(), AppHandle, Option, PathBuf, Result (+4 more)

### Community 103 - "OptionsDocument"
Cohesion: 0.22
Nodes (7): OptionsDocument, Option, Path, Result, Self, String, Vec

### Community 104 - "DownloadRequest"
Cohesion: 0.29
Nodes (8): DownloadRequest, has_real_hash(), Into, Option, PathBuf, Self, String, validate_rejects_empty_url()

### Community 105 - "atlauncher.rs"
Cohesion: 0.40
Nodes (9): AtInstance, AtLauncherBlock, AtLoaderVersion, read_instance(), Option, Path, String, Vec (+1 more)

### Community 106 - "settings.ts"
Cohesion: 0.12
Nodes (18): i(), canAdvance(), create(), ensureVersionInstalled(), loadVersions(), next(), toast(), BedrockInstance (+10 more)

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

### Community 111 - ".parse"
Cohesion: 0.21
Nodes (9): MavenCoordinate, parses_basic_coordinate(), parses_classifier_and_extension(), parses_extension_without_classifier(), Option, Result, Self, String (+1 more)

### Community 112 - "DownloadError"
Cohesion: 0.27
Nodes (7): DownloadError, Box, Error, Into, PathBuf, Self, String

### Community 113 - "modrinth_app.rs"
Cohesion: 0.40
Nodes (9): MrLoaderVersion, MrMetadata, MrProfile, read_profile(), Option, Path, String, Vec (+1 more)

### Community 114 - "UnifiedMod"
Cohesion: 0.35
Nodes (10): fetch_versions(), Provider, From, Option, Result, String, Vec, search_mods() (+2 more)

### Community 115 - "DownloadClient"
Cohesion: 0.31
Nodes (4): DownloadClient, Client, Result, Self

### Community 116 - "GlacierClientVersion"
Cohesion: 0.46
Nodes (3): GlacierClientVersion, Path, PathBuf

### Community 117 - "check"
Cohesion: 0.33
Nodes (9): check(), PrelaunchWarning, Into, JavaInstance, Option, Path, String, Vec (+1 more)

### Community 118 - "ModpacksView.svelte"
Cohesion: 0.32
Nodes (5): onSearchKey(), search(), setModpacksViewMode(), setSource(), saveViewMode()

### Community 119 - "package.json"
Cohesion: 0.25
Nodes (7): description, name, private, scripts, build, dev, version

### Community 120 - "presets.rs"
Cohesion: 0.38
Nodes (6): args_for(), find(), JvmPreset, Option, String, Vec

### Community 121 - "setup_tray"
Cohesion: 0.38
Nodes (6): apply_native_window_chrome(), AppHandle, Result, run(), setup_tray(), WebviewWindow

### Community 122 - "InstallProfile"
Cohesion: 0.15
Nodes (16): DataSides, DataValue, InstallLibrary, InstallProfile, LibraryArtifact, LibraryDownloads, parses_real_shape(), Processor (+8 more)

### Community 123 - "default.json"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 124 - "get_settings"
Cohesion: 0.60
Nodes (5): get_settings(), LauncherSettings, Result, String, set_settings()

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

### Community 135 - "collect_args"
Cohesion: 0.70
Nodes (5): collect_args(), features_match(), os_matches(), Value, rules_allow()

### Community 141 - "java_process.rs"
Cohesion: 0.07
Nodes (51): kill_errors_for_untracked_instance(), kill_process_tree(), mark_exited_removes_from_list_and_drops_buffer(), pid_of_reflects_registration_and_removal(), ProcessEntry, ProcessRegistry, push_log_line_is_noop_for_unknown_instance(), register_push_and_read_roundtrip() (+43 more)

### Community 142 - "resolve_modrinth_node"
Cohesion: 0.21
Nodes (17): InstallPlan, PlannedFile, resolve_curseforge_plan(), resolve_modrinth_node(), resolve_modrinth_node_inner(), resolve_modrinth_plan(), Box, HashMap (+9 more)

### Community 146 - "loader_map.rs"
Cohesion: 0.62
Nodes (6): from_atlauncher_loader_type(), from_curseforge_loader_name(), from_mmc_component_uid(), from_modrinth_loader(), ModLoaderKind, Option

### Community 149 - "apply"
Cohesion: 0.43
Nodes (6): apply(), detach(), Option, Path, Result, DetachMode

### Community 150 - "list_jar_names"
Cohesion: 0.60
Nodes (5): list_jar_names(), mod_filenames(), Path, String, Vec

### Community 151 - "collector"
Cohesion: 0.18
Nodes (18): collector(), enough_time_but_not_enough_delta_is_suppressed(), first_update_always_reported(), reports_once_interval_and_delta_both_satisfied(), Duration, FnMut, Instant, Option (+10 more)

### Community 152 - "processors.rs"
Cohesion: 0.20
Nodes (14): make_jar_with_manifest(), parse_main_class(), ProcessorContext, read_main_class(), reads_main_class_from_manifest(), resolve_token_extracts_installer_relative_data_file(), Option, Path (+6 more)

### Community 153 - "io_err"
Cohesion: 0.23
Nodes (10): io_err(), JavaError, Error, FnOnce, Into, PathBuf, String, ZipError (+2 more)

### Community 154 - "pipeline.rs"
Cohesion: 0.35
Nodes (11): copy_dir_recursive(), import_many(), import_one(), rollback(), JavaInstance, Path, PathBuf, Result (+3 more)

### Community 155 - "get_string"
Cohesion: 0.43
Nodes (6): download_to_file(), get_string(), Client, Path, Result, String

## Ambiguous Edges - Review These
- `inventory-shell.md` → `scratch.txt (per-file special-character scratch dump)`  [AMBIGUOUS]
  scratch.txt · relation: references
- `PARITY.md (parity ledger)` → `scratch.txt (per-file special-character scratch dump)`  [AMBIGUOUS]
  scratch.txt · relation: references
- `README.md (Glacier Launcher overview)` → `scratch.txt (per-file special-character scratch dump)`  [AMBIGUOUS]
  scratch.txt · relation: references

## Knowledge Gaps
- **172 isolated node(s):** `gl-auth`, `gl-bedrock`, `gl-clients`, `gl-integrations`, `gl-modpack` (+167 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 733 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **7 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **What is the exact relationship between `inventory-shell.md` and `scratch.txt (per-file special-character scratch dump)`?**
  _Edge tagged AMBIGUOUS (relation: references) - confidence is low._
- **What is the exact relationship between `PARITY.md (parity ledger)` and `scratch.txt (per-file special-character scratch dump)`?**
  _Edge tagged AMBIGUOUS (relation: references) - confidence is low._
- **What is the exact relationship between `README.md (Glacier Launcher overview)` and `scratch.txt (per-file special-character scratch dump)`?**
  _Edge tagged AMBIGUOUS (relation: references) - confidence is low._
- **Why does `AppState` connect `AppState` to `launch.rs`, `gl-auth/src/lib.rs`, `utilities.rs`, `java_loaders.rs`, `BedrockInstanceService`, `themes.rs`, `releases.rs`, `java_process.rs`, `sync_packs.rs`, `versions.rs`, `worlds.rs`, `.new`, `src/notifications.rs`, `logs.rs`, `modpack.rs`, `commands/notifications.rs`, `DownloadQueue`, `bedrock_addons.rs`, `SyncEngine`, `java_packs.rs`, `commands/bedrock.rs`, `launcher_import.rs`, `java_mods.rs`, `commands/java.rs`, `DiscordRpcService`, `downloads.rs`, `integrations.rs`, `commands/levilamina_mods.rs`, `commands/skins.rs`, `commands/tweaks.rs`, `commands/client_library.rs`, `commands/glacier_client.rs`?**
  _High betweenness centrality (0.442) - this node is a cross-community bridge._
- **Why does `BedrockInstanceService` connect `BedrockInstanceService` to `gl-bedrock/src/lib.rs`, `AppState`?**
  _High betweenness centrality (0.063) - this node is a cross-community bridge._
- **Why does `WuVersion` connect `versions.rs` to `wu.rs`, `parse_version_db`, `gdk.rs`?**
  _High betweenness centrality (0.060) - this node is a cross-community bridge._
- **Are the 4 inferred relationships involving `[]` (e.g. with `cancelDeleteInstance()` and `downloadPlayerSkin()`) actually correct?**
  _`[]` has 4 INFERRED edges - model-reasoned connections that need verification._