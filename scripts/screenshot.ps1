# Launches the sandbox, optionally sends keystrokes to set up a scene, waits
# for it to render, captures the window, and saves a PNG.
#
# Exists so visual milestones (rendering, plants, creatures) can be checked by
# looking at the actual output rather than trusting that code compiles and
# does something reasonable. Built for the overnight session extending the
# plan past M12 — see docs/future-directions.md and the build plan.
#
# Usage:
#   .\scripts\screenshot.ps1 -Out shot.png [-WaitSeconds 3] [-Args @('--scene','forest')]

param(
    [string]$Out = "$env:TEMP\pp_screenshot.png",
    [int]$WaitSeconds = 3,
    [string[]]$ExtraArgs = @(),
    [string]$Exe = ".\target\debug\pixel-physics.exe"
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class PPWin32 {
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
}
"@ -ErrorAction SilentlyContinue

$p = Start-Process -FilePath $Exe -ArgumentList $ExtraArgs -PassThru
Start-Sleep -Seconds $WaitSeconds
$p.Refresh()

if ($p.HasExited) {
    Write-Error "process exited early with code $($p.ExitCode)"
    exit 1
}
if ($p.MainWindowHandle -eq 0) {
    Write-Error "no window handle found"
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

[PPWin32]::SetForegroundWindow($p.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 400

$rect = New-Object PPWin32+RECT
[PPWin32]::GetWindowRect($p.MainWindowHandle, [ref]$rect) | Out-Null
$width = $rect.Right - $rect.Left
$height = $rect.Bottom - $rect.Top

if ($width -le 0 -or $height -le 0) {
    Write-Error "invalid window rect: $width x $height"
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

$bitmap = New-Object System.Drawing.Bitmap $width, $height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
$bitmap.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()

Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Write-Output $Out
