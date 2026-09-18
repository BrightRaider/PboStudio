using System.Text.RegularExpressions;
using PboStudio.Core;
using Xunit;

namespace PboStudio.Tests;

/// <summary>
/// The checks that were being done by opening the program and looking at it.
///
/// Every one of these corresponds to something that actually shipped: two Cancel buttons on one
/// panel, a recommendation card repeating the picker underneath it with its own start button
/// and its own copy of the runtime, a translation key that resolved to nothing. Reading the
/// markup is not as good as looking at the window, but it runs in a second and it never gets
/// bored.
/// </summary>
public class WindowWiringTests
{
    private static string RepoRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null && !File.Exists(Path.Combine(dir.FullName, "PboStudio.sln")))
            dir = dir.Parent;

        Assert.NotNull(dir);
        return dir!.FullName;
    }

    private static string Read(string relative) =>
        File.ReadAllText(Path.Combine(RepoRoot(), relative));

    private static string Xaml => Read(Path.Combine("src", "PboStudio.App", "MainWindow.axaml"));
    private static string Code => Read(Path.Combine("src", "PboStudio.App", "MainWindow.axaml.cs"));

    private static List<string> Names(string xaml) =>
        [.. Regex.Matches(xaml, @"x:Name=""([A-Za-z0-9_]+)""").Select(m => m.Groups[1].Value)];

    [Fact]
    public void NoControlIsNamedTwice()
    {
        var names = Names(Xaml);

        Assert.Empty(names.GroupBy(n => n).Where(g => g.Count() > 1).Select(g => g.Key));
    }

    /// <summary>A handler named in the markup with no method behind it throws when it fires.</summary>
    [Fact]
    public void EveryEventHandlerHasAMethod()
    {
        string code = Code;

        var handlers = Regex.Matches(Xaml,
                @"(?:Click|IsCheckedChanged|SelectionChanged|ValueChanged|TextChanged|KeyDown)=""([A-Za-z0-9_]+)""")
            .Select(m => m.Groups[1].Value)
            .Distinct();

        Assert.Empty(handlers.Where(h => !code.Contains($"void {h}(")));
    }

    /// <summary>
    /// A key with no entry behind it renders as the key itself, in both languages. Compared
    /// against the definitions rather than the output, because a handful of entries — "Passes",
    /// "Close", "STATUS" — legitimately translate to their own key in English.
    /// </summary>
    [Fact]
    public void EveryTranslationKeyTheWindowAsksForExists()
    {
        string code = Code;
        var defined = Regex
            .Matches(Read(Path.Combine("src", "PboStudio.Core", "LocalizationService.cs")),
                @"^\s*""([A-Za-z0-9_]+)"" =>", RegexOptions.Multiline)
            .Select(m => m.Groups[1].Value)
            .ToHashSet();

        var asked = Regex.Matches(code, @"LocalizationService\.Get\(""([^""]+)""\)")
            .Select(m => m.Groups[1].Value)
            .Concat(Regex.Matches(code, @"Tip\([A-Za-z0-9_.]+, ""([^""]+)""\)").Select(m => m.Groups[1].Value))
            .Distinct();

        Assert.Empty(asked.Where(k => !defined.Contains(k)));
    }

    // ── the three tabs ────────────────────────────────────────────

    private static Dictionary<string, List<string>> ByTab()
    {
        string xaml = Xaml;
        (string Tab, int At)[] marks =
        [
            ("Auto", xaml.IndexOf("x:Name=\"RightTabAutoPanel\"", StringComparison.Ordinal)),
            ("Tests", xaml.IndexOf("x:Name=\"RightTabTestsPanel\"", StringComparison.Ordinal)),
            ("Advanced", xaml.IndexOf("x:Name=\"RightTabAdvancedPanel\"", StringComparison.Ordinal)),
            ("End", xaml.IndexOf("BOTTOM: telemetry / log", StringComparison.Ordinal)),
        ];

        Assert.All(marks, m => Assert.True(m.At > 0, $"{m.Tab} not found in the markup"));

        var result = new Dictionary<string, List<string>>();
        for (int i = 0; i < 3; i++)
            result[marks[i].Tab] = [.. Names(xaml[marks[i].At..marks[i + 1].At]).Skip(1)];

        return result;
    }

    [Fact]
    public void TheThreeTabsShareNoControls()
    {
        var tabs = ByTab();

        Assert.Empty(tabs["Auto"].Intersect(tabs["Tests"]));
        Assert.Empty(tabs["Auto"].Intersect(tabs["Advanced"]));
        Assert.Empty(tabs["Tests"].Intersect(tabs["Advanced"]));
    }

    /// <summary>
    /// The Tests tab carried a recommendation card that answered the same question as the
    /// picker below it, with a second start button and a second copy of the runtime that was
    /// refreshed on different events and drifted to eight times the other one.
    /// </summary>
    [Fact]
    public void TheTestsTabHoldsOneProfilePickerAndNothingThatCompetesWithIt()
    {
        var tests = ByTab()["Tests"];

        Assert.Contains("ProfileBox", tests);
        Assert.DoesNotContain(tests, n => n.StartsWith("NextStep", StringComparison.Ordinal));
        Assert.DoesNotContain(tests, n => n.Contains("Start", StringComparison.Ordinal));

        // Small enough to read without scrolling.
        Assert.True(tests.Count <= 12, $"the Tests tab has grown to {tests.Count} controls");
    }

    /// <summary>
    /// Auto starts and stops a campaign with its own button. The footer's single-run button is
    /// hidden there — having both on screen is what produced two Cancel buttons.
    /// </summary>
    [Fact]
    public void OnlyTheAutoTabCarriesItsOwnStartButton()
    {
        var tabs = ByTab();

        Assert.Contains("AutoStartButton", tabs["Auto"]);
        Assert.DoesNotContain(tabs["Tests"], n => n.Contains("Start", StringComparison.Ordinal));
        Assert.DoesNotContain(tabs["Advanced"], n => n.Contains("Start", StringComparison.Ordinal));
    }

    /// <summary>
    /// The step search is an override, like every other knob. It sat on the Tests tab one tab
    /// away from a tab called Auto, which is two different things sharing a word.
    /// </summary>
    [Fact]
    public void TheSearchAndItsSettingsLiveOnTheAdvancedTab()
    {
        var advanced = ByTab()["Advanced"];

        foreach (var control in new[] { "AutoTunerModeBox", "GuardbandBox", "ResetKnowledgeButton" })
            Assert.Contains(control, advanced);
    }

    /// <summary>Nothing left pointing at a control that was taken out of the markup.</summary>
    [Fact]
    public void TheCodeDoesNotReferToControlsThatNoLongerExist()
    {
        string code = Code;
        var names = Names(Xaml).ToHashSet();

        foreach (var gone in new[]
        {
            "NextStepCard", "NextStepHeadline", "ApplyNextStepButton",
            "ShowAllProfilesButton", "EngineKnobsPanel", "OpenAdvancedLink", "TestControlsTitle",
        })
        {
            Assert.DoesNotContain(gone, names);
            Assert.False(code.Contains(gone + "."), $"{gone} is gone from the markup but still used in code");
        }
    }
}
