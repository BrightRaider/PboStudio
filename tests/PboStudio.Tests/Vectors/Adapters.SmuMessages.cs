using System.Text.Json;
using ZenStates.Core;
using ZenStates.Core.SMUSettings;

namespace PboStudio.Tests.Vectors;

internal static class SmuMessagesAdapters
{
    private static SMU For(string generation) => generation switch
    {
        "Zen3" => new Zen3Settings(),
        "Zen4" => new Zen4Settings(),
        "Zen5" => new Zen5Settings(),
        _ => throw new ArgumentException(generation),
    };

    [Vector("smu-messages/getMargin")]
    public static object? GetMargin(JsonElement i) => (long)For(i.Str("generation")).Rsmu.SMU_MSG_GetDldoPsmMargin;

    // The program sends the set message through MP1 when the table has one there, which Zen 3, 4 and 5 do.
    [Vector("smu-messages/setMargin")]
    public static object? SetMargin(JsonElement i) => (long)For(i.Str("generation")).Mp1Smu.SMU_MSG_SetDldoPsmMargin;
}
