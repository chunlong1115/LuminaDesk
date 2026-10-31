# S6 Win+D roundtrip x2 regression: structural check that the container stays
# attached (parent == wallpaper WorkerW), visible, and its rect is unchanged.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File s6-wind.ps1
$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class S6P {
  [DllImport("shcore.dll")] public static extern int SetProcessDpiAwareness(int v);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string cls, string title);
  [DllImport("user32.dll")] public static extern IntPtr FindWindow(string cls, string title);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  public static List<IntPtr> TopWorkerWs = new List<IntPtr>();
  public static List<IntPtr> Children = new List<IntPtr>();
  public static IntPtr Progman = IntPtr.Zero, Host = IntPtr.Zero, Container = IntPtr.Zero;
  static bool IsClass(IntPtr h, string c) { var sb = new StringBuilder(64); GetClassName(h, sb, 64); return sb.ToString() == c; }
  public static void Locate() {
    Progman = FindWindow("Progman", null);
    EnumWindows((h, l) => { if (IsClass(h, "WorkerW")) TopWorkerWs.Add(h); return true; }, IntPtr.Zero);
    foreach (var h in TopWorkerWs) {
      bool hasDef = FindWindowEx(h, IntPtr.Zero, "SHELLDLL_DefView", null) != IntPtr.Zero;
      bool parentIsProgman = GetParent(h) == Progman;
      if (!hasDef && parentIsProgman) Host = h;
    }
    if (Host != IntPtr.Zero) {
      EnumChildWindows(Host, (h, l) => { Children.Add(h); return true; }, IntPtr.Zero);
      if (Children.Count > 0) Container = Children[0];
    }
  }
}
'@
[S6P]::SetProcessDpiAwareness(2) | Out-Null
[S6P]::Locate()
if ([S6P]::Container -eq [IntPtr]::Zero) { Write-Host 'FAIL: container not found'; exit 1 }
$r0 = New-Object S6P+RECT
[S6P]::GetWindowRect([S6P]::Container, [ref]$r0) | Out-Null
Write-Host ('before: parent=0x{0:X} visible={1} rect=({2},{3})-({4},{5})' -f [long][S6P]::GetParent([S6P]::Container), [S6P]::IsWindowVisible([S6P]::Container), $r0.L, $r0.T, $r0.R, $r0.B)

$shell = New-Object -ComObject Shell.Application
1..2 | ForEach-Object {
  $shell.ToggleDesktop(); Start-Sleep -Seconds 2
  $shell.ToggleDesktop(); Start-Sleep -Seconds 2
}

$r1 = New-Object S6P+RECT
[S6P]::GetWindowRect([S6P]::Container, [ref]$r1) | Out-Null
$parent = [S6P]::GetParent([S6P]::Container)
$vis = [S6P]::IsWindowVisible([S6P]::Container)
$sameRect = ($r0.L -eq $r1.L -and $r0.T -eq $r1.T -and $r0.R -eq $r1.R -and $r0.B -eq $r1.B)
Write-Host ('after x2: parent=0x{0:X} visible={1} rect=({2},{3})-({4},{5}) sameRect={6}' -f [long]$parent, $vis, $r1.L, $r1.T, $r1.R, $r1.B, $sameRect)
if (($parent -eq [S6P]::Host) -and $vis -and $sameRect) { Write-Host 'PASS: S6 Win+D roundtrip' } else { Write-Host 'FAIL: container state changed'; exit 1 }
