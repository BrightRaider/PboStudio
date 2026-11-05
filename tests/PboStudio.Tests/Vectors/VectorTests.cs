using System.Reflection;
using System.Text.Json;
using Xunit;

namespace PboStudio.Tests.Vectors;

/// <summary>
/// Language-neutral test vectors (ADR-0002). Each file under spec/vectors holds cases of inputs and
/// expected outputs for one piece of pure logic. The same files are run by the Rust rebuild, so the two
/// implementations are held to one truth.
///
/// A case is "parity" when the C# program does this today and Rust must do the same, and "new" when only
/// the Rust rebuild has to. C# runs the parity cases here; it does not run the new ones.
/// </summary>
public class VectorTests
{
    private static readonly Lazy<IReadOnlyList<VectorCase>> Loaded = new(VectorFiles.Load);

    public static TheoryData<string> Ids()
    {
        var data = new TheoryData<string>();
        foreach (var c in Loaded.Value) data.Add(c.Id);
        return data;
    }

    [Theory]
    [MemberData(nameof(Ids))]
    public void CaseHoldsInCSharp(string id)
    {
        var vector = Loaded.Value.Single(c => c.Id == id);

        // Behaviour only the rebuild has. Nothing to run here, and that is the point of the kind.
        if (vector.Kind == "new") return;

        Assert.True(VectorAdapters.Map.TryGetValue(vector.Key, out var adapter),
            $"no C# adapter for '{vector.Key}'. A parity case needs one, or it is not parity.");

        object? actual = adapter!(vector.Input);
        JsonElement actualJson = JsonSerializer.SerializeToElement(actual);

        Assert.True(VectorJson.Equal(vector.Expected, actualJson),
            $"{vector.Id}\n  expected: {vector.Expected.GetRawText()}\n  actual  : {actualJson.GetRawText()}");
    }

    [Fact]
    public void EveryCaseHasAUniqueName()
    {
        var duplicates = Loaded.Value.GroupBy(c => c.Id).Where(g => g.Count() > 1).Select(g => g.Key).ToList();
        Assert.Empty(duplicates);
    }

    [Fact]
    public void ThereAreVectorFilesAndEachHasCases()
    {
        Assert.NotEmpty(Loaded.Value);
        foreach (var file in VectorFiles.Files())
        {
            using var doc = JsonDocument.Parse(File.ReadAllText(file));
            Assert.True(doc.RootElement.GetProperty("cases").GetArrayLength() > 0, $"{Path.GetFileName(file)} has no cases");
            Assert.Equal(1, doc.RootElement.GetProperty("schema").GetInt32());
        }
    }

    [Fact]
    public void EveryAdapterIsUsedByAtLeastOneCase()
    {
        var used = Loaded.Value.Select(c => c.Key).ToHashSet();
        var unused = VectorAdapters.Map.Keys.Where(k => !used.Contains(k)).ToList();
        Assert.Empty(unused);
    }
}

public sealed record VectorCase(string File, string Area, string Kind, string Function, string Name, JsonElement Input, JsonElement Expected)
{
    public string Key => $"{Area}/{Function}";
    public string Id => $"{Area}/{Function}: {Name}";
}

public static class VectorFiles
{
    /// <summary>The spec/vectors folder, found by walking up from the test binaries.</summary>
    public static string Dir()
    {
        for (var dir = new DirectoryInfo(AppContext.BaseDirectory); dir is not null; dir = dir.Parent)
        {
            string candidate = Path.Combine(dir.FullName, "spec", "vectors");
            if (Directory.Exists(candidate)) return candidate;
        }
        throw new DirectoryNotFoundException("spec/vectors was not found above " + AppContext.BaseDirectory);
    }

    public static IEnumerable<string> Files() => Directory.GetFiles(Dir(), "*.json").OrderBy(f => f, StringComparer.Ordinal);

    public static IReadOnlyList<VectorCase> Load()
    {
        var all = new List<VectorCase>();
        foreach (string file in Files())
        {
            using var doc = JsonDocument.Parse(File.ReadAllText(file));
            var root = doc.RootElement;
            string area = root.GetProperty("area").GetString()!;
            string defaultKind = root.GetProperty("kind").GetString()!;

            foreach (var c in root.GetProperty("cases").EnumerateArray())
            {
                string kind = c.TryGetProperty("kind", out var k) ? k.GetString()! : defaultKind;
                all.Add(new VectorCase(
                    Path.GetFileName(file),
                    area,
                    kind,
                    c.GetProperty("function").GetString()!,
                    c.GetProperty("name").GetString()!,
                    c.GetProperty("input").Clone(),
                    c.GetProperty("expected").Clone()));
            }
        }
        return all;
    }
}

[AttributeUsage(AttributeTargets.Method)]
public sealed class VectorAttribute(string key) : Attribute
{
    public string Key { get; } = key;
}

/// <summary>Finds every method marked [Vector("area/function")] and offers it by that key.</summary>
public static class VectorAdapters
{
    public static readonly IReadOnlyDictionary<string, Func<JsonElement, object?>> Map = Build();

    private static Dictionary<string, Func<JsonElement, object?>> Build()
    {
        var map = new Dictionary<string, Func<JsonElement, object?>>();
        foreach (var type in typeof(VectorAdapters).Assembly.GetTypes())
        {
            foreach (var method in type.GetMethods(BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic))
            {
                var attr = method.GetCustomAttribute<VectorAttribute>();
                if (attr is null) continue;
                map.Add(attr.Key, input => method.Invoke(null, [input]));
            }
        }
        return map;
    }
}

/// <summary>Reading inputs out of a case, without ceremony.</summary>
public static class VectorInput
{
    public static int Int(this JsonElement e, string name) => e.GetProperty(name).GetInt32();
    public static long Long(this JsonElement e, string name) => e.GetProperty(name).GetInt64();
    public static double Dbl(this JsonElement e, string name) => e.GetProperty(name).GetDouble();
    public static string Str(this JsonElement e, string name) => e.GetProperty(name).GetString()!;
    public static bool Bool(this JsonElement e, string name) => e.GetProperty(name).GetBoolean();

    public static int? IntOrNull(this JsonElement e, string name) =>
        e.TryGetProperty(name, out var p) && p.ValueKind != JsonValueKind.Null ? p.GetInt32() : null;

    public static string? StrOrNull(this JsonElement e, string name) =>
        e.TryGetProperty(name, out var p) && p.ValueKind != JsonValueKind.Null ? p.GetString() : null;

    public static int[] Ints(this JsonElement e, string name) =>
        e.GetProperty(name).EnumerateArray().Select(x => x.GetInt32()).ToArray();

    public static double[] Dbls(this JsonElement e, string name) =>
        e.GetProperty(name).EnumerateArray().Select(x => x.GetDouble()).ToArray();

    public static string[] Strs(this JsonElement e, string name) =>
        e.GetProperty(name).EnumerateArray().Select(x => x.GetString()!).ToArray();
}

/// <summary>Equality of two JSON values: numbers by value, objects without regard to key order.</summary>
public static class VectorJson
{
    public static bool Equal(JsonElement a, JsonElement b)
    {
        if (a.ValueKind != b.ValueKind && !(IsBool(a) && IsBool(b)))
            return false;

        switch (a.ValueKind)
        {
            case JsonValueKind.Number:
                return Math.Abs(a.GetDouble() - b.GetDouble()) <= 1e-6 * Math.Max(1.0, Math.Abs(a.GetDouble()));
            case JsonValueKind.String:
                return a.GetString() == b.GetString();
            case JsonValueKind.True:
            case JsonValueKind.False:
                return a.GetBoolean() == b.GetBoolean();
            case JsonValueKind.Null:
                return true;
            case JsonValueKind.Array:
                var xs = a.EnumerateArray().ToList();
                var ys = b.EnumerateArray().ToList();
                return xs.Count == ys.Count && xs.Zip(ys).All(p => Equal(p.First, p.Second));
            case JsonValueKind.Object:
                var pa = a.EnumerateObject().ToDictionary(p => p.Name, p => p.Value);
                var pb = b.EnumerateObject().ToDictionary(p => p.Name, p => p.Value);
                return pa.Count == pb.Count && pa.All(kv => pb.TryGetValue(kv.Key, out var other) && Equal(kv.Value, other));
            default:
                return true;
        }
    }

    private static bool IsBool(JsonElement e) => e.ValueKind is JsonValueKind.True or JsonValueKind.False;
}
