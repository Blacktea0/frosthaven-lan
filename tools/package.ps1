[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = Split-Path -Parent $PSScriptRoot
$metadata = (& cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $root 'Cargo.toml') | ConvertFrom-Json)
if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
$version = ($metadata.packages | Where-Object name -eq 'frosthaven-lan').version
$files = @('frosthaven-lan.exe', 'LanBridge.dll', 'Bootstrap.dll', 'LanPayload.dll', '0Harmony.dll', 'README.md', 'README.en.md', 'PRODUCT.md', 'Harmony-LICENSE.txt')
$dist = Join-Path $root 'dist'
$artifacts = Join-Path $root 'artifacts'
New-Item -ItemType Directory -Force -Path $artifacts | Out-Null
$packageName = "frosthaven-lan-v$version-windows-x64"
$zip = Join-Path $artifacts "$packageName.zip"
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class ExecutableManifest {
 [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern IntPtr LoadLibraryEx(string path, IntPtr file, uint flags);
 [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr FindResource(IntPtr module, IntPtr name, IntPtr type);
 [DllImport("kernel32.dll")] static extern uint SizeofResource(IntPtr module, IntPtr resource);
 [DllImport("kernel32.dll")] static extern IntPtr LoadResource(IntPtr module, IntPtr resource);
 [DllImport("kernel32.dll")] static extern IntPtr LockResource(IntPtr resource);
 [DllImport("kernel32.dll")] static extern bool FreeLibrary(IntPtr module);
 public static string Read(string path) {
  var module=LoadLibraryEx(path,IntPtr.Zero,2);
  if(module==IntPtr.Zero)throw new Exception("Cannot load executable resources");
  try {
   var resource=FindResource(module,new IntPtr(1),new IntPtr(24));
   if(resource==IntPtr.Zero)throw new Exception("Embedded manifest missing");
   var bytes=new byte[SizeofResource(module,resource)];
   Marshal.Copy(LockResource(LoadResource(module,resource)),bytes,0,bytes.Length);
   return Encoding.UTF8.GetString(bytes).TrimStart('\uFEFF');
  } finally { FreeLibrary(module); }
 }
}
"@
function Get-Sha256([string]$Path) {
    $algorithm = [Security.Cryptography.SHA256]::Create()
    $stream = [IO.File]::OpenRead($Path)
    try { return ([BitConverter]::ToString($algorithm.ComputeHash($stream))).Replace('-', '').ToLowerInvariant() }
    finally { $stream.Dispose(); $algorithm.Dispose() }
}
$temporary = Join-Path ([IO.Path]::GetTempPath()) ('frosthaven-package-' + [guid]::NewGuid())
try {
    # Create an archive from an allowlist, excluding logs and stale dist files.
    $stream = [IO.File]::Open($zip, [IO.FileMode]::Create)
    $archive = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($name in $files) {
            $source = Join-Path $dist $name
            if (!(Test-Path -LiteralPath $source -PathType Leaf)) { throw "Missing runtime file: $name" }
            [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, $source, "$packageName/$name", [IO.Compression.CompressionLevel]::Optimal) | Out-Null
        }
    } finally { $archive.Dispose(); $stream.Dispose() }
    [IO.Compression.ZipFile]::ExtractToDirectory($zip, $temporary)
    $topLevel = @(Get-ChildItem -LiteralPath $temporary -Force)
    if ($topLevel.Count -ne 1 -or !$topLevel[0].PSIsContainer -or $topLevel[0].Name -ne $packageName) {
        throw 'Archive must contain exactly one package directory at the top level'
    }
    $runtime = Join-Path $temporary $packageName
    $actual = @(Get-ChildItem -LiteralPath $runtime -Force | Select-Object -ExpandProperty Name)
    if (Compare-Object $files $actual) { throw 'Unexpected archive contents' }
    foreach ($name in $files) {
        $expected = (Get-Sha256 (Join-Path $dist $name))
        $extracted = (Get-Sha256 (Join-Path $runtime $name))
        if ($expected -ne $extracted) { throw "Archive integrity check failed: $name" }
    }
    [xml]$manifest = [ExecutableManifest]::Read((Join-Path $runtime 'frosthaven-lan.exe'))
    $level = $manifest.SelectSingleNode("//*[local-name()='requestedExecutionLevel']")
    if ($null -eq $level -or $level.level -ne 'asInvoker' -or $level.uiAccess -ne 'false') {
        throw 'The launcher must preserve caller permissions with an asInvoker manifest'
    }
    $hash = (Get-Sha256 $zip)
    [IO.File]::WriteAllText("$zip.sha256", "$hash  $([IO.Path]::GetFileName($zip))`n", (New-Object Text.UTF8Encoding($false)))
    Write-Output "Verified portable archive: $zip"
    Write-Output "SHA256: $hash"
} finally {
    # Only delete the unique temporary directory created above.
    $resolved = [IO.Path]::GetFullPath($temporary)
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    if (!$resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -or !(Split-Path -Leaf $resolved).StartsWith('frosthaven-package-')) {
        throw 'Refusing to clean an unexpected temporary path'
    }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
