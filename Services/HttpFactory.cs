using System;
using System.Net.Http;

namespace GlacierLauncher.Services;

public static class HttpFactory
{
    private static readonly Lazy<HttpClient> _shared = new(() =>
    {
        var handler = new SocketsHttpHandler
        {
            PooledConnectionLifetime    = TimeSpan.FromMinutes(2),
            PooledConnectionIdleTimeout = TimeSpan.FromSeconds(60),
            MaxConnectionsPerServer     = 24,
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

public sealed class ThrottledProgress : IProgress<double>
{
    private readonly IProgress<double>? _inner;
    private readonly long _minIntervalTicks;
    private readonly double _completeValue;
    private readonly double _minDelta;
    private long _lastTicks;
    private double _lastValue;

    public ThrottledProgress(
        IProgress<double>? inner,
        int    minIntervalMs = 100,
        double completeValue = 100,
        double minDelta      = 1.0)
    {
        _inner            = inner;
        _minIntervalTicks = TimeSpan.FromMilliseconds(minIntervalMs).Ticks;
        _completeValue    = completeValue;
        _minDelta         = minDelta;
    }

    public void Report(double value)
    {
        if (_inner == null) return;
        var now = DateTime.UtcNow.Ticks;
        // Boundaries (reset / completion) always pass. Everything else must
        // clear BOTH gates: enough time elapsed AND enough movement. With the
        // old OR-logic a fast download hit the delta gate every few
        // milliseconds and re-rendered the UI up to 100×/s; a progress bar
        // repainting 10×/s in ≥1% steps is visually identical.
        if (value <= 0 || value >= _completeValue
            || ((now - _lastTicks) >= _minIntervalTicks && Math.Abs(value - _lastValue) >= _minDelta))
        {
            _lastTicks = now;
            _lastValue = value;
            _inner.Report(value);
        }
    }
}
