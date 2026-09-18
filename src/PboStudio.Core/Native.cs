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

    // Flashing the taskbar button, for a failure found while the window is behind something
    // else. A run lasts hours; nobody watches it, and a beep is missed by anyone wearing
    // headphones or sitting in another room. CoreCycler calls this flashOnError.
    [StructLayout(LayoutKind.Sequential)]
    internal struct FLASHWINFO
    {
        public uint cbSize;
        public nint hwnd;
        public uint dwFlags;
        public uint uCount;
        public uint dwTimeout;
    }

    internal const uint FLASHW_TRAY = 0x00000002;
    internal const uint FLASHW_TIMERNOFG = 0x0000000C;

    [LibraryImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    internal static partial bool FlashWindowEx(ref FLASHWINFO pwfi);
}

/// <summary>
/// Getting the reader's attention when the window is not the one in front. Public because the
/// P/Invoke surface it sits on is deliberately internal to this assembly.
/// </summary>
public static class WindowAlert
{
    /// <summary>
    /// Flashes until the window is brought to the front. Silently does nothing off Windows or
    /// when the handle is gone; a missed notification is never worth an exception.
    /// </summary>
    public static void FlashTaskbar(nint windowHandle)
    {
        if (windowHandle == nint.Zero || !OperatingSystem.IsWindows()) return;

        try
        {
            var info = new Native.FLASHWINFO
            {
                cbSize = (uint)Marshal.SizeOf<Native.FLASHWINFO>(),
                hwnd = windowHandle,
                dwFlags = Native.FLASHW_TRAY | Native.FLASHW_TIMERNOFG,
                uCount = uint.MaxValue,
                dwTimeout = 0,
            };

            Native.FlashWindowEx(ref info);
        }
        catch { }
    }
}
