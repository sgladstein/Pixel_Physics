# Launches the sandbox, waits for it to render, captures the window, and
# saves a PNG.
#
# Exists so visual milestones (rendering, plants, creatures) can be checked by
# looking at the actual output rather than trusting that code compiles and
# does something reasonable. Built for the overnight session extending the
# plan past M12 — see docs/future-directions.md and the build plan.
#
# ONLY CAPTURES WINDOW CHROME ON THIS MACHINE, NOT THE RENDERED CANVAS.
# Found during M14: this build's DXGI/wgpu swapchain is not visible to
# Windows screen capture — neither plain BitBlt/CopyFromScreen nor
# PrintWindow(PW_RENDERFULLCONTENT) (this script tries the latter, falling
# back to the former) can see the client area, both returning solid black
# while the title bar and window frame capture correctly. Useful for
# confirming the window exists, is titled correctly, and the status line
# reports sane fps/material/chunk state — not for seeing what is actually
# drawn.
#
# For that, use the app's own PIXEL_PHYSICS_SCREENSHOT_AFTER_FRAMES=<n> env
# var instead (see save_framebuffer_png in src/main.rs): it dumps the
# in-memory framebuffer directly to
# %TEMP%\pixel_physics_screenshot.png after n rendered frames, with no OS
# capture involved, and is what actually worked for M14's fire-tint check.
# If a *different* machine's GPU/driver/present-mode combination allows
# normal screen capture to work after all, this script's window-chrome-only
# limitation may not apply there — worth re-testing before assuming it is
# still needed.
#
# Usage:
#   .\scripts\screenshot.ps1 -Out shot.png [-WaitSeconds 3]

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
    public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);
    // BitBlt/CopyFromScreen cannot see content from a DXGI/wgpu flip-model
    // swapchain — the compositor presents it via a path GDI screen capture
    // does not observe, so a plain CopyFromScreen silently returns whatever
    // was behind the window instead of an error. PrintWindow with
    // PW_RENDERFULLCONTENT (0x2) is the documented fix: it asks the window
    // itself to render into a supplied DC rather than reading the compositor's
    // framebuffer, which works for DirectX/wgpu content that BitBlt cannot see.
    [DllImport("user32.dll")]
    public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
}
"@ -ErrorAction SilentlyContinue

# HWND_TOPMOST then HWND_NOTOPMOST: forces the window briefly above every
# other window (including other topmost ones) rather than just "foreground,"
# which VS Code — itself often topmost-adjacent while debugging — was
# otherwise still compositing over in the captured region.
$HWND_TOPMOST = [IntPtr](-1)
$HWND_NOTOPMOST = [IntPtr](-2)
$SWP_NOMOVE = 0x2
$SWP_NOSIZE = 0x1
$SWP_SHOWWINDOW = 0x40

# -ArgumentList rejects an empty array outright (parameter validation error,
# not just a no-op), so it can only be passed when there is something in it.
if ($ExtraArgs.Count -gt 0) {
    $p = Start-Process -FilePath $Exe -ArgumentList $ExtraArgs -PassThru
} else {
    $p = Start-Process -FilePath $Exe -PassThru
}
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
[PPWin32]::SetWindowPos($p.MainWindowHandle, $HWND_TOPMOST, 0, 0, 0, 0, $SWP_NOMOVE -bor $SWP_NOSIZE -bor $SWP_SHOWWINDOW) | Out-Null
Start-Sleep -Milliseconds 500
[PPWin32]::SetWindowPos($p.MainWindowHandle, $HWND_NOTOPMOST, 0, 0, 0, 0, $SWP_NOMOVE -bor $SWP_NOSIZE -bor $SWP_SHOWWINDOW) | Out-Null
Start-Sleep -Milliseconds 300

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
$hdc = $graphics.GetHdc()
$ok = [PPWin32]::PrintWindow($p.MainWindowHandle, $hdc, 2) # PW_RENDERFULLCONTENT
$graphics.ReleaseHdc($hdc)
if (-not $ok) {
    Write-Warning "PrintWindow failed, falling back to CopyFromScreen (may capture black for DXGI content)"
    $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
}
$bitmap.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()

Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
Write-Output $Out
