[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$fixture = Join-Path $root ('target/launcher-failure-' + [guid]::NewGuid())
[IO.Directory]::CreateDirectory($fixture) | Out-Null
$exe = Join-Path $fixture 'frosthaven-lan.exe'
[IO.File]::Copy((Join-Path $root 'dist/frosthaven-lan.exe'), $exe)
$info = New-Object Diagnostics.ProcessStartInfo
$info.FileName = $exe
$info.Arguments = 'start'
$info.WorkingDirectory = $fixture
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$info.RedirectStandardInput = $true
$info.RedirectStandardOutput = $true
$info.RedirectStandardError = $true
$info.StandardOutputEncoding = New-Object Text.UTF8Encoding($false)
$info.StandardErrorEncoding = New-Object Text.UTF8Encoding($false)
$process = New-Object Diagnostics.Process
$process.StartInfo = $info
try {
    if (!$process.Start()) { throw 'Could not start the failure fixture' }
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    if ($process.WaitForExit(700)) { throw 'The launcher exited on failure before Enter was pressed' }
    $process.StandardInput.WriteLine()
    $process.StandardInput.Flush()
    if (!$process.WaitForExit(10000)) { throw 'The launcher did not exit after Enter' }
    $output = $stdout.GetAwaiter().GetResult()
    $errors = $stderr.GetAwaiter().GetResult()
    if ($process.ExitCode -ne 1) { throw 'Failure must retain a nonzero exit status' }
    if (!$output.Contains('Enter')) { throw 'The Enter-to-close prompt was missing' }
    if (!$errors.Contains('Missing LanBridge.dll') -or !$errors.Contains('Diagnostic trace:') -or !$errors.Contains($fixture)) {
        throw ('The complete original error or runtime diagnostic context was missing: ' + $errors)
    }
    if (@(Get-ChildItem -LiteralPath $fixture -File).Count -ne 1) { throw 'The launcher wrote unexpected runtime files' }
    Write-Output 'PASS: a failed launcher with start arguments waits for Enter, prints the raw error and diagnostic history, and writes no logs'
} finally {
    if (!$process.HasExited) { $process.StandardInput.Close(); $null = $process.WaitForExit(10000) }
    $process.Dispose()
}
