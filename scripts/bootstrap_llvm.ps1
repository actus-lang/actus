#Requires -Version 7.0
[CmdletBinding()]
param(
    [string]$ManifestPath = (Join-Path $PSScriptRoot "..\toolchains\llvm\llvm-23.1.2.json"),
    [string]$InstallRoot = (Join-Path $PSScriptRoot "..\.actus-toolchain\llvm-23.1.2"),
    [switch]$Offline
)

$ErrorActionPreference = "Stop"
$llvmVersion = "23.1.2"
$hostTriple = "x86_64-pc-windows-msvc"
$manifest = Get-Content -Raw -LiteralPath $ManifestPath | ConvertFrom-Json
$record = $manifest.archives.PSObject.Properties[$hostTriple].Value
if ($null -eq $record -or [string]::IsNullOrWhiteSpace($record.file) -or $record.file.Contains("/") -or $record.file.Contains("\") -or $record.release_url -notlike "https://*" -or $null -eq $record.size -or [string]::IsNullOrWhiteSpace($record.sha256)) {
    throw "manifest record for $hostTriple is incomplete or unsafe"
}

function Test-VerifiedInstall([string]$Root) {
    $llvmConfig = Join-Path $Root "bin\llvm-config.exe"
    if (-not (Test-Path -LiteralPath $llvmConfig -PathType Leaf)) { return $false }
    if (((& $llvmConfig --version).Trim()) -ne $llvmVersion) { return $false }
    $targets = (& $llvmConfig --targets-built)
    foreach ($target in @("AArch64", "ARM", "X86")) { if ($targets -notmatch "(^|[; ])$target($|[; ])") { return $false } }
    if (-not (Test-Path -LiteralPath (Join-Path $Root "lib\actus\bin\ld.lld.exe") -PathType Leaf)) { return $false }
    $llvmNm = Join-Path $Root "bin\llvm-nm.exe"
    foreach ($abi in @("eabihf", "eabi")) {
        $builtin = Join-Path $Root "lib\actus\targets\thumbv7em-none-$abi\libclang_rt.builtins.a"
        if (-not (Test-Path -LiteralPath $builtin -PathType Leaf) -or (Get-Item -LiteralPath $builtin).Length -eq 0) { return $false }
        if ((& $llvmNm --quiet --defined-only $builtin | Select-String "__aeabi_uldivmod") -eq $null) { return $false }
    }
    return (Test-Path -LiteralPath (Join-Path $Root "licenses\LLVM-LICENSE.TXT") -PathType Leaf) -and (Test-Path -LiteralPath (Join-Path $Root "licenses\THIRD-PARTY-NOTICES.txt") -PathType Leaf)
}

if (Test-Path -LiteralPath $InstallRoot -PathType Container) {
    if (Test-VerifiedInstall $InstallRoot) { Write-Host "verified LLVM $llvmVersion already exists at $InstallRoot; no download needed"; exit 0 }
    throw "existing LLVM directory is not verified; refusing to modify $InstallRoot"
}
if ($Offline) { throw "offline bootstrap requested but no verified LLVM directory exists at $InstallRoot" }

$parentRoot = Split-Path -Parent $InstallRoot
New-Item -ItemType Directory -Force -Path $parentRoot | Out-Null
$temporaryRoot = Join-Path $parentRoot (".llvm-$llvmVersion-" + [Guid]::NewGuid().ToString("N"))
$extractRoot = Join-Path $temporaryRoot "extracted"
$archivePath = Join-Path $temporaryRoot $record.file
$archiveCachePath = $env:ACTUS_LLVM_ARCHIVE_CACHE
$archiveCacheHit = $false
New-Item -ItemType Directory -Path $extractRoot | Out-Null
try {
    if (-not [string]::IsNullOrWhiteSpace($archiveCachePath) -and (Test-Path -LiteralPath $archiveCachePath -PathType Leaf)) {
        $archivePath = $archiveCachePath
        $archiveCacheHit = $true
        Write-Host "using cached LLVM archive $archivePath"
    } else {
        Invoke-WebRequest -Uri $record.release_url -OutFile $archivePath -MaximumRetryCount 5 -RetryIntervalSec 10
    }
    $actualSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $archivePath).Hash.ToLowerInvariant()
    if ($actualSha256 -ne $record.sha256.ToLowerInvariant()) { throw "LLVM archive SHA-256 mismatch: expected $($record.sha256), got $actualSha256" }
    $actualSize = (Get-Item -LiteralPath $archivePath).Length
    if ($actualSize -ne [int64]$record.size) { throw "LLVM archive size mismatch: expected $($record.size), got $actualSize" }
    if (-not [string]::IsNullOrWhiteSpace($archiveCachePath) -and -not $archiveCacheHit) {
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $archiveCachePath) | Out-Null
        Copy-Item -LiteralPath $archivePath -Destination $archiveCachePath -Force
        $archivePath = $archiveCachePath
    }
    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractRoot
    if (-not (Test-VerifiedInstall $extractRoot)) { throw "downloaded LLVM archive failed verification" }
    Move-Item -LiteralPath $extractRoot -Destination $InstallRoot
} finally {
    if (Test-Path -LiteralPath $temporaryRoot) { Remove-Item -LiteralPath $temporaryRoot -Recurse -Force }
}
Write-Host "installed verified LLVM $llvmVersion at $InstallRoot"
