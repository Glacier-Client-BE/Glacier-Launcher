mod commands;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::settings::get_settings,
            commands::settings::set_settings,
            commands::java::list_java_versions,
            commands::java::list_java_instances,
            commands::java::create_java_instance,
            commands::java::rename_java_instance,
            commands::java::delete_java_instance,
            commands::java::duplicate_java_instance,
            commands::java::import_java_instance_zip,
            commands::java::import_official_minecraft,
            commands::java::set_java_instance_ram,
            commands::java::install_java_version,
            commands::java::list_jvm_presets,
            commands::java::create_java_instance_from_template,
            commands::java::set_instance_jvm_args,
            commands::java::set_instance_jvm_preset,
            commands::java::set_instance_group,
            commands::java::get_instance_settings,
            commands::java::set_instance_settings,
            commands::java::reset_instance_setting,
            commands::java::set_instance_loader,
            commands::java::set_instance_account,
            commands::java::get_effective_launch_config,
            commands::java::export_java_instance,
            commands::java::set_java_instance_accent_color,
            commands::launch::java_prelaunch_check,
            commands::java_worlds::list_java_worlds,
            commands::java_worlds::list_java_backups,
            commands::java_worlds::backup_java_world,
            commands::java_worlds::backup_active_java_world,
            commands::java_worlds::restore_java_backup,
            commands::java_worlds::delete_java_backup,
            commands::java_worlds::list_java_datapacks,
            commands::java_worlds::import_java_datapack,
            commands::java_worlds::delete_java_datapack,
            commands::java_worlds::open_java_datapacks_folder,
            commands::bedrock::list_bedrock_instances,
            commands::bedrock::create_bedrock_instance,
            commands::bedrock::rename_bedrock_instance,
            commands::bedrock::delete_bedrock_instance,
            commands::bedrock::set_bedrock_instance_accent_color,
            commands::bedrock::switch_bedrock_instance,
            commands::bedrock::save_active_bedrock_instance,
            commands::bedrock::set_bedrock_instance_version,
            commands::bedrock::active_bedrock_instance_id,
            commands::notifications::list_notifications,
            commands::notifications::notification_unread_count,
            commands::notifications::add_notification,
            commands::notifications::mark_notification_read,
            commands::notifications::mark_all_notifications_read,
            commands::notifications::remove_notification,
            commands::notifications::clear_notifications,
            commands::theme::list_themes,
            commands::theme::get_theme,
            commands::theme::create_theme,
            commands::theme::update_theme,
            commands::theme::duplicate_theme,
            commands::theme::delete_theme,
            commands::theme::export_theme,
            commands::theme::import_theme,
            commands::theme::stash_theme_background,
            commands::theme::theme_css_vars,
            commands::theme::theme_presets,
            commands::bedrock::list_bedrock_worlds,
            commands::bedrock::delete_bedrock_world,
            commands::bedrock::export_bedrock_world,
            commands::bedrock::bedrock_worlds_dir,
            commands::bedrock::bedrock_backups_dir,
            commands::bedrock::bedrock_screenshots_dir,
            commands::bedrock::list_bedrock_packs,
            commands::bedrock::delete_bedrock_pack,
            commands::bedrock::list_bedrock_backups,
            commands::bedrock::create_bedrock_backup,
            commands::bedrock::restore_bedrock_backup,
            commands::bedrock::delete_bedrock_backup,
            commands::bedrock::list_bedrock_screenshots,
            commands::bedrock::delete_bedrock_screenshot,
            commands::bedrock::import_bedrock_content,
            commands::bedrock::resolve_com_mojang_folder,
            commands::bedrock::bedrock_auto_backup_interval,
            commands::bedrock::set_bedrock_auto_backup_interval,
            commands::bedrock::bedrock_marketplace_reminder_dismissed,
            commands::bedrock::dismiss_bedrock_marketplace_reminder,
            commands::bedrock_addons::search_bedrock_addons,
            commands::bedrock_addons::install_bedrock_addon,
            commands::bedrock_addons::check_bedrock_pack_conflicts,
            commands::launch::launch_java_instance,
            commands::launch::launch_java_version,
            commands::launch::launch_bedrock_instance,
            commands::launch::launch_bedrock_server,
            commands::launch::is_bedrock_game_running,
            commands::launch::is_java_game_running,
            commands::launch::cancel_launch,
            commands::launch::save_console_log,
            commands::launch::upload_console_log_to_mclogs,
            commands::launch::console_stop_game,
            commands::launch::console_content_title,
            commands::java_mods::list_java_mods,
            commands::java_mods::set_java_mod_enabled,
            commands::java_mods::delete_java_mod,
            commands::java_mods::open_java_mods_folder,
            commands::java_mods::install_fabric_loader,
            commands::java_mods::install_quilt_loader,
            commands::java_mods::download_forge_installer,
            commands::java_mods::download_neoforge_installer,
            commands::java_mods::search_modrinth_projects,
            commands::java_mods::install_modrinth_mod,
            commands::java_mods::check_java_mod_updates,
            commands::java_mods::update_java_mod,
            commands::java_mods::update_all_java_mods,
            commands::java_mods::set_all_java_mods_enabled,
            commands::java_mods::diff_java_instances,
            commands::java_mods::install_local_mod_jar,
            commands::java_loaders::list_loader_versions,
            commands::java_loaders::install_loader,
            commands::java_loaders::uninstall_loader,
            commands::java_loaders::get_instance_loader,
            commands::java_packs::list_java_resourcepacks,
            commands::java_packs::import_java_resourcepack,
            commands::java_packs::delete_java_resourcepack,
            commands::java_packs::list_java_shaderpacks,
            commands::java_packs::import_java_shaderpack,
            commands::java_packs::delete_java_shaderpack,
            commands::java_packs::search_mods,
            commands::java_packs::resolve_mod_dependencies,
            commands::java_packs::install_resolved_mods,
            commands::java_packs::install_curseforge_mod,
            commands::java_packs::get_mod_versions,
            commands::auth::sign_in,
            commands::auth::sign_in_with_refresh_token,
            commands::injection::inject_bedrock_client,
            commands::injection::is_bedrock_running,
            commands::modpack::search_curseforge_modpacks,
            commands::modpack::search_modrinth_modpacks,
            commands::modpack::install_curseforge_modpack,
            commands::modpack::install_modrinth_modpack,
            commands::modpack::check_modpack_updates,
            commands::modpack::update_modpack,
            commands::modpack::read_pack_manifest_ids,
            commands::clients::fetch_levilamina_release,
            commands::clients::check_flarial_dll_up_to_date,
            commands::clients::path_exists,
            commands::clients::download_flarial_client,
            commands::clients::flarial_client_path,
            commands::clients::flarial_client_installed,
            commands::clients::oderso_client_path,
            commands::clients::levilamina_client_path,
            commands::clients::delete_flarial_client,
            commands::clients::list_oderso_dlls,
            commands::clients::oderso_client_installed,
            commands::clients::download_oderso_client,
            commands::clients::delete_oderso_client,
            commands::clients::active_oderso_dll,
            commands::clients::oderso_dll_installed,
            commands::clients::download_oderso_dll,
            commands::clients::set_active_oderso_dll,
            commands::clients::delete_oderso_dll,
            commands::clients::check_oderso_up_to_date,
            commands::clients::levilamina_client_installed,
            commands::clients::download_levilamina_client,
            commands::clients::delete_levilamina_client,
            commands::clients::check_levilamina_up_to_date,
            commands::glacier_client::get_glacier_manifest,
            commands::glacier_client::install_glacier_client,
            commands::glacier_client::uninstall_glacier_client,
            commands::glacier_client::glacier_installed_size_bytes,
            commands::glacier_client::launch_glacier_client,
            commands::integrations::get_announcement,
            commands::integrations::get_news,
            commands::integrations::start_discord_presence,
            commands::integrations::stop_discord_presence,
            commands::integrations::set_discord_in_game_presence,
            commands::integrations::set_discord_idle_presence,
            commands::integrations::login_discord,
            commands::integrations::logout_discord,
            commands::update::check_launcher_update,
            commands::update::launcher_version,
            commands::update::list_launcher_releases,
            commands::update::list_client_releases,
            commands::update::list_glacier_client_releases,
            commands::update::download_launcher_update,
            commands::update::install_launcher_update,
            commands::shaders::shaders_unavailable_reason,
            commands::shaders::list_applied_shaders,
            commands::shaders::apply_shaders,
            commands::shaders::restore_shaders,
            commands::shaders::list_stock_shader_materials,
            commands::shaders::open_shaders_materials_folder,
            commands::versions::list_latite_versions,
            commands::versions::latite_version_installed,
            commands::versions::download_latite_version,
            commands::versions::delete_latite_version,
            commands::versions::latite_version_dll_path,
            commands::versions::list_vanilla_bedrock_versions,
            commands::versions::vanilla_bedrock_version_installed,
            commands::versions::installed_bedrock_version,
            commands::versions::download_vanilla_bedrock_version,
            commands::versions::delete_vanilla_bedrock_version,
            commands::versions::switch_vanilla_bedrock_version,
            commands::tweaks::tweaks_java_versions,
            commands::tweaks::list_vanilla_tweaks,
            commands::tweaks::build_vanilla_tweaks_pack,
            commands::tweaks::list_bedrock_tweaks,
            commands::tweaks::build_bedrock_tweaks_pack,
            commands::tweaks::fetch_image_bytes,
            commands::servers::list_saved_servers,
            commands::servers::add_saved_server,
            commands::servers::update_saved_server,
            commands::servers::remove_saved_server,
            commands::servers::ping_server,
            commands::servers::ping_java_server,
            commands::servers::scan_lan_games,
            commands::leveldat::read_level_dat_summary,
            commands::leveldat::write_level_dat_summary,
            commands::logs::list_logs,
            commands::logs::read_log,
            commands::logs::latest_java_crash_report,
            commands::logs::log_looks_like_crash,
            commands::logs::share_log,
            commands::logs::record_play_session,
            commands::logs::get_stats_summary,
            commands::java_process::list_running_instances,
            commands::java_process::kill_instance,
            commands::java_process::get_instance_log_buffer,
            commands::java_process::analyze_crash,
            commands::levilamina_mods::fetch_levilamina_mods,
            commands::levilamina_mods::levilamina_mod_installed,
            commands::levilamina_mods::download_levilamina_mod,
            commands::levilamina_mods::delete_levilamina_mod,
            commands::levilamina_mods::open_levilamina_mods_folder,
            commands::java_screenshots::list_java_screenshots,
            commands::java_screenshots::delete_java_screenshot,
            commands::java_screenshots::open_java_screenshots_folder,
            commands::launchers::detect_lunar_badlion,
            commands::launchers::launch_client_exe,
            commands::launchers::resolve_lunar_direct_launch,
            commands::launchers::launch_lunar_direct,
            commands::system::get_launcher_root,
            commands::system::close_launcher_window,
            commands::system::open_folder,
            commands::system::open_path,
            commands::system::open_url,
            commands::skins::list_saved_skins,
            commands::skins::add_skin_from_file,
            commands::skins::add_skin_from_username,
            commands::skins::delete_saved_skin,
            commands::skins::apply_skin,
            commands::skins::reset_skin,
            commands::skins::get_capes,
            commands::skins::set_cape,
            commands::skins::hide_cape,
            commands::system::export_settings,
            commands::system::import_latest_settings,
            commands::system::reset_settings,
            commands::utilities::get_bedrock_options,
            commands::utilities::bedrock_options_exist,
            commands::utilities::bedrock_options_has_backup,
            commands::utilities::save_bedrock_options,
            commands::utilities::restore_bedrock_options_backup,
            commands::utilities::get_system_snapshot,
            commands::utilities::get_live_sample,
            commands::utilities::list_custom_clients,
            commands::utilities::add_custom_client_from_file,
            commands::utilities::add_custom_client_from_url,
            commands::utilities::rename_custom_client,
            commands::utilities::toggle_custom_client_auto_update,
            commands::utilities::delete_custom_client,
            commands::utilities::open_custom_client_library_folder,
            commands::client_library::client_library_site_url,
            commands::client_library::get_client_library_packs,
            commands::client_library::client_library_versions,
            commands::client_library::is_client_library_download_installed,
            commands::client_library::install_client_library_download,
            commands::client_library::uninstall_client_library_download,
            commands::downloads::list_downloads,
            commands::downloads::cancel_download,
            commands::downloads::remove_download,
            commands::downloads::clear_finished_downloads,
            commands::update::resume_pending_launcher_update,
            commands::launcher_import::detect_external_launchers,
            commands::launcher_import::scan_launcher_instances,
            commands::launcher_import::preview_import,
            commands::launcher_import::import_instances,
            commands::sync_packs::list_sync_packs,
            commands::sync_packs::create_sync_pack,
            commands::sync_packs::update_sync_pack,
            commands::sync_packs::delete_sync_pack,
            commands::sync_packs::preview_sync,
            commands::sync_packs::apply_sync_pack,
            commands::sync_packs::detach_sync_pack,
            commands::sync_packs::get_sync_report,
        ])
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let app_state = state::AppState::new().map_err(|e| -> Box<dyn std::error::Error> {
                format!("failed to initialize app state: {e}").into()
            })?;
            commands::downloads::spawn_downloads_bridge(
                app.handle().clone(),
                app_state.downloads.clone(),
            );
            app.manage(app_state);

            // Mirrors `AutoUpdateService.TryResumePendingUpdate` being called
            // at startup before any UI is shown: if a prior
            // `install_launcher_update` call downloaded the installer but
            // failed to spawn it, retry here. On success this exits the app
            // immediately (the installer takes over), so nothing after this
            // point runs in that case.
            let _ = commands::update::resume_pending_launcher_update(app.handle().clone());

            if let Some(window) = app.get_webview_window("main") {
                apply_native_window_chrome(&window);

                // `RememberWindowSize`: restore the last persisted size
                // before first paint, and persist future resizes back to
                // settings. `MinimizeToTray`: hide instead of closing on the
                // window's close button when enabled.
                if let Ok(settings) = gl_settings::load() {
                    if settings.remember_window_size
                        && settings.window_width > 0.0
                        && settings.window_height > 0.0
                    {
                        let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
                            width: settings.window_width,
                            height: settings.window_height,
                        }));
                    }
                }
                let tray_close_window = window.clone();
                window.on_window_event(move |event| match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        if let Ok(settings) = gl_settings::load() {
                            if settings.minimize_to_tray {
                                api.prevent_close();
                                let _ = tray_close_window.hide();
                            }
                        }
                    }
                    tauri::WindowEvent::Resized(size) => {
                        if let Ok(mut settings) = gl_settings::load() {
                            if settings.remember_window_size {
                                let scale = tray_close_window.scale_factor().unwrap_or(1.0);
                                let logical = size.to_logical::<f64>(scale);
                                settings.window_width = logical.width;
                                settings.window_height = logical.height;
                                let _ = gl_settings::save(&settings);
                            }
                        }
                    }
                    _ => {}
                });

                // Window is created `visible: false` so chrome (rounded
                // corners, dark titlebar) is applied before first paint,
                // avoiding a flash of default Win32 window styling.
                let _ = window.show();
            }

            setup_tray(app.handle())?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Applies rounded corners, dark titlebar, and resize-border hit-testing to
/// the main window via `gl-win32chrome`, matching the original WPF window's
/// `SourceInitialized` handler. Best-effort: a failure here (e.g. running on
/// a pre-Windows-11 build without corner-rounding support) shouldn't stop
/// the app from starting.
#[cfg(windows)]
fn apply_native_window_chrome(window: &tauri::WebviewWindow) {
    let Ok(hwnd) = window.hwnd() else {
        log::warn!("could not get native HWND for window chrome setup");
        return;
    };

    if let Err(e) = gl_win32chrome::apply_chrome(hwnd) {
        log::warn!("failed to apply native window chrome: {e}");
    }
    if let Err(e) = gl_win32chrome::install_resize_hittest(hwnd) {
        log::warn!("failed to install resize hit-testing: {e}");
    }
}

#[cfg(not(windows))]
fn apply_native_window_chrome(_window: &tauri::WebviewWindow) {}

fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let show = MenuItem::with_id(app, "show", "Show Glacier Launcher", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().unwrap())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}
