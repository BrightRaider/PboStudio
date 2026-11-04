using System.Diagnostics;

namespace PboStudio.Core;

/// <summary>Where the Windows tools this program starts actually live.</summary>
public static class SystemTools
{
    private static string System32 => Environment.GetFolderPath(Environment.SpecialFolder.System);

    /// <summary>
    /// A tool by its full path in the system folder.
    /// <para>
    /// Started by bare name, <c>powershell.exe</c> and <c>schtasks.exe</c> are looked for in the
    /// program's own folder first. This program runs as administrator, so whatever sits next to it
    /// with the right name would be started with those rights. The system folder is not writable by
    /// an ordinary user. Falls back to the bare name only where the file is not there.
    /// </para>
    /// </summary>
    public static string Path(string relativeToSystem32)
    {
        string full = System.IO.Path.Combine(System32, relativeToSystem32);
        return File.Exists(full) ? full : System.IO.Path.GetFileName(relativeToSystem32);
    }

    public static string PowerShell => Path(@"WindowsPowerShell\v1.0\powershell.exe");
    public static string SchTasks => Path("schtasks.exe");
    public static string Shutdown => Path("shutdown.exe");
    public static string RunDll32 => Path("rundll32.exe");
}

/// <summary>
/// Runs a program to completion and returns what it said, with a timeout that actually works.
///
/// The earlier version read standard output to the end <em>before</em> waiting with the timeout.
/// Reading to the end blocks until the process closes its output, so a process that hung never got
/// as far as the timeout: a stuck PowerShell froze the call for good, and with it the thread that
/// asked - which was the one starting a run. Both streams are read in the background and the wait
/// is the only thing the caller blocks on.
/// </summary>
internal static class ProcessRunner
{
    /// <returns>
    /// Whether the process exited with 0 within the timeout, and its standard error when it wrote any
    /// (that is where the useful sentence is) otherwise its standard output.
    /// </returns>
    public static (bool Ok, string Output) Run(string fileName, string arguments, TimeSpan timeout)
    {
        try
        {
            var psi = new ProcessStartInfo(fileName)
            {
                Arguments = arguments,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true,
            };

            using var process = Process.Start(psi);
            if (process is null) return (false, $"could not start {System.IO.Path.GetFileName(fileName)}");

            var stdout = process.StandardOutput.ReadToEndAsync();
            var stderr = process.StandardError.ReadToEndAsync();

            if (!process.WaitForExit((int)Math.Min(timeout.TotalMilliseconds, int.MaxValue)))
            {
                try { process.Kill(entireProcessTree: true); } catch { }
                return (false, "timed out");
            }

            // The process has exited, so the readers finish promptly; a stuck grandchild holding the
            // pipe open must not hold the caller as well.
            Task.WaitAll([stdout, stderr], TimeSpan.FromSeconds(5));

            string err = stderr.IsCompletedSuccessfully ? stderr.Result : "";
            string output = stdout.IsCompletedSuccessfully ? stdout.Result : "";

            return (process.ExitCode == 0, string.IsNullOrWhiteSpace(err) ? output : err);
        }
        catch (Exception ex)
        {
            return (false, ex.Message);
        }
    }
}
