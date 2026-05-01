using System;
using DiscordRPC;
using DiscordRPC.Logging;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

public class DiscordRpcService : IDisposable
{
    private const string ApplicationId = "1482726422094024779";

    private DiscordRpcClient? _client;
    private readonly SettingsService _settings;
    private bool _running;

    public DiscordRpcService(SettingsService settings)
    {
        _settings = settings;
    }

    public void Start()
    {
        if (!_settings.Settings.DiscordRichPresence || _running) return;

        try
        {
            _client = new DiscordRpcClient(ApplicationId) { Logger = new NullLogger() };
            _client.Initialize();
            SetIdlePresence();
            _running = true;
        }
        catch (Exception ex)
        {
            Console.WriteLine($"[Discord RPC] {ex.Message}");
        }
    }

    public void Stop()
    {
        if (!_running || _client == null) return;

        try
        {
            _client.ClearPresence();
            _client.Deinitialize();
            _client.Dispose();
            _client = null;
            _running = false;
        }
        catch { }
    }

    public void Toggle()
    {
        _settings.Settings.DiscordRichPresence = !_settings.Settings.DiscordRichPresence;
        _settings.Save();

        if (_settings.Settings.DiscordRichPresence)
            Start();
        else
            Stop();
    }

    public void SetIdlePresence()
    {
        if (!_running || _client == null) return;

        _client.SetPresence(new RichPresence
        {
            Details    = "In the Launcher",
            State      = "Selecting a version",
            Assets     = new Assets { LargeImageKey = "glacier_logo", LargeImageText = "Glacier Launcher" },
            Timestamps = Timestamps.Now
        });
    }

    public void SetInGamePresence(string versionTag, string? clientName = null)
    {
        if (!_running || _client == null) return;

        var state = clientName switch
        {
        "Flarial Client" => "Using Flarial",
        "OderSo Client"  => string.IsNullOrEmpty(versionTag) ? "Using OderSo" : $"Using OderSo · {versionTag}",
        "Custom DLL"     => string.IsNullOrEmpty(versionTag) ? "Using a custom DLL" : $"Using {versionTag}",
        "Vanilla"        => "Playing Vanilla",
        _                => string.IsNullOrEmpty(versionTag) ? "Using Latite" : $"Using Latite · {versionTag}",
    };

    _client.SetPresence(new RichPresence
    {
        Details    = "Playing Minecraft",
        State      = state,
        Assets     = new Assets
        {
            LargeImageKey  = "minecraft_icon",
            LargeImageText = "Minecraft",
            SmallImageKey  = "glacier_logo",
            SmallImageText = "Glacier Launcher"
        },
        Timestamps = Timestamps.Now
    });
}

    public void Dispose() => Stop();
}
