using System.Text;

namespace PboStudio.Core;

/// <summary>
/// Reads the new, complete lines of a log file that another process is still writing.
///
/// Both engines report a failed calculation by writing a line into a file this program polls
/// once a second. The poll read up to the end of the file and then remembered the file's length
/// as the place to resume from. If the engine was part-way through a line at that moment, the
/// first half was inspected and the second half never was: a marker such as "FATAL ERROR" split
/// across two polls matched neither, and the core passed. A missed failure is the one mistake a
/// stability test cannot afford, so nothing past the last newline is consumed until it has a
/// newline after it.
/// </summary>
internal static class LogTail
{
    /// <param name="offset">
    /// Where the previous call stopped. Advanced only over lines that were returned. A file that
    /// has shrunk since - truncated or recreated - is read again from the start.
    /// </param>
    public static IReadOnlyList<string> ReadNewLines(string path, ref long offset)
    {
        try
        {
            using var stream = new FileStream(path, FileMode.Open, FileAccess.Read, FileShare.ReadWrite);

            if (stream.Length < offset) offset = 0;
            if (stream.Length == offset) return [];

            stream.Seek(offset, SeekOrigin.Begin);

            var buffer = new byte[stream.Length - offset];
            int read = 0;
            while (read < buffer.Length)
            {
                int n = stream.Read(buffer, read, buffer.Length - read);
                if (n <= 0) break;
                read += n;
            }

            int lastNewline = Array.LastIndexOf(buffer, (byte)'\n', read - 1);
            if (lastNewline < 0) return [];   // a line is still being written

            offset += lastNewline + 1;

            return Encoding.UTF8.GetString(buffer, 0, lastNewline + 1)
                .Split('\n', StringSplitOptions.RemoveEmptyEntries)
                .Select(l => l.TrimEnd('\r'))
                .ToList();
        }
        catch (IOException)
        {
            // The engine holds the file briefly while writing; the next poll picks it up.
            return [];
        }
        catch (UnauthorizedAccessException)
        {
            return [];
        }
    }
}
