# S3 no-activation regression: expose desktop (Win+D), click a blank point inside
# the container's transparent area, verify the foreground window does NOT change
# (WS_EX_NOACTIVATE). Restores desktop afterwards.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File s3-noactivate.ps1
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class S3P {
  [DllImport("shcore.dll")] public static extern int SetProcessDpiAwareness(int v);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, EnumProc cb, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string cls, string title);
  [DllImport("user32.dll")] public static extern IntPtr FindWindow(string cls, string title);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
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
[S3P]::SetProcessDpiAwareness(2) | Out-Null
[S3P]::Locate()
Write-Host ('Container=0x{0:X}' -f [long][S3P]::Container)
if ([S3P]::Container -eq [IntPtr]::Zero) { Write-Host 'FAIL: container not found'; exit 1 }
$r = New-Object S3P+RECT
[S3P]::GetWindowRect([S3P]::Container, [ref]$r) | Out-Null
Write-Host ('container rect=({0},{1})-({2},{3})' -f $r.L, $r.T, $r.R, $r.B)

# Probe candidate points in the container's lower-right quadrant for a wallpaper-blank spot
function IsWallpaper($x, $y) {
  $b = New-Object System.Drawing.Bitmap 1, 1
  $g = [System.Drawing.Graphics]::FromImage($b)
  $g.CopyFromScreen($x, $y, 0, 0, $b.Size)
  $g.Dispose()
  $c = $b.GetPixel(0, 0)
  $b.Dispose()
  return ($c.R -ge 12 -and $c.R -le 36 -and $c.G -ge 12 -and $c.G -le 38 -and $c.B -ge 12 -and $c.B -le 38)
}
$cx = [int](($r.L + $r.R) / 2); $cy = [int](($r.T + $r.B) / 2)
$candidates = @()
for ($dy = 0; $dy -lt ($r.B - $cy); $dy += 30) {
  for ($dx = 0; $dx -lt ($r.R - $cx); $dx += 30) {
    $candidates += , @(($cx + $dx), ($cy + $dy))
  }
}
$click = $null
foreach ($pt in $candidates) {
  if (IsWallpaper $pt[0] $pt[1]) { $click = $pt; break }
}
if ($null -eq $click) { Write-Host 'FAIL: no blank point found in container area'; exit 1 }
Write-Host ('click point=({0},{1})' -f $click[0], $click[1])

$shell = New-Object -ComObject Shell.Application
$shell.ToggleDesktop()
Start-Sleep -Milliseconds 1500
$fgBefore = [S3P]::GetForegroundWindow()
[S3P]::SetCursorPos($click[0], $click[1]) | Out-Null
Start-Sleep -Milliseconds 300
[S3P]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)  # LEFTDOWN
[S3P]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)  # LEFTUP
Start-Sleep -Milliseconds 500
$fgAfter = [S3P]::GetForegroundWindow()
$unchanged = ($fgBefore -eq $fgAfter)
Write-Host ('foreground before=0x{0:X} after=0x{1:X} unchanged={2}' -f [long]$fgBefore, [long]$fgAfter, $unchanged)
$shell.ToggleDesktop()
Start-Sleep -Milliseconds 1500
if ($unchanged) { Write-Host 'PASS: S3 no-activation' } else { Write-Host 'FAIL: foreground changed' ; exit 1 }
