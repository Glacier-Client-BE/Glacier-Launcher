//! Optional AI features, gated entirely behind a user-supplied API key
//! (`LauncherSettings.gemini_api_key`, Settings → System → AI). Glacier never
//! ships its own key — every request here runs with the caller's own key,
//! straight from their machine to Google, so there's nothing embedded in the
//! distributed app for anyone to extract.

use serde::Deserialize;
use tauri::State;

use crate::state::AppState;

const GEMINI_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-flash-latest:generateContent";

/// Truncated server-side too — a full multi-thousand-line crash log is mostly
/// noise for this purpose, and staying well under Gemini's input limits
/// avoids a needless 400 for an oversized log.
const MAX_LOG_CHARS: usize = 12_000;

#[derive(Deserialize)]
struct GenerateContentResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: ContentBody,
}

#[derive(Deserialize)]
struct ContentBody {
    parts: Vec<Part>,
}

#[derive(Deserialize)]
struct Part {
    text: String,
}

/// Sends a Minecraft crash log (or any log Logs panel flagged as a crash) to
/// Gemini and asks for a short, plain-English explanation: likely cause and
/// a concrete next step. Mirrors nothing in the original C# app — this is a
/// new feature, not a port.
#[tauri::command]
pub async fn explain_crash_log(
    state: State<'_, AppState>,
    api_key: String,
    log_text: String,
) -> Result<String, String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(
            "No Gemini API key set — add your own free key in Settings to use this.".to_string(),
        );
    }

    let truncated = if log_text.len() > MAX_LOG_CHARS {
        // Keep the tail: a crash's actual exception/cause is almost always
        // at the bottom of the file, not the top.
        let start = log_text.len() - MAX_LOG_CHARS;
        format!("[...truncated...]\n{}", &log_text[start..])
    } else {
        log_text
    };

    let prompt = format!(
        "You are helping a Minecraft player understand a crash log from their launcher. \
         Explain in plain English, in under 150 words: (1) what most likely caused this crash, \
         and (2) one concrete thing they can try to fix it. Don't repeat the raw log back. \
         If the log doesn't look like an actual crash/error, say so plainly instead of guessing.\n\n\
         --- LOG ---\n{truncated}"
    );

    let body = serde_json::json!({
        "contents": [{
            "parts": [{ "text": prompt }]
        }]
    });

    let resp = state
        .http
        .post(GEMINI_URL)
        .header("Content-Type", "application/json")
        .header("X-goog-api-key", api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Gemini request failed ({status}): {text}"));
    }

    let parsed: GenerateContentResponse = resp.json().await.map_err(|e| e.to_string())?;
    parsed
        .candidates
        .into_iter()
        .next()
        .and_then(|c| c.content.parts.into_iter().next())
        .map(|p| p.text)
        .ok_or_else(|| "Gemini returned no explanation.".to_string())
}
