using System;
using System.Linq;
using System.Threading.Tasks;
using Windows.ApplicationModel.Store.Preview.InstallControl;
using Windows.Management.Deployment;

namespace GlacierLauncher.Services;

/// <summary>
/// Installs / updates the official Minecraft Bedrock UWP package through the
/// Microsoft Store's AppInstallManager. Mirrors the approach used by Aetopia's
/// BedrockUpdater (https://github.com/Aetopia/BedrockUpdater) — kept as an
/// alternative to the SOAP-based sideload flow in <see cref="VanillaVersionService"/>.
///
/// Use this when:
///   • the user wants the latest official Release/Preview without enabling
///     Developer Mode and sideloading;
///   • a sideload attempt failed and we want a guaranteed-clean fall back.
/// </summary>
public sealed class StoreInstallService
{
    public sealed record Product(string ProductId, string PackageFamilyName, string DisplayName)
    {
        public static readonly Product MinecraftRelease  = new("9NBLGGH2JHXJ", "Microsoft.MinecraftUWP_8wekyb3d8bbwe",         "Minecraft (Release)");
        public static readonly Product MinecraftPreview  = new("9P5X4QVLC2XR", "Microsoft.MinecraftWindowsBeta_8wekyb3d8bbwe", "Minecraft (Preview)");
        public static readonly Product GamingServices    = new("9MWPM2CQNLHN", "Microsoft.GamingServices_8wekyb3d8bbwe",        "Gaming Services");
    }

    public sealed record InstallProgress(
        string ProductName,
        AppInstallState State,
        double Percent,
        long BytesDownloaded,
        long BytesTotal)
    {
        public string Stage => State switch
        {
            AppInstallState.Pending     => "Pending",
            AppInstallState.Downloading => "Downloading",
            AppInstallState.Installing  => "Installing",
            AppInstallState.Completed   => "Completed",
            AppInstallState.Canceled    => "Cancelled",
            AppInstallState.Error       => "Error",
            AppInstallState.Paused      => "Paused",
            _                            => State.ToString()
        };
    }

    private readonly AppInstallManager _installer = new();
    private readonly PackageManager    _packages  = new();

    private AppInstallItem? _activeItem;
    private TaskCompletionSource<bool>? _activeSource;

    public bool IsBusy => _activeSource != null && !_activeSource.Task.IsCompleted;

    /// <summary>
    /// Installs / updates the given product. Reports incremental progress via
    /// <paramref name="progress"/>. Returns the final state — <c>Completed</c>
    /// on success, anything else means the user (or the platform) aborted.
    /// Throws if the platform surfaces an error.
    /// </summary>
    public async Task<AppInstallState> InstallAsync(Product product, IProgress<InstallProgress>? progress = null)
    {
        // Always pre-install GamingServices first when targeting a Minecraft
        // SKU — without it modern Minecraft refuses to start on a fresh PC.
        if (product != Product.GamingServices)
        {
            await InstallInternalAsync(Product.GamingServices, progress, throwOnError: false);
        }

        return await InstallInternalAsync(product, progress, throwOnError: true);
    }

    /// <summary>
    /// Cancels (or pauses, if the package is already registered) the in-flight
    /// install. No-op if nothing is running.
    /// </summary>
    public bool Cancel()
    {
        if (_activeItem is null || _activeSource is null || _activeSource.Task.IsCompleted)
            return false;

        try
        {
            if (_packages.FindPackagesForUser(string.Empty, _activeItem.PackageFamilyName).Any())
                _activeItem.Pause();
            else
                _activeItem.Cancel();
        }
        catch { /* item may already be in a terminal state */ }

        return true;
    }

    private async Task<AppInstallState> InstallInternalAsync(
        Product product,
        IProgress<InstallProgress>? progress,
        bool throwOnError)
    {
        var item = await ResolveInstallItemAsync(product);
        if (item is null)
            return AppInstallState.Completed; // nothing to do — clean state

        _activeItem   = item;
        _activeSource = new TaskCompletionSource<bool>();

        // Push to the front of the queue so the user-initiated install is
        // not stuck behind Windows Update sundries.
        try { _installer.MoveToFrontOfDownloadQueue(item.ProductId, string.Empty); } catch { }

        item.Completed += (sender, _) =>
        {
            var state = sender.GetCurrentStatus().InstallState;
            if (state == AppInstallState.Completed)      _activeSource.TrySetResult(true);
            else if (state == AppInstallState.Canceled)  _activeSource.TrySetResult(false);
        };

        item.StatusChanged += (sender, _) =>
        {
            var status = sender.GetCurrentStatus();
            switch (status.InstallState)
            {
                case AppInstallState.Pending:
                case AppInstallState.Downloading:
                case AppInstallState.Installing:
                    progress?.Report(new InstallProgress(
                        product.DisplayName,
                        status.InstallState,
                        status.PercentComplete,
                        (long)status.BytesDownloaded,
                        (long)status.DownloadSizeInBytes));
                    break;

                case AppInstallState.Paused:
                    _activeSource.TrySetResult(false);
                    break;

                case AppInstallState.Error:
                    if (throwOnError) _activeSource.TrySetException(status.ErrorCode ?? new Exception("Install failed."));
                    else              _activeSource.TrySetResult(false);
                    break;
            }
        };

        // Emit an initial pending tick so the UI has something to show.
        progress?.Report(new InstallProgress(product.DisplayName, AppInstallState.Pending, 0, 0, 0));

        var success = await _activeSource.Task;
        var finalState = item.GetCurrentStatus().InstallState;

        _activeItem   = null;
        _activeSource = null;

        return success ? AppInstallState.Completed : finalState;
    }

    private async Task<AppInstallItem?> ResolveInstallItemAsync(Product product)
    {
        // 1. Reuse / clean up any existing queue entry for this product.
        var existing = await Task.Run(() =>
        {
            AppInstallItem? match = null;
            foreach (var item in _installer.AppInstallItems)
            {
                if (item.GetCurrentStatus().InstallState == AppInstallState.Error)
                {
                    try { item.Cancel(); } catch { }
                    continue;
                }
                if (item.ProductId.Equals(product.ProductId, StringComparison.OrdinalIgnoreCase))
                    match = item;
            }
            return match;
        });

        if (existing is not null) return existing;

        // 2. Package already installed → update flow.
        if (_packages.FindPackagesForUser(string.Empty, product.PackageFamilyName).Any())
            return await _installer.UpdateAppByPackageFamilyNameAsync(product.PackageFamilyName);

        // 3. Fresh install.
        return await _installer.StartAppInstallAsync(product.ProductId, string.Empty, false, false);
    }
}
