using System.Runtime.InteropServices;

namespace PboStudio.Core;

internal static partial class Native
{
    [LibraryImport("kernel32.dll")]
    internal static partial nint GetCurrentThread();

    [LibraryImport("kernel32.dll", SetLastError = true)]
    internal static partial nuint SetThreadAffinityMask(nint hThread, nuint dwThreadAffinityMask);

    [LibraryImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static partial bool SetThreadPriority(nint hThread, int nPriority);

    internal const int THREAD_PRIORITY_HIGHEST = 2;

    [LibraryImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static partial bool GetLogicalProcessorInformationEx(
        int relationshipType, nint buffer, ref uint returnedLength);

    // Freezing and thawing the whole worker is how the load gets interrupted without killing it.
    // There is no documented Win32 equivalent; these ntdll entry points are what every debugger
    // and every tool doing this uses.
    [LibraryImport("ntdll.dll")]
    internal static partial int NtSuspendProcess(nint processHandle);

    [LibraryImport("ntdll.dll")]
    internal static partial int NtResumeProcess(nint processHandle);
}
