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

    private static readonly string ComMojangRoot = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
        "Minecraft Bedrock", "Users", "Shared", "games", "com.mojang");

    public static string ResourcePacksDir => Path.Combine(ComMojangRoot, "resource_packs");
    public static string BehaviorPacksDir => Path.Combine(ComMojangRoot, "behaviour_packs");
    public static string SkinPacksDir     => Path.Combine(ComMojangRoot, "skin_packs");
    public static string WorldsDir        => Path.Combine(ComMojangRoot, "minecraftWorlds");

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

    private readonly HttpClient _http;
    private readonly SettingsService _settings;

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

    // ── Download & install ────────────────────────────────────

    public async Task DownloadAndInstallAsync(CfFile file, int classId, IProgress<double>? progress = null)
    {
        var tmpPath = Path.Combine(Path.GetTempPath(), file.FileName);
        try
        {
            using var req  = BuildRequest(HttpMethod.Get, file.DownloadUrl);
            using var resp = await _http.SendAsync(req, HttpCompletionOption.ResponseHeadersRead);
            resp.EnsureSuccessStatusCode();

            var total = resp.Content.Headers.ContentLength ?? file.FileLength;
            using var src = await resp.Content.ReadAsStreamAsync();
            using var dest = File.Create(tmpPath);

            var buf = new byte[81920];
            long downloaded = 0;
            int read;
            while ((read = await src.ReadAsync(buf)) > 0)
            {
                await dest.WriteAsync(buf.AsMemory(0, read));
                downloaded += read;
                if (total > 0) progress?.Report(downloaded * 100.0 / total);
            }
            dest.Close();

            await InstallAddonFileAsync(tmpPath, classId);
        }
        finally
        {
            try { if (File.Exists(tmpPath)) File.Delete(tmpPath); } catch { }
        }
    }

    // ── Installation logic ────────────────────────────────────

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
        var name     = item.GetProperty("name").GetString() ?? "";
        var summary  = item.TryGetProperty("summary", out var s) ? s.GetString() ?? "" : "";
        var dlCount  = item.TryGetProperty("downloadCount", out var dc) ? (long)dc.GetDouble() : 0;
        var classId  = item.TryGetProperty("classId", out var ci) ? ci.GetInt32() : 0;

        var thumb = "";
        if (item.TryGetProperty("logo", out var logo) && logo.ValueKind != JsonValueKind.Null)
            thumb = logo.TryGetProperty("thumbnailUrl", out var tu) ? tu.GetString() ?? "" : "";

        var author = "";
        if (item.TryGetProperty("authors", out var authors) && authors.GetArrayLength() > 0)
            author = authors[0].GetProperty("name").GetString() ?? "";

        var catName = "";
        if (item.TryGetProperty("categories", out var cats) && cats.GetArrayLength() > 0)
            catName = cats[0].TryGetProperty("name", out var cn) ? cn.GetString() ?? "" : "";

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