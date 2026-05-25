namespace GlacierLauncher.Models;

public class XboxProfile
{
    public string Gamertag        { get; set; } = "";
    public string Xuid            { get; set; } = "";
    public string GamerPictureUrl { get; set; } = "";
    public string Gamerscore      { get; set; } = "";
    public string AccountTier     { get; set; } = "";
    public string Bio             { get; set; } = "";
}

public class JavaAccount
{
    public string Id { get; set; } = "";
    public string Gamertag { get; set; } = "";
    public string Xuid { get; set; } = "";
    public string GamerPictureUrl { get; set; } = "";
    public string MinecraftUsername { get; set; } = "";
    public string MinecraftUuid { get; set; } = "";
    public string MinecraftAccessToken { get; set; } = "";
    public string MinecraftAccessTokenExpiry { get; set; } = "";
    public string MinecraftSkinUrl { get; set; } = "";
    public string RefreshToken { get; set; } = "";
    public string LastUsedAt { get; set; } = "";
}
