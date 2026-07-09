using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Net.Http;
using System.Reflection;
using System.Text.Json;
using System.Threading.Tasks;

namespace GlacierLauncher.Services;

public class CurseForgeService
{
    private const string BaseUrl = "https://api.curseforge.com";

    // CurseForge game IDs — 78022 is the bespoke "Minecraft Bedrock" game; 432
    // is the original "Minecraft" (Java Edition).
    private const int GameIdBedrock = 78022;
    private const int GameIdJava    = 432;

    // Bedrock class ids (Minecraft Bedrock game).
    private const int BedrockClassAddons        = 4984;
    private const int BedrockClassMaps          = 6913;
    private const int BedrockClassSkins         = 6925;
    private const int BedrockClassTexturePacks  = 6929;
    private const int BedrockClassScripts       = 6940;

    // Java class ids (Minecraft Java game).
    private const int JavaClassMods             = 6;
    private const int JavaClassModpacks         = 4471;
    private const int JavaClassResourcePacks    = 12;
    private const int JavaClassWorlds           = 17;
    private const int JavaClassShaderPacks      = 6552;

    // The "Users" folder under "Minecraft Bedrock" contains a "Shared" folder
    // plus one folder per signed-in Xbox account (numeric id). Actual world/pack
    // data lives under whichever one the game is actually using, which is not
    // always "Shared" — so pick the folder that actually has content, and only
    // fall back to "Shared" if nothing else does.
    public static readonly string ComMojangRoot = ResolveComMojangRoot();

    private static string ResolveComMojangRoot()
    {
        var usersRoot = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            "Minecraft Bedrock", "Users");

        var sharedRoot = Path.Combine(usersRoot, "Shared", "games", "com.mojang");

        try
        {
            if (Directory.Exists(usersRoot))
            {
                string? best = null;
                var bestWorldCount = 0;
                foreach (var userDir in Directory.EnumerateDirectories(usersRoot))
                {
                    var candidate = Path.Combine(userDir, "games", "com.mojang");
                    var worldsDir = Path.Combine(candidate, "minecraftWorlds");
                    if (!Directory.Exists(worldsDir)) continue;

                    var worldCount = Directory.EnumerateDirectories(worldsDir).Take(1).Count() > 0
                        ? Directory.EnumerateDirectories(worldsDir).Count()
                        : 0;
                    if (worldCount > bestWorldCount)
                    {
                        bestWorldCount = worldCount;
                        best = candidate;
                    }
                }

                if (best != null) return best;
            }
        }
        catch { }

        return sharedRoot;
    }

    public static string ResourcePacksDir  => Path.Combine(ComMojangRoot, "resource_packs");
    public static string BehaviorPacksDir  => Path.Combine(ComMojangRoot, "behavior_packs");
    public static string SkinPacksDir      => Path.Combine(ComMojangRoot, "skin_packs");
    public static string WorldsDir         => Path.Combine(ComMojangRoot, "minecraftWorlds");
    public static string WorldTemplatesDir => Path.Combine(ComMojangRoot, "world_templates");

    // Unpacked/dev-mode packs — created when "Content Log" / dev tools are enabled in
    // Minecraft, or dropped in manually for local development (e.g. LeviLamina work).
    public static string DevResourcePacksDir => Path.Combine(ComMojangRoot, "development_resource_packs");
    public static string DevBehaviorPacksDir => Path.Combine(ComMojangRoot, "development_behavior_packs");
    public static string DevSkinPacksDir     => Path.Combine(ComMojangRoot, "development_skin_packs");

    // Java paths resolve against the JavaVersionService's MinecraftDir at
    // install time — we don't capture them statically because the user can
    // point the launcher at a non-default .minecraft folder.
    private readonly JavaVersionService? _javaVersions;

    // UPDATED: Added .Trim() and .Trim('"') to handle quoted strings from MSBuild/GitHub Actions
    private static readonly string BuiltInApiKey =
        (typeof(CurseForgeService).Assembly
            .GetCustomAttributes<AssemblyMetadataAttribute>()
            .FirstOrDefault(a => a.Key == "CurseForgeApiKey")?.Value ?? "")
            .Trim()
            .Trim('"');

    private readonly HttpClient      _http;
    private readonly SettingsService _settings;
    private readonly DownloadService _download = new();

    public CurseForgeService(SettingsService settings, JavaVersionService javaVersions)
    {
        _settings     = settings;
        _javaVersions = javaVersions;
        _http         = HttpFactory.Shared;
    }

    /// <summary>Active edition routed off the launcher's edition toggle.</summary>
    private bool IsJava => string.Equals(_settings.Settings.Edition, "java", StringComparison.OrdinalIgnoreCase);

    /// <summary>Categories the UI knows about for the active edition.</summary>
    public IReadOnlyList<(string Key, string Label, string Icon)> AvailableCategories => IsJava
        ? new (string, string, string)[]
        {
            ("all",          "All",            "fa-solid fa-layer-group"),
            ("mods",         "Mods",           "fa-solid fa-puzzle-piece"),
            ("modpacks",     "Modpacks",       "fa-solid fa-cubes"),
            ("texturepacks", "Resource Packs", "fa-solid fa-palette"),
            ("shaders",      "Shaders",        "fa-solid fa-droplet"),
            ("worlds",       "Worlds",         "fa-solid fa-globe"),
        }
        : new (string, string, string)[]
        {
            ("all",          "All",          "fa-solid fa-layer-group"),
            ("texturepacks", "Texture Packs","fa-solid fa-palette"),
            ("worlds",       "Worlds",       "fa-solid fa-globe"),
            ("addons",       "Add-ons",      "fa-solid fa-puzzle-piece"),
            ("skins",        "Skins",        "fa-solid fa-user"),
            ("scripts",      "Scripts",      "fa-solid fa-code"),
        };

    private string EffectiveApiKey =>
        !string.IsNullOrWhiteSpace(_settings.Settings.CurseForgeApiKey)
            ? _settings.Settings.CurseForgeApiKey.Trim().Trim('"')
            : BuiltInApiKey;

    public bool IsAvailable => !string.IsNullOrWhiteSpace(EffectiveApiKey);

    /// <summary>Builds a request with the API key + Accept header per-call (shared client safe).</summary>
    private HttpRequestMessage BuildRequest(HttpMethod method, string url)
    {
        var key = EffectiveApiKey;
        if (string.IsNullOrWhiteSpace(key))
            throw new InvalidOperationException("CurseForge API key is not set. Add it in Settings.");
        var req = new HttpRequestMessage(method, url);
        req.Headers.TryAddWithoutValidation("x-api-key", key);
        req.Headers.TryAddWithoutValidation("Accept", "application/json");
        return req;
    }

    private async Task<string> GetStringWithKeyAsync(string url)
    {
        using var req = BuildRequest(HttpMethod.Get, url);
        using var resp = await _http.SendAsync(req);
        resp.EnsureSuccessStatusCode();
        return await resp.Content.ReadAsStringAsync();
    }

    // ── Models ────────────────────────────────────────────────

    public record CfAddon(
        int    Id,
        string Name,
        string Summary,
        string ThumbnailUrl,
        string AuthorName,
        long   DownloadCount,
        int    ClassId,
        int    LatestFileId,
        string CategoryName);

    public record CfFile(
        int    FileId,
        string FileName,
        string DownloadUrl,
        long   FileLength,
        string DisplayName);

    public record CfSearchResult(List<CfAddon> Addons, int TotalCount);

    // ── Search ────────────────────────────────────────────────

    public async Task<CfSearchResult> SearchAsync(string query, string category = "all", int pageSize = 20, int page = 0)
    {
        var gameId = IsJava ? GameIdJava : GameIdBedrock;
        var classFilter = (IsJava, category) switch
        {
            (true,  "mods")         => $"&classId={JavaClassMods}",
            (true,  "modpacks")     => $"&classId={JavaClassModpacks}",
            (true,  "texturepacks") => $"&classId={JavaClassResourcePacks}",
            (true,  "shaders")      => $"&classId={JavaClassShaderPacks}",
            (true,  "worlds")       => $"&classId={JavaClassWorlds}",

            (false, "texturepacks") => $"&classId={BedrockClassTexturePacks}",
            (false, "worlds")       => $"&classId={BedrockClassMaps}",
            (false, "addons")       => $"&classId={BedrockClassAddons}",
            (false, "skins")        => $"&classId={BedrockClassSkins}",
            (false, "scripts")      => $"&classId={BedrockClassScripts}",
            _                       => ""
        };

        var searchFilter = string.IsNullOrWhiteSpace(query) ? "" : $"&searchFilter={Uri.EscapeDataString(query)}";
        var url = $"{BaseUrl}/v1/mods/search?gameId={gameId}{classFilter}{searchFilter}" +
                  $"&pageSize={pageSize}&index={page * pageSize}&sortField=2&sortOrder=desc";

        var json = await GetStringWithKeyAsync(url);
        using var doc = JsonDocument.Parse(json);

        var addons = new List<CfAddon>();
        if (doc.RootElement.TryGetProperty("data", out var data))
        {
            foreach (var item in data.EnumerateArray())
            {
                addons.Add(ParseAddon(item));
            }
        }

        var total = 0;
        if (doc.RootElement.TryGetProperty("pagination", out var pag) &&
            pag.TryGetProperty("totalCount", out var tc))
            total = tc.GetInt32();

        return new CfSearchResult(addons, total);
    }

    // ── Get files for a mod ───────────────────────────────────

    public async Task<List<CfFile>> GetFilesAsync(int modId)
    {
        var url = $"{BaseUrl}/v1/mods/{modId}/files?pageSize=10";
        var json = await GetStringWithKeyAsync(url);
        using var doc = JsonDocument.Parse(json);

        var files = new List<CfFile>();
        if (doc.RootElement.TryGetProperty("data", out var data))
        {
            foreach (var item in data.EnumerateArray())
            {
                var fileId     = item.GetProperty("id").GetInt32();
                var fileName   = item.GetProperty("fileName").GetString() ?? "";
                var dlUrl      = item.TryGetProperty("downloadUrl", out var du) ? du.GetString() ?? "" : "";
                var length     = item.TryGetProperty("fileLength", out var fl) ? fl.GetInt64() : 0;
                var dispName   = item.TryGetProperty("displayName", out var dn) ? dn.GetString() ?? fileName : fileName;

                if (!string.IsNullOrEmpty(dlUrl))
                    files.Add(new CfFile(fileId, fileName, dlUrl, length, dispName));
            }
        }
        return files;
    }

    // ── Modpack resolution (used by ModpackInstallService) ────

    public bool IsModpackClass(int classId) => classId == JavaClassModpacks;

    /// <summary>Downloads an arbitrary CurseForge file to <paramref name="destPath"/> with the API key attached.</summary>
    public Task DownloadFileToAsync(CfFile file, string destPath, IProgress<double>? progress = null, System.Threading.CancellationToken cancel = default) =>
        _download.DownloadAsync(
            file.DownloadUrl, destPath,
            progress: progress,
            knownTotalBytes: file.FileLength,
            configureRequest: req => req.Headers.TryAddWithoutValidation("x-api-key", EffectiveApiKey),
            cancel: cancel);

    /// <summary>
    /// Resolves a single modpack manifest entry (projectID + fileID) to its
    /// download details. Returns null when CurseForge disallows third-party
    /// downloads for that file (no downloadUrl) — the caller collects those as
    /// manual links.
    /// </summary>
    public async Task<(CfFile? File, string ProjectName)> ResolveManifestFileAsync(int projectId, int fileId)
    {
        var url = $"{BaseUrl}/v1/mods/{projectId}/files/{fileId}";
        var json = await GetStringWithKeyAsync(url);
        using var doc = JsonDocument.Parse(json);
        if (!doc.RootElement.TryGetProperty("data", out var item))
            return (null, $"project {projectId}");

        var fileName = item.TryGetProperty("fileName", out var fn) ? fn.GetString() ?? "" : "";
        var length   = item.TryGetProperty("fileLength", out var fl) ? fl.GetInt64() : 0;
        var dlUrl    = item.TryGetProperty("downloadUrl", out var du) && du.ValueKind != JsonValueKind.Null
                       ? du.GetString() ?? "" : "";

        if (string.IsNullOrEmpty(dlUrl))
        {
            // Reconstruct the CDN URL CurseForge uses when downloadUrl is null but
            // the file is actually distributable (common for older files).
            if (fileId > 0 && !string.IsNullOrEmpty(fileName))
            {
                var idStr = fileId.ToString();
                var a = idStr.Length > 4 ? idStr[..(idStr.Length - 3)] : idStr;
                var b = idStr.Length > 4 ? idStr[(idStr.Length - 3)..].TrimStart('0') : "0";
                if (string.IsNullOrEmpty(b)) b = "0";
                dlUrl = $"https://edge.forgecdn.net/files/{a}/{b}/{Uri.EscapeDataString(fileName)}";
            }
        }

        if (string.IsNullOrEmpty(dlUrl))
            return (null, fileName.Length > 0 ? fileName : $"project {projectId}");

        return (new CfFile(fileId, fileName, dlUrl, length, fileName), fileName);
    }

    // ── Download & install ────────────────────────────────────

    public async Task DownloadAndInstallAsync(CfFile file, int classId, IProgress<double>? progress = null)
    {
        var tmpPath = Path.Combine(Path.GetTempPath(), file.FileName);
        try
        {
            await _download.DownloadAsync(
                file.DownloadUrl, tmpPath,
                progress: progress,
                knownTotalBytes: file.FileLength,
                configureRequest: req => req.Headers.TryAddWithoutValidation("x-api-key", EffectiveApiKey));

            await InstallAddonFileAsync(tmpPath, classId);
        }
        finally
        {
            try { if (File.Exists(tmpPath)) File.Delete(tmpPath); } catch { }
        }
    }

    // ── Installation logic ────────────────────────────────────

    /// <summary>
    /// Installs a locally-supplied Bedrock pack/world file — used by drag-and-drop
    /// import. Detects type from extension (falling back to manifest sniffing for
    /// .mcpack) rather than requiring a CurseForge class id.
    /// </summary>
    public async Task<string> ImportBedrockPackFileAsync(string filePath)
    {
        var ext = Path.GetExtension(filePath).ToLowerInvariant();
        switch (ext)
        {
            case ".mcworld":
                await ExtractZipAsync(filePath, WorldsDir, Path.GetFileNameWithoutExtension(filePath));
                return "World";

            case ".mctemplate":
                await ExtractZipAsync(filePath, WorldTemplatesDir, Path.GetFileNameWithoutExtension(filePath));
                return "World template";

            case ".mcpack":
                await InstallMcPackAsync(filePath);
                return "Pack";

            case ".mcaddon":
                await InstallMcAddonAsync(filePath);
                return "Add-on";

            default:
                throw new NotSupportedException($"'{ext}' isn't a recognized Bedrock pack format.");
        }
    }

    private async Task InstallAddonFileAsync(string filePath, int classId)
    {
        var ext = Path.GetExtension(filePath).ToLowerInvariant();

        if (IsJava)
        {
            await InstallJavaAddonAsync(filePath, classId, ext);
            return;
        }

        switch (ext)
        {
            case ".mcworld":
                await ExtractZipAsync(filePath, WorldsDir, Path.GetFileNameWithoutExtension(filePath));
                break;

            case ".mcpack":
                await InstallMcPackAsync(filePath);
                break;

            case ".mcaddon":
                await InstallMcAddonAsync(filePath);
                break;

            case ".zip":
                var dir = GetDirectoryForClass(classId);
                await ExtractZipAsync(filePath, dir, Path.GetFileNameWithoutExtension(filePath));
                break;

            default:
                var fallbackDir = GetDirectoryForClass(classId);
                await ExtractZipAsync(filePath, fallbackDir, Path.GetFileNameWithoutExtension(filePath));
                break;
        }
    }

    // ── Java install paths ─────────────────────────────────────

    private string JavaMinecraftDir =>
        _javaVersions?.MinecraftDir ?? Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            ".minecraft");

    private string JavaModsDir          => Path.Combine(JavaMinecraftDir, "mods");
    private string JavaResourcePacksDir => Path.Combine(JavaMinecraftDir, "resourcepacks");
    private string JavaShaderPacksDir   => Path.Combine(JavaMinecraftDir, "shaderpacks");
    private string JavaSavesDir         => Path.Combine(JavaMinecraftDir, "saves");

    private async Task InstallJavaAddonAsync(string filePath, int classId, string ext)
    {
        switch (classId)
        {
            case JavaClassMods:
                // .jar files drop straight into the mods folder; that's the entire
                // Forge/Fabric install contract.
                Directory.CreateDirectory(JavaModsDir);
                File.Copy(filePath, Path.Combine(JavaModsDir, Path.GetFileName(filePath)), overwrite: true);
                break;

            case JavaClassResourcePacks:
                // Resource packs are zips that go in resourcepacks/ verbatim — DON'T
                // extract them; the game expects the zip itself.
                Directory.CreateDirectory(JavaResourcePacksDir);
                File.Copy(filePath, Path.Combine(JavaResourcePacksDir, SanitizeFileName(Path.GetFileName(filePath))), overwrite: true);
                break;

            case JavaClassShaderPacks:
                Directory.CreateDirectory(JavaShaderPacksDir);
                File.Copy(filePath, Path.Combine(JavaShaderPacksDir, SanitizeFileName(Path.GetFileName(filePath))), overwrite: true);
                break;

            case JavaClassWorlds:
                // Worlds are zipped folders — extract into saves/.
                Directory.CreateDirectory(JavaSavesDir);
                await ExtractZipAsync(filePath, JavaSavesDir, Path.GetFileNameWithoutExtension(filePath));
                break;

            case JavaClassModpacks:
                // CurseForge modpacks ship as a .zip with manifest.json + overrides/.
                // A real installer would resolve every mod listed in manifest.json from
                // CurseForge and download it; that's a substantial project on its own.
                // For now we just copy the pack to a "modpacks" folder and surface a
                // toast so the user knows to import it via a modpack manager.
                var modpacksDir = Path.Combine(JavaMinecraftDir, "modpacks");
                Directory.CreateDirectory(modpacksDir);
                File.Copy(filePath, Path.Combine(modpacksDir, Path.GetFileName(filePath)), overwrite: true);
                throw new InvalidOperationException(
                    "Modpack saved to .minecraft\\modpacks. Import it via your modpack manager (e.g. Prism, ATLauncher) — Glacier's modpack installer lands later.");

            default:
                // Unknown class — drop next to the mods folder so it's not lost.
                Directory.CreateDirectory(JavaModsDir);
                File.Copy(filePath, Path.Combine(JavaModsDir, Path.GetFileName(filePath)), overwrite: true);
                break;
        }
    }

    private static string SanitizeFileName(string name)
    {
        foreach (var c in Path.GetInvalidFileNameChars())
            name = name.Replace(c, '_');
        return name;
    }

    private async Task InstallMcPackAsync(string filePath)
    {
        var targetDir = await DetectPackTypeAsync(filePath);
        var folderName = Path.GetFileNameWithoutExtension(filePath);
        await ExtractZipAsync(filePath, targetDir, folderName);
    }

    private async Task InstallMcAddonAsync(string filePath)
    {
        var tmpExtract = Path.Combine(Path.GetTempPath(), "glacier_mcaddon_" + Guid.NewGuid().ToString("N"));
        try
        {
            Directory.CreateDirectory(tmpExtract);
            ZipFile.ExtractToDirectory(filePath, tmpExtract, true);

            foreach (var innerFile in Directory.GetFiles(tmpExtract, "*.*", SearchOption.AllDirectories))
            {
                var innerExt = Path.GetExtension(innerFile).ToLowerInvariant();
                if (innerExt is ".mcpack")
                    await InstallMcPackAsync(innerFile);
                else if (innerExt is ".mcworld")
                    await ExtractZipAsync(innerFile, WorldsDir, Path.GetFileNameWithoutExtension(innerFile));
            }
        }
        finally
        {
            try { Directory.Delete(tmpExtract, true); } catch { }
        }
    }

    private static async Task<string> DetectPackTypeAsync(string zipPath)
    {
        return await Task.Run(() =>
        {
            try
            {
                using var zip = ZipFile.OpenRead(zipPath);
                var manifest = zip.Entries.FirstOrDefault(e =>
                    e.Name.Equals("manifest.json", StringComparison.OrdinalIgnoreCase));

                if (manifest != null)
                {
                    using var stream = manifest.Open();
                    using var reader = new StreamReader(stream);
                    var json = reader.ReadToEnd();
                    var lower = json.ToLowerInvariant();

                    if (lower.Contains("\"skin_pack\"") || lower.Contains("\"skin\""))
                    {
                        Directory.CreateDirectory(SkinPacksDir);
                        return SkinPacksDir;
                    }
                    if (lower.Contains("\"data\"") || lower.Contains("\"script\""))
                    {
                        Directory.CreateDirectory(BehaviorPacksDir);
                        return BehaviorPacksDir;
                    }
                    if (lower.Contains("\"world_template\""))
                    {
                        Directory.CreateDirectory(WorldsDir);
                        return WorldsDir;
                    }
                }
            }
            catch { }

            Directory.CreateDirectory(ResourcePacksDir);
            return ResourcePacksDir;
        });
    }

    private static Task ExtractZipAsync(string zipPath, string parentDir, string folderName)
    {
        return Task.Run(() =>
        {
            Directory.CreateDirectory(parentDir);
            var destDir = Path.Combine(parentDir, SanitizeFolderName(folderName));

            if (Directory.Exists(destDir))
                Directory.Delete(destDir, true);

            Directory.CreateDirectory(destDir);
            ZipFile.ExtractToDirectory(zipPath, destDir, true);
        });
    }

    // ── Helpers ───────────────────────────────────────────────

    private static string GetDirectoryForClass(int classId)
    {
        var dir = classId switch
        {
            BedrockClassMaps         => WorldsDir,
            BedrockClassTexturePacks => ResourcePacksDir,
            BedrockClassSkins        => SkinPacksDir,
            BedrockClassAddons       => ResourcePacksDir,
            BedrockClassScripts      => BehaviorPacksDir,
            _                        => ResourcePacksDir
        };
        Directory.CreateDirectory(dir);
        return dir;
    }

    private static CfAddon ParseAddon(JsonElement item)
    {
        var id       = item.GetProperty("id").GetInt32();
        // CurseForge delivers rich text HTML-escaped (e.g. "Blocks &amp; Items"); decode
        // once here so the plain-@ bindings in the UI don't show literal entities.
        var name     = System.Net.WebUtility.HtmlDecode(item.GetProperty("name").GetString() ?? "");
        var summary  = System.Net.WebUtility.HtmlDecode(item.TryGetProperty("summary", out var s) ? s.GetString() ?? "" : "");
        var dlCount  = item.TryGetProperty("downloadCount", out var dc) ? (long)dc.GetDouble() : 0;
        var classId  = item.TryGetProperty("classId", out var ci) ? ci.GetInt32() : 0;

        var thumb = "";
        if (item.TryGetProperty("logo", out var logo) && logo.ValueKind != JsonValueKind.Null)
            thumb = logo.TryGetProperty("thumbnailUrl", out var tu) ? tu.GetString() ?? "" : "";

        var author = "";
        if (item.TryGetProperty("authors", out var authors) && authors.GetArrayLength() > 0)
            author = System.Net.WebUtility.HtmlDecode(authors[0].GetProperty("name").GetString() ?? "");

        var catName = "";
        if (item.TryGetProperty("categories", out var cats) && cats.GetArrayLength() > 0)
            catName = System.Net.WebUtility.HtmlDecode(cats[0].TryGetProperty("name", out var cn) ? cn.GetString() ?? "" : "");

        var latestFileId = 0;
        if (item.TryGetProperty("latestFilesIndexes", out var lfi) && lfi.GetArrayLength() > 0)
            latestFileId = lfi[0].TryGetProperty("fileId", out var fid) ? fid.GetInt32() : 0;
        if (latestFileId == 0 && item.TryGetProperty("mainFileId", out var mfid))
            latestFileId = mfid.GetInt32();

        return new CfAddon(id, name, summary, thumb, author, dlCount, classId, latestFileId, catName);
    }

    private static string SanitizeFolderName(string name)
    {
        foreach (var c in Path.GetInvalidFileNameChars())
            name = name.Replace(c, '_');
        return name.Trim();
    }
}