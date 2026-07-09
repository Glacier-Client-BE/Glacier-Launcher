using System;
using System.Threading.Tasks;
using Microsoft.AspNetCore.Components;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private bool   onboardingOpen     = false;
    private int    onboardingStep     = 0;
    private string onboardingEdition  = "bedrock";
    private string onboardingUsername = "";
    private bool   onboardingImporting = false;
    private bool   onboardingImportDone = false;

    private void InitOnboarding()
    {
        onboardingOpen = !SettingsService.Settings.OnboardingCompleted;
        onboardingEdition = SettingsService.Settings.Edition;
        onboardingUsername = SettingsService.Settings.Username;
    }

    private void OnboardingNext()
    {
        if (onboardingStep == 0)
        {
            // "Both" isn't a real edition value elsewhere in the app (IsBedrock/IsJava
            // is a strict toggle) — start on Bedrock, the edition switcher on the home
            // screen lets them flip to Java anytime.
            SetEditionCore(onboardingEdition == "both" ? "bedrock" : onboardingEdition);
        }
        // Java's import step applies when Java (or Both, since they use Java too) was picked.
        var lastStep = onboardingEdition is "java" or "both" ? 2 : 1;
        onboardingStep = Math.Min(onboardingStep + 1, lastStep);
        StateHasChanged();
    }

    private void OnboardingBack()
    {
        onboardingStep = Math.Max(onboardingStep - 1, 0);
        StateHasChanged();
    }

    private void OnboardingUsernameInput(ChangeEventArgs e) =>
        onboardingUsername = e.Value?.ToString() ?? "";

    private void OnboardingPickEdition(string edition) => onboardingEdition = edition;

    private async Task OnboardingImportMinecraft()
    {
        onboardingImporting = true;
        StateHasChanged();
        try
        {
            var instance = await JavaInstances.ImportOfficialMinecraftAsync();
            JavaInstances.SetActive(instance.Id);
            onboardingImportDone = true;
            _ = ShowToast("Imported your existing Minecraft data.", "success");
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        finally { onboardingImporting = false; StateHasChanged(); }
    }

    private void FinishOnboarding()
    {
        if (!string.IsNullOrWhiteSpace(onboardingUsername))
        {
            displayName = onboardingUsername.Trim();
            displayHandle = "@" + displayName.ToLowerInvariant().Replace(" ", "");
            SettingsService.Settings.Username = displayName;
            SettingsService.Settings.UserHandle = displayHandle;
        }
        SettingsService.Settings.OnboardingCompleted = true;
        SettingsService.Save();
        onboardingOpen = false;
        StateHasChanged();
    }

    private void SkipOnboarding()
    {
        SettingsService.Settings.OnboardingCompleted = true;
        SettingsService.Save();
        onboardingOpen = false;
        StateHasChanged();
    }
}
