use std::path::PathBuf;

/// Injects `dll_path` into the running Bedrock (`Minecraft.Windows.exe`)
/// process. Returns an error if the game isn't running or if any step of
/// the injection sequence fails — see `gl_injection::InjectionError` for the
/// specific failure reasons surfaced to the frontend.
#[tauri::command]
pub fn inject_bedrock_client(dll_path: String, verify: bool) -> Result<(), String> {
    let pid = gl_injection::find_process_by_name("Minecraft.Windows.exe")
        .ok_or_else(|| "Minecraft isn't running.".to_string())?;
    gl_injection::inject_dll(pid, &PathBuf::from(dll_path), verify).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn is_bedrock_running() -> bool {
    gl_injection::find_process_by_name("Minecraft.Windows.exe").is_some()
}
