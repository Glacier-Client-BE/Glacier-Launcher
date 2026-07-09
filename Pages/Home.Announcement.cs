using System.Threading.Tasks;
using GlacierLauncher.Services;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private Announcement? _announcement;

    private bool ShowAnnouncementBanner =>
        _announcement != null && _announcement.Id != SettingsService.Settings.LastDismissedAnnouncementId;

    private async Task LoadAnnouncementAsync()
    {
        _announcement = await AnnouncementService.GetAnnouncementAsync();
        if (_announcement != null) StateHasChanged();
    }

    private void DismissAnnouncement()
    {
        if (_announcement == null) return;
        SettingsService.Settings.LastDismissedAnnouncementId = _announcement.Id;
        SettingsService.Save();
        StateHasChanged();
    }
}
