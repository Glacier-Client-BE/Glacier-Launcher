using System.Collections.Generic;

namespace GlacierLauncher.Models;

public class LevelDatSummary
{
    public string WorldPath      { get; set; } = "";
    public int    GameType       { get; set; } // 0 Survival, 1 Creative, 2 Adventure, 3 Spectator
    public int    Difficulty     { get; set; } // 0 Peaceful, 1 Easy, 2 Normal, 3 Hard
    public bool   CheatsEnabled  { get; set; }
    public int    Generator      { get; set; } // 0 Legacy, 1 Overworld/Infinite, 2 Flat
    public long   RandomSeed     { get; set; } // read-only in the UI — changing it post-creation doesn't regenerate terrain
    public bool   HasRandomSeed  { get; set; }

    // Byte-flag toggles under the "experiments" compound, name → enabled. Shown
    // generically (not hardcoded per-flag) since experiment names change across
    // game versions and a stale hardcoded list would silently drop new ones.
    public Dictionary<string, bool> Experiments { get; set; } = new();
}
