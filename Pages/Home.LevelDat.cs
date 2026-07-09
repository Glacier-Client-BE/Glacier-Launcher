using System;
using System.Linq;
using GlacierLauncher.Models;

namespace GlacierLauncher.Pages;

public partial class Home
{
    private bool levelDatModalOpen = false;
    private bool levelDatSaving = false;
    private LevelDatSummary? levelDatEditing = null;
    private string levelDatWorldName = "";

    private void OpenLevelDatEditor(BedrockWorld w)
    {
        try
        {
            levelDatEditing = LevelDat.Load(w.FolderPath);
            levelDatWorldName = w.Name;
            levelDatModalOpen = true;
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        StateHasChanged();
    }

    private void CloseLevelDatEditor()
    {
        levelDatModalOpen = false;
        levelDatEditing = null;
    }

    private void SaveLevelDat()
    {
        if (levelDatEditing == null) return;
        levelDatSaving = true;
        StateHasChanged();
        try
        {
            LevelDat.Save(levelDatEditing);
            _ = ShowToast("level.dat saved.", "success");
            CloseLevelDatEditor();
        }
        catch (Exception ex) { _ = ShowToast(ex.Message, "error"); }
        finally { levelDatSaving = false; StateHasChanged(); }
    }

    private void OnLevelDatGameTypeChanged(Microsoft.AspNetCore.Components.ChangeEventArgs e)
    {
        if (levelDatEditing != null && int.TryParse(e.Value?.ToString(), out var v)) levelDatEditing.GameType = v;
    }

    private void OnLevelDatDifficultyChanged(Microsoft.AspNetCore.Components.ChangeEventArgs e)
    {
        if (levelDatEditing != null && int.TryParse(e.Value?.ToString(), out var v)) levelDatEditing.Difficulty = v;
    }

    private void OnLevelDatGeneratorChanged(Microsoft.AspNetCore.Components.ChangeEventArgs e)
    {
        if (levelDatEditing != null && int.TryParse(e.Value?.ToString(), out var v)) levelDatEditing.Generator = v;
    }

    private void ToggleLevelDatCheats() { if (levelDatEditing != null) levelDatEditing.CheatsEnabled = !levelDatEditing.CheatsEnabled; }

    private void ToggleLevelDatExperiment(string name)
    {
        if (levelDatEditing == null) return;
        levelDatEditing.Experiments[name] = !levelDatEditing.Experiments[name];
    }

    private static readonly (int Value, string Label)[] GameTypeOptions =
        { (0, "Survival"), (1, "Creative"), (2, "Adventure"), (3, "Spectator") };

    private static readonly (int Value, string Label)[] DifficultyOptions =
        { (0, "Peaceful"), (1, "Easy"), (2, "Normal"), (3, "Hard") };

    private static readonly (int Value, string Label)[] GeneratorOptions =
        { (0, "Legacy"), (1, "Infinite"), (2, "Flat") };
}
