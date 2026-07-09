using System;
using System.IO;
using GlacierLauncher.Models;

namespace GlacierLauncher.Services;

/// <summary>
/// Reads/writes a handful of well-known scalar fields in a Bedrock world's
/// level.dat without disturbing anything else in the file — the NBT tree is
/// parsed in full and only the fields the summary model exposes are mutated
/// before writing the whole tree back out.
/// </summary>
public class LevelDatEditorService
{
    // Bedrock level.dat = 8-byte header (int32 version LE, int32 payload length LE)
    // followed by exactly that many bytes of little-endian NBT (no gzip).
    private const int HeaderSize = 8;
    private const int SaveVersion = 10; // current Bedrock level.dat header version as of 1.20+

    public LevelDatSummary Load(string worldPath)
    {
        var path = Path.Combine(worldPath, "level.dat");
        if (!File.Exists(path))
            throw new FileNotFoundException("This world has no level.dat.", path);

        using var fs = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read);
        using var r = new BinaryReader(fs);
        r.ReadInt32(); // version — not needed to read the fields, preserved verbatim on save
        var length = r.ReadInt32();
        if (length < 0 || HeaderSize + length > fs.Length)
            throw new InvalidDataException("level.dat header doesn't match the file size — it may be corrupt.");

        var root = NbtIo.ReadRootTag(r);

        var summary = new LevelDatSummary { WorldPath = worldPath };

        summary.GameType      = ReadInt(root, "GameType", 0);
        summary.Difficulty    = ReadInt(root, "Difficulty", 2);
        summary.CheatsEnabled = ReadByte(root, "commandsEnabled") != 0;
        summary.Generator     = ReadInt(root, "Generator", 1);

        var seedTag = NbtIo.Find(root, "RandomSeed");
        if (seedTag != null)
        {
            summary.HasRandomSeed = true;
            summary.RandomSeed = seedTag.Type == NbtType.Long ? (long)seedTag.Value! : (int)seedTag.Value!;
        }

        var experiments = NbtIo.Find(root, "experiments");
        if (experiments != null && experiments.Type == NbtType.Compound)
        {
            foreach (var child in experiments.Items)
            {
                if (child.Type == NbtType.Byte)
                    summary.Experiments[child.Name] = (byte)child.Value! != 0;
            }
        }

        return summary;
    }

    public void Save(LevelDatSummary summary)
    {
        var path = Path.Combine(summary.WorldPath, "level.dat");
        if (!File.Exists(path))
            throw new FileNotFoundException("This world has no level.dat.", path);

        NbtTag root;
        int version;
        using (var fs = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.Read))
        using (var r = new BinaryReader(fs))
        {
            version = r.ReadInt32();
            r.ReadInt32(); // length — recomputed on write
            root = NbtIo.ReadRootTag(r);
        }

        NbtIo.SetOrAdd(root, "GameType", NbtType.Int, summary.GameType);
        NbtIo.SetOrAdd(root, "Difficulty", NbtType.Int, summary.Difficulty);
        NbtIo.SetOrAdd(root, "commandsEnabled", NbtType.Byte, (byte)(summary.CheatsEnabled ? 1 : 0));
        NbtIo.SetOrAdd(root, "Generator", NbtType.Int, summary.Generator);

        if (summary.Experiments.Count > 0)
        {
            var experiments = NbtIo.Find(root, "experiments");
            if (experiments == null)
            {
                experiments = new NbtTag { Type = NbtType.Compound, Name = "experiments" };
                root.Items.Add(experiments);
            }
            foreach (var (name, enabled) in summary.Experiments)
                NbtIo.SetOrAdd(experiments, name, NbtType.Byte, (byte)(enabled ? 1 : 0));
        }

        // Write payload to a buffer first so we know its length for the header,
        // then swap the whole file atomically — a half-written level.dat would
        // brick the world, so this must never leave a partial file on disk.
        byte[] payload;
        using (var ms = new MemoryStream())
        using (var w = new BinaryWriter(ms))
        {
            NbtIo.WriteRootTag(w, root);
            payload = ms.ToArray();
        }

        var tmpPath = path + ".tmp";
        using (var outFs = new FileStream(tmpPath, FileMode.Create, FileAccess.Write))
        using (var w = new BinaryWriter(outFs))
        {
            w.Write(version > 0 ? version : SaveVersion);
            w.Write(payload.Length);
            w.Write(payload);
        }
        File.Move(tmpPath, path, overwrite: true);
    }

    private static int ReadInt(NbtTag compound, string name, int fallback)
    {
        var tag = NbtIo.Find(compound, name);
        return tag?.Value is int i ? i : fallback;
    }

    private static byte ReadByte(NbtTag compound, string name)
    {
        var tag = NbtIo.Find(compound, name);
        return tag?.Value is byte b ? b : (byte)0;
    }
}
