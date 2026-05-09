using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using System.Xml.Linq;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

public class VanillaVersionService
{
    private const string WuCategoryId = "d25480ca-36aa-46e6-b76b-39608d49558c";
    private const string McProductId  = "9NBLGGH2JHXJ";

    private static readonly XNamespace Soap = "http://www.w3.org/2003/05/soap-envelope";
    private static readonly XNamespace Wuws = "http://www.microsoft.com/SoftwareDistribution/Server/ClientWebService";

    private readonly SettingsService _settingsService;
    private readonly HttpClient      _httpClient;

    public static string VersionsDirectory =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            "Glacier Launcher", "mc-versions");

    public static string CacheFile =>
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            "Glacier Launcher", "cache", "mc-versions.json");

    public string? LastError { get; private set; }

    public VanillaVersionService(SettingsService settingsService)
    {
        _settingsService = settingsService;
        _httpClient      = HttpFactory.Shared;
        Directory.CreateDirectory(VersionsDirectory);
    }

    public async Task<List<VanillaVersion>> GetVersionsAsync()
    {
        LastError = null;
        var versions = new List<VanillaVersion>();
        try
        {
            var cookie = await GetAuthCookieAsync();
            var updates = await FetchUpdatesAsync(cookie);
            var active  = _settingsService.Settings.ActiveVanillaVersion;

            foreach (var u in updates)
            {
                var ver = ParseVersionFromPackage(u.PackageName);
                if (string.IsNullOrEmpty(ver)) continue;

                var versionDir = Path.Combine(VersionsDirectory, ver);
                var manifest   = Path.Combine(versionDir, "AppxManifest.xml");

                versions.Add(new VanillaVersion
                {
                    Version      = ver,
                    UpdateId     = u.UpdateId,
                    PackageName  = u.PackageName,
                    IsDownloaded = File.Exists(manifest),
                    IsActive     = ver == active,
                    SizeBytes    = u.Size
                });
            }

            versions = versions
                .OrderByDescending(v => TryParseVersion(v.Version))
                .ToList();

            SaveVersionCache(versions);
        }
        catch (Exception ex)
        {
            LastError = ex.Message;
            versions = LoadVersionCache();
        }

        var activeVer = _settingsService.Settings.ActiveVanillaVersion;
        foreach (var v in versions)
        {
            var versionDir = Path.Combine(VersionsDirectory, v.Version);
            v.IsDownloaded = File.Exists(Path.Combine(versionDir, "AppxManifest.xml"));
            v.IsActive = v.Version == activeVer;
        }

        return versions;
    }

    public async Task DownloadVersionAsync(VanillaVersion version, IProgress<double>? progress = null)
    {
        var downloadUrl = await GetDownloadUrlAsync(version.UpdateId);
        if (string.IsNullOrEmpty(downloadUrl))
            throw new InvalidOperationException("Could not resolve download URL for this version.");

        var tempFile = Path.Combine(VersionsDirectory, version.Version + ".appx.tmp");
        var versionDir = Path.Combine(VersionsDirectory, version.Version);

        try
        {
            using var response = await _httpClient.GetAsync(downloadUrl, HttpCompletionOption.ResponseHeadersRead);
            response.EnsureSuccessStatusCode();

            var total = response.Content.Headers.ContentLength ?? version.SizeBytes;
            long downloaded = 0;
            var buf = new byte[65536];

            await using var stream = await response.Content.ReadAsStreamAsync();
            await using var fs = File.Create(tempFile);

            int read;
            while ((read = await stream.ReadAsync(buf)) > 0)
            {
                await fs.WriteAsync(buf.AsMemory(0, read));
                downloaded += read;
                if (total > 0) progress?.Report((double)downloaded / total * 100);
            }
        }
        catch
        {
            try { File.Delete(tempFile); } catch { }
            throw;
        }

        Directory.CreateDirectory(versionDir);

        try
        {
            await Task.Run(() =>
            {
                using var zip = ZipFile.OpenRead(tempFile);
                foreach (var entry in zip.Entries)
                {
                    if (entry.FullName.StartsWith("AppxSignature", StringComparison.OrdinalIgnoreCase))
                        continue;
                    if (entry.FullName.StartsWith("[Content_Types]", StringComparison.OrdinalIgnoreCase))
                        continue;

                    var destPath = Path.Combine(versionDir, entry.FullName);
                    var destDir  = Path.GetDirectoryName(destPath)!;
                    Directory.CreateDirectory(destDir);

                    if (string.IsNullOrEmpty(entry.Name)) continue;
                    entry.ExtractToFile(destPath, overwrite: true);
                }
            });
        }
        finally
        {
            try { File.Delete(tempFile); } catch { }
        }

        version.IsDownloaded = true;
        progress?.Report(100);
    }

    public async Task<string> SwitchVersionAsync(VanillaVersion version)
    {
        var versionDir = Path.Combine(VersionsDirectory, version.Version);
        var manifest   = Path.Combine(versionDir, "AppxManifest.xml");

        if (!File.Exists(manifest))
            return "Version not downloaded. Download it first.";

        if (!IsDeveloperModeEnabled())
            return "Developer Mode must be enabled in Windows Settings > For Developers.";

        var unregResult = await UnregisterCurrentAsync();
        if (!string.IsNullOrEmpty(unregResult))
            return unregResult;

        var regResult = await RegisterVersionAsync(manifest);
        if (!string.IsNullOrEmpty(regResult))
            return regResult;

        _settingsService.Settings.ActiveVanillaVersion = version.Version;
        _settingsService.Save();
        version.IsActive = true;
        return "";
    }

    public void DeleteVersion(VanillaVersion version)
    {
        var versionDir = Path.Combine(VersionsDirectory, version.Version);
        try
        {
            if (Directory.Exists(versionDir))
                Directory.Delete(versionDir, recursive: true);
        }
        catch { }

        version.IsDownloaded = false;

        if (_settingsService.Settings.ActiveVanillaVersion == version.Version)
        {
            _settingsService.Settings.ActiveVanillaVersion = "";
            _settingsService.Save();
        }
    }

    public async Task ReregisterActiveAsync()
    {
        var active = _settingsService.Settings.ActiveVanillaVersion;
        if (string.IsNullOrEmpty(active)) return;

        var manifest = Path.Combine(VersionsDirectory, active, "AppxManifest.xml");
        if (!File.Exists(manifest)) return;

        await RegisterVersionAsync(manifest);
    }

    // ── Microsoft Store Update API ──────────────────────────────

    private record UpdateInfo(string UpdateId, string PackageName, long Size);

    private static readonly int[] InstalledNonLeafUpdateIDs =
    {
        1, 2, 3, 11, 19, 2359974, 5169044, 8788830, 23110993, 23110994,
        54341900, 59830006, 59830007, 59830008, 60484010, 62450018, 62450019,
        62450020, 98959022, 98959023, 98959024, 98959025, 98959026, 104433538,
        129905029, 130040031, 132387090, 132393049, 133399034, 138537048,
        140377312, 143747671, 158941041, 158941042, 158941043, 158941044,
        159123858, 159130928, 164836897, 164847386, 164848327, 164852241,
        164852246, 164852253
    };

    private static string SecurityHeader()
    {
        var now     = DateTime.UtcNow;
        var expires = now.AddMinutes(5);
        return $@"<o:Security s:mustUnderstand=""1""
                xmlns:o=""http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd"">
            <Timestamp xmlns=""http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd"">
                <Created>{now:yyyy-MM-ddTHH:mm:ss.fffffffZ}</Created>
                <Expires>{expires:yyyy-MM-ddTHH:mm:ss.fffffffZ}</Expires>
            </Timestamp>
            <wuws:WindowsUpdateTicketsToken wsu:id=""ClientMSA""
                xmlns:wuws=""http://schemas.microsoft.com/msus/2014/10/WindowsUpdateAuthorization""
                xmlns:wsu=""http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd"">
                <TicketType Name=""MSA"" Version=""1.0"" Policy=""MBI_SSL"">
                    <User/>
                </TicketType>
            </wuws:WindowsUpdateTicketsToken>
        </o:Security>";
    }

    private async Task<string> GetAuthCookieAsync()
    {
        var body = $@"<s:Envelope xmlns:s=""http://www.w3.org/2003/05/soap-envelope""
            xmlns:a=""http://www.w3.org/2005/08/addressing"">
            <s:Header>
                <a:Action s:mustUnderstand=""1"">http://www.microsoft.com/SoftwareDistribution/Server/ClientWebService/GetCookie</a:Action>
                <a:To s:mustUnderstand=""1"">https://fe3.delivery.mp.microsoft.com/ClientWebService/client.asmx</a:To>
                {SecurityHeader()}
            </s:Header>
            <s:Body>
                <GetCookie xmlns=""http://www.microsoft.com/SoftwareDistribution/Server/ClientWebService"">
                    <oldCookie></oldCookie>
                    <lastChange>2015-10-21T17:01:07.1472913Z</lastChange>
                    <currentTime>{DateTime.UtcNow:yyyy-MM-ddTHH:mm:ss.fffffffZ}</currentTime>
                    <protocolVersion>1.40</protocolVersion>
                </GetCookie>
            </s:Body>
        </s:Envelope>";

        var xml = await PostSoapAsync(body);
        var cookie = xml.Descendants(Wuws + "EncryptedData").FirstOrDefault()?.Value;
        return cookie ?? throw new Exception("Failed to get auth cookie from Microsoft Store.");
    }

    private async Task<List<UpdateInfo>> FetchUpdatesAsync(string cookie)
    {
        var idsXml = string.Join("\n", InstalledNonLeafUpdateIDs.Select(i => $"<int>{i}</int>"));

        var body = $@"<s:Envelope xmlns:s=""http://www.w3.org/2003/05/soap-envelope""
            xmlns:a=""http://www.w3.org/2005/08/addressing"">
            <s:Header>
                <a:Action s:mustUnderstand=""1"">http://www.microsoft.com/SoftwareDistribution/Server/ClientWebService/SyncUpdates</a:Action>
                <a:To s:mustUnderstand=""1"">https://fe3.delivery.mp.microsoft.com/ClientWebService/client.asmx</a:To>
                {SecurityHeader()}
            </s:Header>
            <s:Body>
                <SyncUpdates xmlns=""http://www.microsoft.com/SoftwareDistribution/Server/ClientWebService"">
                    <cookie>
                        <Expiration>2036-01-01T00:00:00Z</Expiration>
                        <EncryptedData>{cookie}</EncryptedData>
                    </cookie>
                    <parameters>
                        <ExpressQuery>false</ExpressQuery>
                        <InstalledNonLeafUpdateIDs>
                            {idsXml}
                        </InstalledNonLeafUpdateIDs>
                        <OtherCachedUpdateIDs/>
                        <SkipSoftwareSync>false</SkipSoftwareSync>
                        <NeedTwoGroupOutOfScopeUpdates>true</NeedTwoGroupOutOfScopeUpdates>
                        <FilterAppCategoryIds>
                            <CategoryIdentifier>
                                <Id>{WuCategoryId}</Id>
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
                            <DeviceAttributes>E:BranchReadinessLevel=CBServicingBranch&amp;AttrDataVer=264&amp;FlightRing=Retail&amp;FlightContent=Mainline&amp;InstallLanguage=en-US&amp;OSUILocale=en-US&amp;InstallationType=Client&amp;OSArchitecture=AMD64&amp;OSVersion=10.0.22621.1&amp;OSSkuId=48&amp;App=WU&amp;IsFlightingEnabled=1&amp;IsDeviceRetailDemo=0</DeviceAttributes>
                            <CallerAttributes>Interactive=1;IsSeeker=1;</CallerAttributes>
                            <Products/>
                        </ProductsParameters>
                    </parameters>
                </SyncUpdates>
            </s:Body>
        </s:Envelope>";

        var xml = await PostSoapAsync(body);

        var idMap = new Dictionary<string, string>();
        foreach (var upd in xml.Descendants(Wuws + "UpdateInfo"))
        {
            var id  = upd.Element(Wuws + "ID")?.Value;
            var xml2 = upd.Descendants(Wuws + "Xml").FirstOrDefault()?.Value;
            if (id != null && xml2 != null)
                idMap[id] = xml2;
        }

        var results = new List<UpdateInfo>();
        foreach (var (id, fragmentXml) in idMap)
        {
            try
            {
                if (!fragmentXml.Contains("Microsoft.MinecraftUWP", StringComparison.OrdinalIgnoreCase))
                    continue;
                if (fragmentXml.Contains(".EAppx", StringComparison.OrdinalIgnoreCase))
                    continue;
                if (fragmentXml.Contains("arm", StringComparison.OrdinalIgnoreCase) &&
                    !fragmentXml.Contains("x64", StringComparison.OrdinalIgnoreCase))
                    continue;

                var fragDoc = XDocument.Parse("<r>" + fragmentXml + "</r>");

                var pkgIdentity = fragDoc.Descendants("AppxPackageInstallData")
                    .Select(e => e.Element("AppxMetadata")?.Attribute("PackageMoniker")?.Value)
                    .FirstOrDefault();

                if (string.IsNullOrEmpty(pkgIdentity)) continue;
                if (!pkgIdentity.Contains("x64", StringComparison.OrdinalIgnoreCase)) continue;

                var sizeStr = fragDoc.Descendants("File")
                    .Select(e => e.Attribute("Size")?.Value)
                    .FirstOrDefault();
                long.TryParse(sizeStr, out long size);

                var updateId = fragDoc.Descendants("UpdateIdentity")
                    .Select(e => e.Attribute("UpdateID")?.Value)
                    .FirstOrDefault() ?? id;

                results.Add(new UpdateInfo(updateId, pkgIdentity, size));
            }
            catch { }
        }

        return results;
    }

    private async Task<string?> GetDownloadUrlAsync(string updateId)
    {
        var body = $@"<s:Envelope xmlns:s=""http://www.w3.org/2003/05/soap-envelope""
            xmlns:a=""http://www.w3.org/2005/08/addressing"">
            <s:Header>
                <a:Action s:mustUnderstand=""1"">http://www.microsoft.com/SoftwareDistribution/Server/ClientWebService/GetExtendedUpdateInfo2</a:Action>
                <a:To s:mustUnderstand=""1"">https://fe3.delivery.mp.microsoft.com/ClientWebService/client.asmx/secured</a:To>
                {SecurityHeader()}
            </s:Header>
            <s:Body>
                <GetExtendedUpdateInfo2 xmlns=""http://www.microsoft.com/SoftwareDistribution/Server/ClientWebService"">
                    <updateIDs>
                        <UpdateIdentity>
                            <UpdateID>{updateId}</UpdateID>
                            <RevisionNumber>1</RevisionNumber>
                        </UpdateIdentity>
                    </updateIDs>
                    <infoTypes>
                        <XmlUpdateFragmentType>FileUrl</XmlUpdateFragmentType>
                        <XmlUpdateFragmentType>FileDecryption</XmlUpdateFragmentType>
                    </infoTypes>
                    <deviceAttributes>E:BranchReadinessLevel=CBServicingBranch&amp;AttrDataVer=264&amp;FlightRing=Retail&amp;FlightContent=Mainline&amp;InstallLanguage=en-US&amp;OSUILocale=en-US&amp;InstallationType=Client&amp;OSArchitecture=AMD64&amp;OSVersion=10.0.22621.1&amp;OSSkuId=48&amp;App=WU&amp;IsFlightingEnabled=1&amp;IsDeviceRetailDemo=0</deviceAttributes>
                </GetExtendedUpdateInfo2>
            </s:Body>
        </s:Envelope>";

        var xml = await PostSoapAsync(body, secured: true);

        var url = xml.Descendants(Wuws + "Url")
            .Select(e => e.Value)
            .FirstOrDefault(u => u.Contains(".appx", StringComparison.OrdinalIgnoreCase)
                              || u.Contains("microsoft", StringComparison.OrdinalIgnoreCase));

        return url;
    }

    private async Task<XDocument> PostSoapAsync(string body, bool secured = false)
    {
        var endpoint = secured
            ? "https://fe3.delivery.mp.microsoft.com/ClientWebService/client.asmx/secured"
            : "https://fe3.delivery.mp.microsoft.com/ClientWebService/client.asmx";

        using var content = new StringContent(body, Encoding.UTF8, "application/soap+xml");
        using var resp = await _httpClient.PostAsync(endpoint, content);
        resp.EnsureSuccessStatusCode();
        var text = await resp.Content.ReadAsStringAsync();
        return XDocument.Parse(text);
    }

    // ── AppX registration ────────────────────────────────────────

    private static async Task<string> UnregisterCurrentAsync()
    {
        try
        {
            var result = await RunPowerShellAsync(
                "Get-AppxPackage -Name Microsoft.MinecraftUWP | Remove-AppxPackage");
            return "";
        }
        catch (Exception ex)
        {
            if (ex.Message.Contains("not found", StringComparison.OrdinalIgnoreCase) ||
                ex.Message.Contains("no package", StringComparison.OrdinalIgnoreCase))
                return "";
            return $"Failed to unregister current version: {ex.Message}";
        }
    }

    private static async Task<string> RegisterVersionAsync(string manifestPath)
    {
        try
        {
            var escapedPath = manifestPath.Replace("'", "''");
            await RunPowerShellAsync(
                $"Add-AppxPackage -Register '{escapedPath}' -DevelopmentMode");
            return "";
        }
        catch (Exception ex)
        {
            return $"Failed to register version: {ex.Message}";
        }
    }

    private static async Task<string> RunPowerShellAsync(string command)
    {
        var psi = new ProcessStartInfo
        {
            FileName  = "powershell.exe",
            Arguments = $"-NoProfile -NonInteractive -Command \"{command.Replace("\"", "\\\"")}\"",
            UseShellExecute        = false,
            RedirectStandardOutput = true,
            RedirectStandardError  = true,
            CreateNoWindow         = true,
            WindowStyle            = ProcessWindowStyle.Hidden
        };

        using var proc = Process.Start(psi)
                         ?? throw new Exception("Failed to start PowerShell.");
        var stdout = await proc.StandardOutput.ReadToEndAsync();
        var stderr = await proc.StandardError.ReadToEndAsync();
        await proc.WaitForExitAsync();

        if (proc.ExitCode != 0 && !string.IsNullOrWhiteSpace(stderr))
            throw new Exception(stderr.Trim());

        return stdout;
    }

    private static bool IsDeveloperModeEnabled()
    {
        try
        {
            using var key = Microsoft.Win32.Registry.LocalMachine.OpenSubKey(
                @"SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock");
            if (key == null) return false;
            var val = key.GetValue("AllowDevelopmentWithoutDevLicense");
            return val is int i && i == 1;
        }
        catch { return false; }
    }

    // ── Version parsing ──────────────────────────────────────────

    private static readonly Regex VersionRegex =
        new(@"(\d+\.\d+\.\d+(?:\.\d+)?)", RegexOptions.Compiled);

    private static string? ParseVersionFromPackage(string packageName)
    {
        var m = VersionRegex.Match(packageName);
        return m.Success ? m.Groups[1].Value : null;
    }

    private static Version TryParseVersion(string ver)
    {
        return System.Version.TryParse(ver, out var v) ? v : new Version(0, 0);
    }

    // ── Cache ────────────────────────────────────────────────────

    private static void SaveVersionCache(List<VanillaVersion> versions)
    {
        try
        {
            var dir = Path.GetDirectoryName(CacheFile)!;
            Directory.CreateDirectory(dir);
            var data = versions.Select(v => new { v.Version, v.UpdateId, v.PackageName, v.SizeBytes }).ToList();
            var json = JsonSerializer.Serialize(data, new JsonSerializerOptions { WriteIndented = true });
            File.WriteAllText(CacheFile, json);
        }
        catch { }
    }

    private List<VanillaVersion> LoadVersionCache()
    {
        try
        {
            if (!File.Exists(CacheFile)) return new();
            var json = File.ReadAllText(CacheFile);
            using var doc = JsonDocument.Parse(json);
            var list = new List<VanillaVersion>();
            var active = _settingsService.Settings.ActiveVanillaVersion;

            foreach (var el in doc.RootElement.EnumerateArray())
            {
                var ver = el.GetProperty("Version").GetString() ?? "";
                var uid = el.GetProperty("UpdateId").GetString() ?? "";
                var pkg = el.GetProperty("PackageName").GetString() ?? "";
                var sz  = el.TryGetProperty("SizeBytes", out var szp) ? szp.GetInt64() : 0;

                var versionDir = Path.Combine(VersionsDirectory, ver);
                list.Add(new VanillaVersion
                {
                    Version      = ver,
                    UpdateId     = uid,
                    PackageName  = pkg,
                    SizeBytes    = sz,
                    IsDownloaded = File.Exists(Path.Combine(versionDir, "AppxManifest.xml")),
                    IsActive     = ver == active
                });
            }
            return list;
        }
        catch { return new(); }
    }
}
