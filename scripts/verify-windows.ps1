#Requires -Version 5.1
[CmdletBinding()]
param(
    [string]$Executable
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSEdition -ne 'Desktop' -or -not [Environment]::UserInteractive) {
    throw 'Run this check in Windows PowerShell 5.1 on an interactive Windows desktop.'
}

Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class PomotroidWindowsCheck {
    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr window);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string windowClass, string name);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool PostMessage(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);

    public static bool IsOwnedBy(IntPtr window, uint process) {
        uint owner;
        return GetWindowThreadProcessId(window, out owner) != 0 && owner == process;
    }

    public static void PostToOwner(IntPtr window, uint process, uint message, int wparam, int lparam) {
        if (!IsOwnedBy(window, process)) throw new InvalidOperationException("Window owner changed.");
        if (!PostMessage(window, message, (IntPtr)wparam, (IntPtr)lparam)) {
            throw new System.ComponentModel.Win32Exception();
        }
    }

    public static IntPtr FindTrayWindow(uint process) {
        IntPtr window = IntPtr.Zero;
        while ((window = FindWindowEx(IntPtr.Zero, window, "tray_icon_app", null)) != IntPtr.Zero) {
            if (IsOwnedBy(window, process)) return window;
        }
        return IntPtr.Zero;
    }

    public static IntPtr FindVisibleAppWindow(uint process) {
        IntPtr window = IntPtr.Zero;
        while ((window = FindWindowEx(IntPtr.Zero, window, null, "Pomotroid")) != IntPtr.Zero) {
            if (IsOwnedBy(window, process) && IsWindowVisible(window)) return window;
        }
        return IntPtr.Zero;
    }
}
'@

function Wait-For([string]$Description, [scriptblock]$Read, [int]$Seconds = 10) {
    $time = [Diagnostics.Stopwatch]::StartNew()
    do {
        if ($app.HasExited) { throw "Pomotroid exited while waiting for $Description." }
        $result = & $Read
        if ($result) { return $result }
        Start-Sleep -Milliseconds 150
    } while ($time.Elapsed.TotalSeconds -lt $Seconds)
    throw "Timed out waiting for $Description."
}

function Get-TimerServerOwner {
    $connections = & "$env:SystemRoot\System32\netstat.exe" -ano -p tcp
    if ($LASTEXITCODE -ne 0) { throw 'Could not read local TCP listener ownership.' }
    foreach ($connection in $connections) {
        if ($connection -match '^\s*TCP\s+\S+:1314\s+0\.0\.0\.0:0\s+\S+\s+(\d+)\s*$') {
            [int]$Matches[1]
        }
    }
}

function Get-VisibleAppWindow {
    $handle = [PomotroidWindowsCheck]::FindVisibleAppWindow($app.Id)
    if ($handle -ne [IntPtr]::Zero) { $handle }
}

function Find-UiElement([string]$Name) {
    try {
        $root = [Windows.Automation.AutomationElement]::FromHandle($window)
        $condition = [Windows.Automation.PropertyCondition]::new(
            [Windows.Automation.AutomationElement]::NameProperty, $Name)
        $root.FindFirst([Windows.Automation.TreeScope]::Descendants, $condition)
    } catch [Windows.Automation.ElementNotAvailableException] {
        return
    }
}

function Invoke-Button([string]$Name) {
    $element = Wait-For "the '$Name' button" { Find-UiElement $Name }
    $invoke = $element.GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern)
    $invoke.Invoke()
}

function Assert-AppHidden {
    if ($app.HasExited) { throw 'Pomotroid quit during the hidden break.' }
    if (Get-VisibleAppWindow) { throw "The Pomotroid window reopened after $([Math]::Round($breakElapsed.Elapsed.TotalSeconds, 1)) seconds." }
}

function Read-TimerEvent([string]$EventName, [string]$State, [int]$Seconds = 8, [switch]$WhileHidden) {
    $cancel = [Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds($Seconds))
    $payload = [IO.MemoryStream]::new()
    try {
        $buffer = [byte[]]::new(1024)
        do {
            $receive = $socket.ReceiveAsync([ArraySegment[byte]]::new($buffer), $cancel.Token)
            if ($WhileHidden) {
                while (-not $receive.IsCompleted) {
                    Assert-AppHidden
                    Start-Sleep -Milliseconds 200
                }
                Assert-AppHidden
            }
            $frame = $receive.GetAwaiter().GetResult()
            if ($frame.MessageType -ne [Net.WebSockets.WebSocketMessageType]::Text) {
                throw 'Expected a text timer event.'
            }
            $payload.Write($buffer, 0, $frame.Count)
            if ($payload.Length -gt 4096) { throw 'The timer event exceeded 4096 bytes.' }
        } until ($frame.EndOfMessage)
        $message = [Text.Encoding]::UTF8.GetString($payload.ToArray()) | ConvertFrom-Json
    } catch {
        if ($cancel.IsCancellationRequested) { throw "Timed out waiting for $EventName with state '$State'." }
        throw
    } finally {
        $payload.Dispose()
        $cancel.Dispose()
    }
    if ($message.event -cne $EventName -or $message.data.state -cne $State) {
        throw "Expected $EventName with state '$State'. Received $($message | ConvertTo-Json -Compress)."
    }
}

function Invoke-TrayIcon {
    $tray = Wait-For 'the native tray window' {
        $handle = [PomotroidWindowsCheck]::FindTrayWindow($app.Id)
        if ($handle -ne [IntPtr]::Zero) { $handle }
    }
    # tray-icon 0.21 posts this callback for a left-button release on its first icon.
    [PomotroidWindowsCheck]::PostToOwner($tray, $app.Id, 6002, 0, 0x0202)
}

function Restore-FromTray {
    Invoke-TrayIcon
    Wait-For 'the window restored from the tray' { Get-VisibleAppWindow }
}

if (-not $Executable) { $Executable = Join-Path $PSScriptRoot '..\target\debug\pomotroid.exe' }
$Executable = (Get-Item -LiteralPath $Executable).FullName
if (Get-TimerServerOwner) { throw 'TCP port 1314 is in use. Close the listener before running this check.' }

$profilePath = [IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) "pomotroid-verify-$([guid]::NewGuid())"))
$app = $null
$socket = $null
$errors = $null
try {
    [void][IO.Directory]::CreateDirectory($profilePath)
    @{
        timeWork = 2
        timeShortBreak = 1
        autoStartWorkTimer = $false
        autoStartBreakTimer = $false
        minToTray = $true
        minToTrayOnClose = $true
        notifications = $false
        tickSoundsDuringBreak = $false
        volume = 0
        globalShortcuts = @{
            'call-timer-toggle' = ''
            'call-timer-reset' = ''
            'call-timer-skip' = ''
        }
    } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $profilePath 'user-preferences.json') -Encoding UTF8

    $launch = [Diagnostics.ProcessStartInfo]::new($Executable)
    $launch.UseShellExecute = $false
    $launch.CreateNoWindow = $true
    $launch.RedirectStandardError = $true
    $launch.EnvironmentVariables['POMOTROID_HOME'] = $profilePath
    $app = [Diagnostics.Process]::Start($launch)
    $errors = $app.StandardError.ReadToEndAsync()

    $window = Wait-For 'the new Pomotroid window' { Get-VisibleAppWindow } 20
    [void](Wait-For 'port 1314 owned by the new Pomotroid process' { (Get-TimerServerOwner) -contains $app.Id })

    $socket = [Net.WebSockets.ClientWebSocket]::new()
    $cancel = [Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(8))
    try {
        [void]$socket.ConnectAsync([Uri]'ws://127.0.0.1:1314', $cancel.Token).GetAwaiter().GetResult()
        $request = [Text.Encoding]::UTF8.GetBytes('{"event":"getState"}')
        [void]$socket.SendAsync([ArraySegment[byte]]::new($request), [Net.WebSockets.WebSocketMessageType]::Text,
            $true, $cancel.Token).GetAwaiter().GetResult()
    } finally {
        $cancel.Dispose()
    }

    Read-TimerEvent 'getState' 'idle'
    [void](Wait-For 'the two-minute focus timer' { Find-UiElement '2:00' })
    Invoke-Button 'Boop the tomato'
    [void](Wait-For 'the start control after a boop' { Find-UiElement 'Start timer' })
    Invoke-Button 'Start timer'
    Read-TimerEvent 'roundChange' 'work'
    Invoke-Button 'Pause timer'
    Read-TimerEvent 'roundChange' 'paused'
    Invoke-Button 'Resume timer'
    Read-TimerEvent 'roundChange' 'work'
    Invoke-Button 'Skip round'
    Read-TimerEvent 'roundChange' 'idle'
    [void](Wait-For 'the short break after skipping' { Find-UiElement 'SHORT BREAK' })
    Write-Host 'Passed WebSocket state queries and native start, pause, resume, and skip.'

    [PomotroidWindowsCheck]::PostToOwner($window, $app.Id, 0x0112, 0xF020, 0)
    [void](Wait-For 'the minimized window to hide' { -not [PomotroidWindowsCheck]::IsWindowVisible($window) })
    $window = Restore-FromTray
    Write-Host 'Passed minimize to tray and restore.'

    Invoke-TrayIcon
    [void](Wait-For 'a tray click to hide the visible window' { -not [PomotroidWindowsCheck]::IsWindowVisible($window) })
    $window = Restore-FromTray
    Write-Host 'Passed toggling the window from the tray.'

    Invoke-Button 'Start timer'
    Read-TimerEvent 'roundChange' 'short-break'
    $breakElapsed = [Diagnostics.Stopwatch]::StartNew()
    [PomotroidWindowsCheck]::PostToOwner($window, $app.Id, 0x0010, 0, 0)
    [void](Wait-For 'the closed window to hide' { -not [PomotroidWindowsCheck]::IsWindowVisible($window) })
    Read-TimerEvent 'roundChange' 'idle' 75 -WhileHidden
    if ($breakElapsed.Elapsed.TotalSeconds -lt 59) { throw 'The one-minute break completed early.' }
    $window = Restore-FromTray
    [void](Wait-For 'the focus timer after the hidden break completed' { Find-UiElement 'FOCUS' })
    [void](Wait-For 'the full two-minute focus round' { Find-UiElement '2:00' })
    Write-Host 'Passed close to tray, natural timer completion while hidden, and restore.'
} finally {
    if ($null -ne $socket) { $socket.Dispose() }
    if ($null -ne $app) {
        if (-not $app.HasExited) { $app.Kill() }
        if (-not $app.WaitForExit(5000)) { throw 'The isolated Pomotroid process did not exit.' }
        $app.Dispose()
    }
    if ($null -ne $errors) {
        $stderr = $errors.GetAwaiter().GetResult()
        if ($stderr) { Write-Host $stderr }
    }
    if (Test-Path -LiteralPath $profilePath) {
        $resolved = (Resolve-Path -LiteralPath $profilePath).ProviderPath
        if ($resolved -ne $profilePath) { throw "The temporary profile resolved outside the test directory: $resolved" }
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
}
