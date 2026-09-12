//! Windows Update SOAP protocol client used to enumerate and resolve
//! download URLs for installable vanilla Bedrock builds (the
//! `Microsoft.MinecraftUWP` / `Microsoft.MinecraftWindowsBeta` appx
//! packages), ported from the WU-facing half of the C#
//! `Services/VanillaVersionService.cs` (`GetAuthCookieAsync`,
//! `FetchUpdatesAsync`, `GetDownloadUrlAsync`, `ParseVersionFromPackage`).
//!
//! This talks to Microsoft's undocumented `client.asmx` SOAP endpoint the
//! same way the Windows Update client itself does: `GetCookie` to obtain an
//! anonymous auth cookie, `SyncUpdates` (scoped to the Bedrock WU category
//! id, pretending a fixed, well-known set of "already installed" parent
//! update IDs so the server returns only the leaf Minecraft updates) to
//! enumerate available builds, then `GetExtendedUpdateInfo2` per update to
//! resolve its actual `.appx` download URL. None of this requires
//! authentication beyond the anonymous cookie.
//!
//! Deviation from the original: XML is parsed with `roxmltree` instead of
//! `System.Xml.Linq.XDocument` — the only XML parser this workspace
//! depends on, pulled in specifically for this module.

use std::fs::File;
use std::io::Read as _;
use std::path::Path;
use std::process::Command as StdCommand;
#[cfg(windows)]
use std::os::windows::process::CommandExt as _;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

use roxmltree::Document;
use serde::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::error::{BedrockError, IoContext, Result};
use crate::util::{iso7_now, write_atomic};

const WU_ENDPOINT: &str = "https://fe3.delivery.mp.microsoft.com/ClientWebService/client.asmx";
const WU_ENDPOINT_SECURED: &str =
    "https://fe3.delivery.mp.microsoft.com/ClientWebService/client.asmx/secured";
const WU_CATEGORY_ID: &str = "d25480ca-36aa-46e6-b76b-39608d49558c";
const WUWS_NS: &str = "http://www.microsoft.com/SoftwareDistribution/Server/ClientWebService";

/// Device attributes string WU expects, describing a generic retail x64
/// Windows 10/11 client. Mirrors the C# constant of the same shape.
///
/// This is embedded directly into a hand-built XML request body below (the
/// C# original goes through `XElement`, which escapes automatically). Every
/// interpolation site must run this through [`xml_escape_amp`] first, since
/// the raw value contains bare `&` separators that are illegal inside XML text.
const DEVICE_ATTRIBUTES: &str = "E:BranchReadinessLevel=CBServicingBranch&AttrDataVer=264&FlightRing=Retail&FlightContent=Mainline&InstallLanguage=en-US&OSUILocale=en-US&InstallationType=Client&OSArchitecture=AMD64&OSVersion=10.0.22621.1&OSSkuId=48&App=WU&IsFlightingEnabled=1&IsDeviceRetailDemo=0";

/// Escapes `&` for embedding `DEVICE_ATTRIBUTES` in the hand-built XML
/// bodies below. Only `&` needs handling — the constant contains no `<`,
/// `>`, `"`, or `'`.
fn xml_escape_amp(s: &str) -> String {
    s.replace('&', "&amp;")
}

/// A parent/"non-leaf" update ID WU treats as already installed so that
/// `SyncUpdates` returns only the Minecraft leaf updates under it, instead
/// of the entire OS servicing tree. Lifted verbatim from the C# original —
/// these are stable, well-known Microsoft catalog IDs, not anything
/// per-machine.
const INSTALLED_NON_LEAF_UPDATE_IDS: &[u32] = &[
    1, 2, 3, 11, 19, 2359974, 5169044, 8788830, 23110993, 23110994, 54341900, 59830006, 59830007,
    59830008, 60484010, 62450018, 62450019, 62450020, 98959022, 98959023, 98959024, 98959025,
    98959026, 104433538, 129905029, 130040031, 132387090, 132393049, 133399034, 138537048,
    140377312, 143747671, 158941041, 158941042, 158941043, 158941044, 159123858, 159130928,
    164836897, 164847386, 164848327, 164852241, 164852246, 164852253,
];

/// One installable vanilla Bedrock build resolved from Windows Update.
/// Mirrors the WU-sourced fields of the C# `VanillaVersion` model.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WuVersion {
    pub version: String,
    pub update_id: String,
    pub package_name: String,
    pub size_bytes: u64,
    /// `"Release"` or `"Preview"`.
    pub channel: String,

    // --- Fields carried by entries from the community GDK archive.
    // All default, so a Windows Update entry (and any previously cached JSON)
    // deserializes unchanged.
    /// `"gdk"` for community-archive entries, empty for Windows Update ones.
    /// Port of `VanillaVersion.Source`; `IsGdk` in the C# is `Source == "gdk"`.
    #[serde(default)]
    pub source: String,
    /// The archive release tag the asset came from, shown as the row's label.
    #[serde(default)]
    pub friendly_version: String,
    /// Direct asset URL. Windows Update entries resolve theirs from
    /// `update_id` at download time and leave this empty.
    #[serde(default)]
    pub download_url: String,
    /// Published SHA-256 for the asset, verified on download. Empty when the
    /// archive published no usable digest.
    #[serde(default)]
    pub sha256: String,
}

struct UpdateInfo {
    update_id: String,
    package_name: String,
    size_bytes: u64,
}

fn security_header() -> String {
    let created = iso7_now();
    // `iso7_now` doesn't take an offset, so approximate "+5 minutes" via a
    // raw string swap is wrong; recompute using SystemTime directly.
    let expires = crate::util::iso7_from_systemtime(
        std::time::SystemTime::now() + std::time::Duration::from_secs(5 * 60),
    );

    format!(
        r#"<o:Security s:mustUnderstand="1"
                xmlns:o="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd">
            <Timestamp xmlns="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
                <Created>{created}</Created>
                <Expires>{expires}</Expires>
            </Timestamp>
            <wuws:WindowsUpdateTicketsToken wsu:id="ClientMSA"
                xmlns:wuws="http://schemas.microsoft.com/msus/2014/10/WindowsUpdateAuthorization"
                xmlns:wsu="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd">
                <TicketType Name="MSA" Version="1.0" Policy="MBI_SSL">
                    <User/>
                </TicketType>
            </wuws:WindowsUpdateTicketsToken>
        </o:Security>"#
    )
}

async fn post_soap(
    client: &reqwest::Client,
    action: &str,
    to: &str,
    body: &str,
    secured: bool,
) -> Result<String> {
    let endpoint = if secured {
        WU_ENDPOINT_SECURED
    } else {
        WU_ENDPOINT
    };

    let envelope = format!(
        r#"<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:a="http://www.w3.org/2005/08/addressing">
            <s:Header>
                <a:Action s:mustUnderstand="1">{action}</a:Action>
                <a:To s:mustUnderstand="1">{to}</a:To>
                {header}
            </s:Header>
            <s:Body>
                {body}
            </s:Body>
        </s:Envelope>"#,
        header = security_header()
    );

    let resp = client
        .post(endpoint)
        .header("Content-Type", "application/soap+xml; charset=utf-8")
        .body(envelope)
        .send()
        .await
        .map_err(|e| BedrockError::Wu(format!("request failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(BedrockError::Wu(format!(
            "Windows Update returned HTTP {}",
            resp.status()
        )));
    }

    resp.text()
        .await
        .map_err(|e| BedrockError::Wu(format!("failed to read response: {e}")))
}

fn find_text<'a>(doc: &'a Document, ns: &str, local: &str) -> Option<&'a str> {
    doc.descendants()
        .find(|n| n.has_tag_name((ns, local)))
        .and_then(|n| n.text())
}

/// `GetCookie` — obtains an anonymous auth cookie for subsequent calls.
async fn get_auth_cookie(client: &reqwest::Client) -> Result<String> {
    let body = format!(
        r#"<GetCookie xmlns="{WUWS_NS}">
            <oldCookie></oldCookie>
            <lastChange>2015-10-21T17:01:07.1472913Z</lastChange>
            <currentTime>{now}</currentTime>
            <protocolVersion>1.40</protocolVersion>
        </GetCookie>"#,
        now = iso7_now()
    );

    let text = post_soap(
        client,
        &format!("{WUWS_NS}/GetCookie"),
        WU_ENDPOINT,
        &body,
        false,
    )
    .await?;

    let doc = Document::parse(&text)
        .map_err(|e| BedrockError::Wu(format!("bad GetCookie response: {e}")))?;
    find_text(&doc, WUWS_NS, "EncryptedData")
        .map(|s| s.to_string())
        .ok_or_else(|| BedrockError::Wu("failed to get auth cookie from Microsoft Store".into()))
}

/// `SyncUpdates`, scoped to the Bedrock WU category — enumerates every
/// currently-offered Minecraft Bedrock appx update.
async fn fetch_updates(client: &reqwest::Client, cookie: &str) -> Result<Vec<UpdateInfo>> {
    let ids_xml: String = INSTALLED_NON_LEAF_UPDATE_IDS
        .iter()
        .map(|i| format!("<int>{i}</int>"))
        .collect::<Vec<_>>()
        .join("\n");

    let device_attributes = xml_escape_amp(DEVICE_ATTRIBUTES);
    let body = format!(
        r#"<SyncUpdates xmlns="{WUWS_NS}">
            <cookie>
                <Expiration>2036-01-01T00:00:00Z</Expiration>
                <EncryptedData>{cookie}</EncryptedData>
            </cookie>
            <parameters>
                <ExpressQuery>false</ExpressQuery>
                <InstalledNonLeafUpdateIDs>
                    {ids_xml}
                </InstalledNonLeafUpdateIDs>
                <OtherCachedUpdateIDs/>
                <SkipSoftwareSync>false</SkipSoftwareSync>
                <NeedTwoGroupOutOfScopeUpdates>true</NeedTwoGroupOutOfScopeUpdates>
                <FilterAppCategoryIds>
                    <CategoryIdentifier>
                        <Id>{WU_CATEGORY_ID}</Id>
                    </CategoryIdentifier>
                </FilterAppCategoryIds>
                <TreatAppCategoryIdsAsInstalled>true</TreatAppCategoryIdsAsInstalled>
                <AlsoPerformRegularSync>false</AlsoPerformRegularSync>
                <ComputerSpec/>
                <ExtendedUpdateInfoParameters>
                    <XmlUpdateFragmentTypes>
                        <XmlUpdateFragmentType>Extended</XmlUpdateFragmentType>
                        <XmlUpdateFragmentType>LocalizedProperties</XmlUpdateFragmentType>
                    </XmlUpdateFragmentTypes>
                    <Locales><string>en-US</string></Locales>
                </ExtendedUpdateInfoParameters>
                <ClientPreferredLanguages/>
                <ProductsParameters>
                    <SyncCurrentVersionOnly>false</SyncCurrentVersionOnly>
                    <DeviceAttributes>{device_attributes}</DeviceAttributes>
                    <CallerAttributes>Interactive=1;IsSeeker=1;</CallerAttributes>
                    <Products/>
                </ProductsParameters>
            </parameters>
        </SyncUpdates>"#
    );

    let text = post_soap(
        client,
        &format!("{WUWS_NS}/SyncUpdates"),
        WU_ENDPOINT,
        &body,
        false,
    )
    .await?;

    let doc = Document::parse(&text)
        .map_err(|e| BedrockError::Wu(format!("bad SyncUpdates response: {e}")))?;

    // Each <UpdateInfo><ID>.../<Xml>...fragment...</Xml></UpdateInfo> holds
    // an escaped XML fragment describing one candidate update. Collect the
    // (server ID, fragment text) pairs first, deduping by ID like the C#
    // `idMap` dictionary does.
    let mut id_map: Vec<(String, String)> = Vec::new();
    for upd in doc
        .descendants()
        .filter(|n| n.has_tag_name((WUWS_NS, "UpdateInfo")))
    {
        let id = upd
            .descendants()
            .find(|n| n.has_tag_name((WUWS_NS, "ID")))
            .and_then(|n| n.text());
        let frag = upd
            .descendants()
            .find(|n| n.has_tag_name((WUWS_NS, "Xml")))
            .and_then(|n| n.text());

        if let (Some(id), Some(frag)) = (id, frag) {
            // Matches C#'s `idMap[id] = frag` on a `Dictionary<string, string>`:
            // a repeated ID overwrites the fragment (last-wins) while keeping
            // the key's original iteration position, rather than the first
            // occurrence winning.
            match id_map.iter_mut().find(|(existing, _)| existing == id) {
                Some((_, existing_frag)) => *existing_frag = frag.to_string(),
                None => id_map.push((id.to_string(), frag.to_string())),
            }
        }
    }

    let mut results = Vec::new();
    for (id, fragment_xml) in id_map {
        let has_uwp = fragment_xml.contains("Microsoft.MinecraftUWP");
        let has_beta = fragment_xml.contains("Microsoft.MinecraftWindowsBeta");
        if !has_uwp && !has_beta {
            continue;
        }
        if fragment_xml.contains(".EAppx") {
            continue;
        }
        let lower = fragment_xml.to_ascii_lowercase();
        if lower.contains("arm") && !lower.contains("x64") {
            continue;
        }

        let wrapped = format!("<r>{fragment_xml}</r>");
        let Ok(frag_doc) = Document::parse(&wrapped) else {
            continue;
        };

        // Matches `VanillaVersionService`'s `AppxPackageInstallData`
        // .Descendants("AppxMetadata")` - `AppxMetadata` must be a *child* of
        // `AppxPackageInstallData`, not merely present anywhere in the
        // fragment (a plain `.descendants()` search over-matches when a
        // fragment nests an unrelated `AppxMetadata` element elsewhere).
        let Some(package_moniker) = frag_doc
            .descendants()
            .find(|n| n.has_tag_name("AppxPackageInstallData"))
            .and_then(|n| n.children().find(|c| c.has_tag_name("AppxMetadata")))
            .and_then(|n| n.attribute("PackageMoniker"))
        else {
            continue;
        };
        if !package_moniker.to_ascii_lowercase().contains("x64") {
            continue;
        }

        let size_bytes = frag_doc
            .descendants()
            .find(|n| n.has_tag_name("File"))
            .and_then(|n| n.attribute("Size"))
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let update_id = frag_doc
            .descendants()
            .find(|n| n.has_tag_name("UpdateIdentity"))
            .and_then(|n| n.attribute("UpdateID"))
            .map(|s| s.to_string())
            .unwrap_or_else(|| id.clone());

        results.push(UpdateInfo {
            update_id,
            package_name: package_moniker.to_string(),
            size_bytes,
        });
    }

    Ok(results)
}

/// `GetExtendedUpdateInfo2` — resolves the actual `.appx` download URL for
/// a single update ID.
pub async fn get_download_url(client: &reqwest::Client, update_id: &str) -> Result<Option<String>> {
    let device_attributes = xml_escape_amp(DEVICE_ATTRIBUTES);
    let body = format!(
        r#"<GetExtendedUpdateInfo2 xmlns="{WUWS_NS}">
            <updateIDs>
                <UpdateIdentity>
                    <UpdateID>{update_id}</UpdateID>
                    <RevisionNumber>1</RevisionNumber>
                </UpdateIdentity>
            </updateIDs>
            <infoTypes>
                <XmlUpdateFragmentType>FileUrl</XmlUpdateFragmentType>
                <XmlUpdateFragmentType>FileDecryption</XmlUpdateFragmentType>
            </infoTypes>
            <deviceAttributes>{device_attributes}</deviceAttributes>
        </GetExtendedUpdateInfo2>"#
    );

    let text = post_soap(
        client,
        &format!("{WUWS_NS}/GetExtendedUpdateInfo2"),
        WU_ENDPOINT_SECURED,
        &body,
        true,
    )
    .await?;

    let doc = Document::parse(&text)
        .map_err(|e| BedrockError::Wu(format!("bad GetExtendedUpdateInfo2 response: {e}")))?;

    let url = doc
        .descendants()
        .filter(|n| n.has_tag_name((WUWS_NS, "Url")))
        .filter_map(|n| n.text())
        .find(|u| u.contains(".appx") || u.to_ascii_lowercase().contains("microsoft"))
        .map(|s| s.to_string());

    Ok(url)
}

pub fn detect_channel(package_name: &str) -> &'static str {
    if package_name.to_ascii_lowercase().contains("windowsbeta") {
        "Preview"
    } else {
        "Release"
    }
}

/// Extracts a dotted version number from an appx package moniker, mirroring
/// `VanillaVersionService.ParseVersionFromPackage`. Two strategies, tried in
/// order:
/// 1. A named `_v1.2.3`/`-v1.2` scheme, bounded by `_`/`-`/end-of-string.
/// 2. The first bare `\d+\.\d+\.\d+(\.\d+)?` run anywhere in the string.
pub fn parse_version_from_package(package_name: &str) -> Option<String> {
    named_scheme_version(package_name).or_else(|| generic_version(package_name))
}

fn named_scheme_version(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let is_v_marker = (bytes[i] == b'_' || bytes[i] == b'-')
            && (bytes[i + 1] == b'v' || bytes[i + 1] == b'V');
        if is_v_marker {
            let start = i + 2;
            let mut j = start;
            while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b'.') {
                j += 1;
            }
            let candidate = &s[start..j];
            if is_dotted_number(candidate, 2, 3, 3) {
                let boundary_ok = j == bytes.len() || bytes[j] == b'_' || bytes[j] == b'-';
                if boundary_ok {
                    return Some(candidate.to_string());
                }
            }
        }
        i += 1;
    }
    None
}

fn generic_version(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        if bytes[i].is_ascii_digit() {
            let start = i;
            let mut j = i;
            while j < n && (bytes[j].is_ascii_digit() || bytes[j] == b'.') {
                j += 1;
            }
            let mut end = j;
            while end > start && bytes[end - 1] == b'.' {
                end -= 1;
            }
            let candidate = &s[start..end];
            if is_dotted_number(candidate, 3, 4, usize::MAX) {
                return Some(candidate.to_string());
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    None
}

/// Whether `s` is `min_parts..=max_parts` dot-separated all-digit groups,
/// each at most `max_group_len` digits long (`usize::MAX` = unbounded).
fn is_dotted_number(s: &str, min_parts: usize, max_parts: usize, max_group_len: usize) -> bool {
    if s.is_empty() {
        return false;
    }
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() < min_parts || parts.len() > max_parts {
        return false;
    }
    parts
        .iter()
        .all(|p| !p.is_empty() && p.len() <= max_group_len && p.bytes().all(|c| c.is_ascii_digit()))
}

/// Lists installable vanilla Bedrock builds currently offered by Windows
/// Update. Mirrors `VanillaVersionService.FetchFromStoreApiAsync`.
pub async fn list_versions(client: &reqwest::Client) -> Result<Vec<WuVersion>> {
    let cookie = get_auth_cookie(client).await?;
    let updates = fetch_updates(client, &cookie).await?;

    let mut versions: Vec<WuVersion> = updates
        .into_iter()
        .filter_map(|u| {
            let version = parse_version_from_package(&u.package_name)?;
            Some(WuVersion {
                channel: detect_channel(&u.package_name).to_string(),
                version,
                update_id: u.update_id,
                package_name: u.package_name,
                size_bytes: u.size_bytes,
                // Windows Update entries carry none of the archive fields;
                // the download path resolves their URL from `update_id`.
                ..Default::default()
            })
        })
        .collect();

    versions.sort_by_key(|v| std::cmp::Reverse(version_key(&v.version)));
    Ok(versions)
}

/// Parses a dotted version string into a comparable tuple, newest-first
/// sort key. Mirrors `VanillaVersionService.TryParseVersion` + the
/// `OrderByDescending` it feeds.
pub fn version_key(v: &str) -> (u32, u32, u32, u32) {
    let mut parts = v.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

/// Extracts a downloaded vanilla Bedrock `.appx`/`.msix` into `dest_dir`,
/// mirroring `VanillaVersionService.ExtractPackage`: finds the
/// `AppxManifest.xml` entry, strips its containing directory as a prefix
/// from every other entry, skips the package signature and content-types
/// manifest, and guards against zip-slip path traversal.
pub fn extract_appx(zip_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = File::open(zip_path).ctx(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut manifest_full_name: Option<String> = None;
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let is_manifest = Path::new(entry.name())
            .file_name()
            .and_then(|f| f.to_str())
            .map(|f| f.eq_ignore_ascii_case("AppxManifest.xml"))
            .unwrap_or(false);
        if is_manifest {
            manifest_full_name = Some(entry.name().to_string());
            break;
        }
    }

    let manifest_full_name = manifest_full_name.ok_or_else(|| {
        BedrockError::Wu(
            "This archive has no AppxManifest.xml — it isn't a registrable Minecraft package."
                .into(),
        )
    })?;

    let file_name_len = Path::new(&manifest_full_name)
        .file_name()
        .map(|f| f.to_string_lossy().len())
        .unwrap_or(0);
    let prefix = manifest_full_name[..manifest_full_name.len() - file_name_len].to_string();
    let prefix_lower = prefix.to_ascii_lowercase();

    std::fs::create_dir_all(dest_dir).ctx(dest_dir)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name.ends_with('/') {
            continue;
        }
        if Path::new(&name).file_name().is_none() {
            continue;
        }

        let relative = if !prefix.is_empty() {
            if !name.to_ascii_lowercase().starts_with(&prefix_lower) {
                continue;
            }
            &name[prefix.len()..]
        } else {
            name.as_str()
        };

        let relative_lower = relative.to_ascii_lowercase();
        if relative_lower.starts_with("appxsignature")
            || relative_lower.starts_with("[content_types]")
        {
            continue;
        }

        let dest_path = dest_dir.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        if !dest_path.starts_with(dest_dir) {
            continue; // zip-slip guard
        }

        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent).ctx(parent)?;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).ctx(&dest_path)?;
        write_atomic(&dest_path, &bytes)?;
    }

    Ok(())
}

fn run_powershell(script: &str) -> Result<String> {
    let mut ps_cmd = StdCommand::new("powershell.exe");
    ps_cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    #[cfg(windows)]
    ps_cmd.creation_flags(CREATE_NO_WINDOW);
    let output = ps_cmd
        .output()
        .map_err(|e| BedrockError::Wu(format!("failed to run powershell.exe: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BedrockError::Wu(stderr.trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Unregisters the currently-installed vanilla Bedrock package (release and
/// preview), mirroring `VanillaVersionService.UnregisterCurrentAsync`. A
/// "no such package" failure is treated as success — there's nothing to
/// unregister.
pub fn unregister_current() -> Result<()> {
    match run_powershell(
        "Get-AppxPackage -Name Microsoft.MinecraftUWP,Microsoft.MinecraftWindowsBeta | Remove-AppxPackage",
    ) {
        Ok(_) => Ok(()),
        Err(BedrockError::Wu(msg))
            if msg.to_ascii_lowercase().contains("not found")
                || msg.to_ascii_lowercase().contains("no package") =>
        {
            Ok(())
        }
        Err(e) => Err(BedrockError::Wu(format!("Failed to unregister current version: {e}"))),
    }
}

/// Registers a downloaded, extracted vanilla Bedrock build in developer
/// mode, mirroring `VanillaVersionService.RegisterVersionAsync`. Requires
/// Windows Developer Mode to be enabled (see `gl_launch::bedrock::resolve`'s
/// sibling checks elsewhere in this crate for the release/preview package
/// family names this then becomes resolvable under).
pub fn register_version(manifest_path: &Path) -> Result<()> {
    let escaped = manifest_path.to_string_lossy().replace('\'', "''");
    run_powershell(&format!(
        "Add-AppxPackage -Register '{escaped}' -DevelopmentMode"
    ))
    .map(|_| ())
    .map_err(|e| BedrockError::Wu(format!("Failed to register version: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_ampersands_for_xml_embedding() {
        assert_eq!(
            xml_escape_amp("A=1&B=2&C=3"),
            "A=1&amp;B=2&amp;C=3"
        );
    }

    #[test]
    fn parses_generic_dotted_version_from_package_moniker() {
        assert_eq!(
            parse_version_from_package("Microsoft.MinecraftUWP_1.21.44.0_x64__8wekyb3d8bbwe"),
            Some("1.21.44.0".to_string())
        );
    }

    #[test]
    fn parses_named_v_scheme() {
        assert_eq!(
            parse_version_from_package("SomePackage_v1.21.44_x64__8wekyb3d8bbwe"),
            Some("1.21.44".to_string())
        );
    }

    #[test]
    fn detects_preview_channel() {
        assert_eq!(
            detect_channel("Microsoft.MinecraftWindowsBeta_1.21.44.0_x64__8wekyb3d8bbwe"),
            "Preview"
        );
        assert_eq!(
            detect_channel("Microsoft.MinecraftUWP_1.21.44.0_x64__8wekyb3d8bbwe"),
            "Release"
        );
    }

    #[test]
    fn sorts_versions_newest_first() {
        assert!(version_key("1.21.44.0") > version_key("1.20.0.0"));
        assert!(version_key("1.21.44.2") > version_key("1.21.44.1"));
    }
}
