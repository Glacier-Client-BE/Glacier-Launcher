using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace GlacierLauncher.Services;

// Minimal little-endian NBT reader/writer for Bedrock's level.dat. Bedrock NBT
// differs from Java NBT only in endianness (everything LE here, including
// string/list/array lengths) and in never being gzip-compressed on disk.
// Deliberately generic (an ordered tag tree, not a typed POCO) so a save
// round-trips every byte the game itself doesn't understand back untouched —
// the editor only mutates specific leaf values it explicitly knows about.
public enum NbtType : byte
{
    End = 0, Byte = 1, Short = 2, Int = 3, Long = 4, Float = 5, Double = 6,
    ByteArray = 7, String = 8, List = 9, Compound = 10, IntArray = 11, LongArray = 12
}

public sealed class NbtTag
{
    public NbtType Type { get; set; }
    public string  Name { get; set; } = "";
    public object? Value { get; set; } // see type mapping in NbtIo remarks below

    // For List tags: the element type of every item in Items.
    public NbtType ListElementType { get; set; }
    public List<NbtTag> Items { get; set; } = new(); // Compound (named children) or List (unnamed elements)
}

/// <summary>
/// Value type mapping by NbtType: Byte→byte, Short→short, Int→int, Long→long,
/// Float→float, Double→double, ByteArray→byte[], String→string,
/// IntArray→int[], LongArray→long[]. Compound/List use <see cref="NbtTag.Items"/> instead.
/// </summary>
public static class NbtIo
{
    public static NbtTag ReadRootTag(BinaryReader r)
    {
        var type = (NbtType)r.ReadByte();
        var name = ReadString(r);
        var tag = new NbtTag { Type = type, Name = name };
        ReadPayload(r, tag);
        return tag;
    }

    public static void WriteRootTag(BinaryWriter w, NbtTag tag)
    {
        w.Write((byte)tag.Type);
        WriteString(w, tag.Name);
        WritePayload(w, tag);
    }

    private static NbtTag ReadNamedTag(BinaryReader r)
    {
        var type = (NbtType)r.ReadByte();
        if (type == NbtType.End) return new NbtTag { Type = NbtType.End };
        var name = ReadString(r);
        var tag = new NbtTag { Type = type, Name = name };
        ReadPayload(r, tag);
        return tag;
    }

    private static void WriteNamedTag(BinaryWriter w, NbtTag tag)
    {
        w.Write((byte)tag.Type);
        WriteString(w, tag.Name);
        WritePayload(w, tag);
    }

    private static void ReadPayload(BinaryReader r, NbtTag tag)
    {
        switch (tag.Type)
        {
            case NbtType.Byte: tag.Value = r.ReadByte(); break;
            case NbtType.Short: tag.Value = r.ReadInt16(); break;
            case NbtType.Int: tag.Value = r.ReadInt32(); break;
            case NbtType.Long: tag.Value = r.ReadInt64(); break;
            case NbtType.Float: tag.Value = r.ReadSingle(); break;
            case NbtType.Double: tag.Value = r.ReadDouble(); break;
            case NbtType.ByteArray:
                {
                    var len = r.ReadInt32();
                    tag.Value = r.ReadBytes(len);
                    break;
                }
            case NbtType.String: tag.Value = ReadString(r); break;
            case NbtType.List:
                {
                    var elemType = (NbtType)r.ReadByte();
                    var count = r.ReadInt32();
                    tag.ListElementType = elemType;
                    for (var i = 0; i < count; i++)
                    {
                        var item = new NbtTag { Type = elemType };
                        if (elemType != NbtType.End) ReadPayload(r, item);
                        tag.Items.Add(item);
                    }
                    break;
                }
            case NbtType.Compound:
                {
                    while (true)
                    {
                        var child = ReadNamedTag(r);
                        if (child.Type == NbtType.End) break;
                        tag.Items.Add(child);
                    }
                    break;
                }
            case NbtType.IntArray:
                {
                    var len = r.ReadInt32();
                    var arr = new int[len];
                    for (var i = 0; i < len; i++) arr[i] = r.ReadInt32();
                    tag.Value = arr;
                    break;
                }
            case NbtType.LongArray:
                {
                    var len = r.ReadInt32();
                    var arr = new long[len];
                    for (var i = 0; i < len; i++) arr[i] = r.ReadInt64();
                    tag.Value = arr;
                    break;
                }
            case NbtType.End: break;
            default: throw new NotSupportedException($"Unknown NBT tag type {(byte)tag.Type}.");
        }
    }

    private static void WritePayload(BinaryWriter w, NbtTag tag)
    {
        switch (tag.Type)
        {
            case NbtType.Byte: w.Write((byte)tag.Value!); break;
            case NbtType.Short: w.Write((short)tag.Value!); break;
            case NbtType.Int: w.Write((int)tag.Value!); break;
            case NbtType.Long: w.Write((long)tag.Value!); break;
            case NbtType.Float: w.Write((float)tag.Value!); break;
            case NbtType.Double: w.Write((double)tag.Value!); break;
            case NbtType.ByteArray:
                {
                    var arr = (byte[])tag.Value!;
                    w.Write(arr.Length);
                    w.Write(arr);
                    break;
                }
            case NbtType.String: WriteString(w, (string)tag.Value!); break;
            case NbtType.List:
                {
                    w.Write((byte)tag.ListElementType);
                    w.Write(tag.Items.Count);
                    foreach (var item in tag.Items)
                        if (tag.ListElementType != NbtType.End) WritePayload(w, item);
                    break;
                }
            case NbtType.Compound:
                {
                    foreach (var child in tag.Items) WriteNamedTag(w, child);
                    w.Write((byte)NbtType.End);
                    break;
                }
            case NbtType.IntArray:
                {
                    var arr = (int[])tag.Value!;
                    w.Write(arr.Length);
                    foreach (var v in arr) w.Write(v);
                    break;
                }
            case NbtType.LongArray:
                {
                    var arr = (long[])tag.Value!;
                    w.Write(arr.Length);
                    foreach (var v in arr) w.Write(v);
                    break;
                }
            case NbtType.End: break;
        }
    }

    private static string ReadString(BinaryReader r)
    {
        var len = r.ReadUInt16();
        var bytes = r.ReadBytes(len);
        return Encoding.UTF8.GetString(bytes);
    }

    private static void WriteString(BinaryWriter w, string s)
    {
        var bytes = Encoding.UTF8.GetBytes(s);
        w.Write((ushort)bytes.Length);
        w.Write(bytes);
    }

    // ── Tree helpers ────────────────────────────────────────────

    public static NbtTag? Find(NbtTag compound, string name) =>
        compound.Items.Find(t => t.Name == name);

    public static void SetOrAdd(NbtTag compound, string name, NbtType type, object value)
    {
        var existing = Find(compound, name);
        if (existing != null) { existing.Value = value; return; }
        compound.Items.Add(new NbtTag { Type = type, Name = name, Value = value });
    }
}
