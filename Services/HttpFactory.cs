using System;
using System.Net.Http;

namespace GlacierLauncher.Services;

/// <summary>
/// Shared HttpClient instance for all services. A single client (re)used across the
/// process avoids socket exhaustion and keeps connection pooling effective.
/// </summary>
public static class HttpFactory
{
    private static readonly Lazy<HttpClient> _shared = new(() =>
    {
        var handler = new SocketsHttpHandler
        {
            PooledConnectionLifetime    = TimeSpan.FromMinutes(2),
            PooledConnectionIdleTimeout = TimeSpan.FromSeconds(60),
            MaxConnectionsPerServer     = 8,
            AutomaticDecompression      = System.Net.DecompressionMethods.GZip
                                        | System.Net.DecompressionMethods.Deflate
                                        | System.Net.DecompressionMethods.Brotli
        };
        var client = new HttpClient(handler) { Timeout = TimeSpan.FromMinutes(5) };
        client.DefaultRequestHeaders.Add("User-Agent", "GlacierLauncher/1.0");
        return client;
    });

    public static HttpClient Shared => _shared.Value;
}

/// <summary>
/// Reports progress at most every <paramref name="minIntervalMs"/> milliseconds.
/// Reduces UI re-renders during fast network downloads.
/// </summary>
public sealed class ThrottledProgress : IProgress<double>
{
    private readonly IProgress<double>? _inner;
    private readonly long _minIntervalTicks;
    private long _lastTicks;
    private double _lastValue;

    public ThrottledProgress(IProgress<double>? inner, int minIntervalMs = 100)
    {
        _inner = inner;
        _minIntervalTicks = TimeSpan.FromMilliseconds(minIntervalMs).Ticks;
    }

    public void Report(double value)
    {
        if (_inner == null) return;
        var now = DateTime.UtcNow.Ticks;
        // Always forward 0 and 100 immediately; throttle in-between.
        if (value <= 0 || value >= 100 || (now - _lastTicks) >= _minIntervalTicks
            || Math.Abs(value - _lastValue) >= 1.0)
        {
            _lastTicks = now;
            _lastValue = value;
            _inner.Report(value);
        }
    }
}
