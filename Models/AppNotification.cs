using System;

namespace GlacierLauncher.Models;

public class AppNotification
{
    public string Id        { get; set; } = Guid.NewGuid().ToString("N");
    public string Title     { get; set; } = "";
    public string Message   { get; set; } = "";
    public string Kind      { get; set; } = "info"; // info | success | error
    public string Icon      { get; set; } = "fa-solid fa-circle-info";
    public string TimeIso   { get; set; } = DateTime.UtcNow.ToString("o");
    public bool   Read      { get; set; }
    public string Url       { get; set; } = "";
}
