using System.Diagnostics;
using System.Runtime.InteropServices;

namespace PboStudio.Core;

/// <summary>
/// Starts a process that is pinned to a set of processors before it executes its first
/// instruction.
///
/// Setting affinity after Process.Start is a race, and one that both stress engines lose:
/// they inspect the machine and spread their workers across every processor within
/// milliseconds. y-cruncher in particular reads the affinity mask once at startup and sizes
/// itself accordingly, so a mask applied afterwards leaves it running on cores it already
/// claimed. Creating the process suspended closes the window entirely - by the time any of
/// its code runs, the mask is already in place.
/// </summary>
internal static class PinnedProcess
{
    private const uint CREATE_SUSPENDED = 0x00000004;
    private const uint CREATE_NO_WINDOW = 0x08000000;

    /// <param name="priority">
    /// What the stress process runs at. CoreCycler exposes this as
    /// <c>stressTestProgramPriority</c>; the reason to lower it is that a test pinned to one
    /// core at normal priority still competes with whatever the machine is doing on that core,
    /// and the reason to raise it is to stop anything else interfering with the measurement.
    /// </param>
    public static Process Start(
        string exePath, string arguments, string workingDirectory, nuint affinityMask,
        CoreJail? jail = null, ProcessPriorityClass priority = ProcessPriorityClass.Normal)
    {
        var startupInfo = new STARTUPINFO { cb = Marshal.SizeOf<STARTUPINFO>() };

        // CreateProcess may modify the command line buffer, so it cannot be a literal.
        string commandLine = $"\"{exePath}\" {arguments}";

        bool ok = CreateProcess(
            lpApplicationName: exePath,
            lpCommandLine: new System.Text.StringBuilder(commandLine),
            lpProcessAttributes: nint.Zero,
            lpThreadAttributes: nint.Zero,
            bInheritHandles: false,
            dwCreationFlags: CREATE_SUSPENDED | CREATE_NO_WINDOW,
            lpEnvironment: nint.Zero,
            lpCurrentDirectory: workingDirectory,
            lpStartupInfo: ref startupInfo,
            lpProcessInformation: out PROCESS_INFORMATION info);

        if (!ok)
            throw new InvalidOperationException(
                $"Could not start {Path.GetFileName(exePath)}: {Marshal.GetLastWin32Error()}");

        try
        {
            if (!SetProcessAffinityMask(info.hProcess, affinityMask))
                throw new InvalidOperationException(
                    $"Could not pin process: {Marshal.GetLastWin32Error()}");

            var process = Process.GetProcessById(info.dwProcessId);

            // Assigned while still suspended, so the limit is in force from the first instruction.
            jail?.Add(process);

            // Set before the first instruction runs, like the affinity above it. A refused
            // priority is not worth aborting a run over.
            if (priority != ProcessPriorityClass.Normal)
            {
                try { process.PriorityClass = priority; } catch { }
            }

            if (ResumeThread(info.hThread) == unchecked((uint)-1))
                throw new InvalidOperationException(
                    $"Could not resume process: {Marshal.GetLastWin32Error()}");

            return process;
        }
        catch
        {
            TerminateProcess(info.hProcess, 1);
            throw;
        }
        finally
        {
            CloseHandle(info.hThread);
            CloseHandle(info.hProcess);
        }
    }

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CreateProcess(
        string? lpApplicationName,
        System.Text.StringBuilder lpCommandLine,
        nint lpProcessAttributes,
        nint lpThreadAttributes,
        [MarshalAs(UnmanagedType.Bool)] bool bInheritHandles,
        uint dwCreationFlags,
        nint lpEnvironment,
        string? lpCurrentDirectory,
        ref STARTUPINFO lpStartupInfo,
        out PROCESS_INFORMATION lpProcessInformation);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetProcessAffinityMask(nint hProcess, nuint dwProcessAffinityMask);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern uint ResumeThread(nint hThread);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool TerminateProcess(nint hProcess, uint uExitCode);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool CloseHandle(nint hObject);

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct STARTUPINFO
    {
        public int cb;
        public string? lpReserved, lpDesktop, lpTitle;
        public int dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars, dwFillAttribute, dwFlags;
        public short wShowWindow, cbReserved2;
        public nint lpReserved2, hStdInput, hStdOutput, hStdError;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct PROCESS_INFORMATION
    {
        public nint hProcess, hThread;
        public int dwProcessId, dwThreadId;
    }
}
