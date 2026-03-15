using System;
using DiscordRPC;
using DiscordRPC.Logging;
using MinecraftLauncher.Models;

namespace MinecraftLauncher.Services;

/// <summary>
/// Manages Discord Rich Presence for the Glacier Launcher.
/// Call <see cref="Start"/> once and <see cref="Stop"/> on app exit.
/// </summary>
public class DiscordRpcService : IDisposable
{
    // Register your own application at https://discord.com/developers/applications
    // and replace this ID with yours to get custom assets/text.
    private const string ApplicationId = "1482726422094024779";

    private DiscordRpcClient? _client;
    private readonly SettingsService _settings;
    private bool _running;

    public DiscordRpcService(SettingsService settings)
    {
        _settings = settings;
    }

    /// <summary>Start the RPC connection if Discord Rich Presence is enabled.</summary>
    public void Start()
    {
        if (!_settings.Settings.DiscordRichPresence) return;
        if (_running) return;

        try
        {
            _client = new DiscordRpcClient(ApplicationId)
            {
                Logger = new NullLogger()
            };

            _client.OnReady += (_, args) =>
                Console.WriteLine($"[Discord RPC] Connected as {args.User.Username}");

            _client.OnError += (_, args) =>
                Console.WriteLine($"[Discord RPC] Error: {args.Message}");

            _client.Initialize();
            SetIdlePresence();
            _running = true;
        }
        catch (Exception ex)
        {
            Console.WriteLine($"[Discord RPC] Failed to start: {ex.Message}");
        }
    }

    /// <summary>Stop the RPC connection and clear presence.</summary>
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
        catch (Exception ex)
        {
            Console.WriteLine($"[Discord RPC] Failed to stop: {ex.Message}");
        }
    }

    /// <summary>Toggle Discord RPC on or off and persist the setting.</summary>
    public void Toggle()
    {
        _settings.Settings.DiscordRichPresence = !_settings.Settings.DiscordRichPresence;
        _settings.Save();

        if (_settings.Settings.DiscordRichPresence)
            Start();
        else
            Stop();
    }

    /// <summary>Update presence to show the launcher is idle (browsing versions, etc.).</summary>
    public void SetIdlePresence()
    {
        if (!_running || _client == null) return;

        _client.SetPresence(new RichPresence
        {
            Details = "In the Launcher",
            State   = "Selecting a version",
            Assets  = new Assets
            {
                LargeImageKey  = "glacier_logo",
                LargeImageText = "Glacier Launcher",
            },
            Timestamps = Timestamps.Now
        });
    }

    /// <summary>Update presence to show Minecraft is running with the given client version.</summary>
    public void SetInGamePresence(string versionTag)
    {
        if (!_running || _client == null) return;

        _client.SetPresence(new RichPresence
        {
            Details = "Playing Minecraft",
            State   = $"Using Latite {versionTag}",
            Assets  = new Assets
            {
                LargeImageKey  = "minecraft_icon",
                LargeImageText = "Minecraft",
                SmallImageKey  = "glacier_logo",
                SmallImageText = "Glacier Launcher",
            },
            Timestamps = Timestamps.Now
        });
    }

    public void Dispose() => Stop();
}
