# Install an Actus release on Windows x86-64.
#
#   irm https://actus-lang.org/install.ps1 | iex
#   & ([scriptblock]::Create((irm https://actus-lang.org/install.ps1))) -Version 0.1.0
#
# The archive is checked against the release's SHA256SUMS before anything is
# installed. Each version lives in <prefix>\toolchains\<archive>; the
# <prefix>\current junction moves to a new version only after its compiler has
# run, so a failed install leaves the previous one in place.

param(
    [string]$Version = $env:ACTUS_VERSION,
    [string]$Prefix = $(if ($env:ACTUS_HOME) { $env:ACTUS_HOME } else { Join-Path $env:LOCALAPPDATA 'Actus' }),
    [switch]$NoModifyPath
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$repository = 'https://github.com/actus-lang/actus'

function Fail([string]$Message) {
    throw "actus-install: $Message"
}

if (-not [Environment]::Is64BitOperatingSystem -or $env:PROCESSOR_ARCHITECTURE -ne 'AMD64') {
    Fail "no Actus release for $env:PROCESSOR_ARCHITECTURE; releases cover Windows x86-64, Linux x86-64 and macOS arm64"
}
$target = 'x86_64-pc-windows-msvc'

# ACTUS_INSTALL_BASE_URL points at a directory holding the archive and
# SHA256SUMS, such as a local directory in release verification.
$baseUrl = $env:ACTUS_INSTALL_BASE_URL
if (-not $Version) {
    if ($baseUrl) { Fail '-Version is required with ACTUS_INSTALL_BASE_URL' }
    $response = Invoke-WebRequest -UseBasicParsing -Method Head -Uri "$repository/releases/latest"
    $latest = $response.BaseResponse.ResponseUri.AbsoluteUri
    if (-not $latest) { $latest = $response.BaseResponse.RequestMessage.RequestUri.AbsoluteUri }
    if ($latest -notmatch '/tag/v([^/]+)$') { Fail "cannot read the latest release from $latest" }
    $Version = $Matches[1]
}
$Version = $Version.TrimStart('v')
if ($Version -notmatch '^[0-9A-Za-z.+-]+$') { Fail "invalid version: $Version" }
if (-not $baseUrl) { $baseUrl = "$repository/releases/download/v$Version" }

$name = "actus-$Version-$target"
$work = Join-Path ([IO.Path]::GetTempPath()) ("actus-install-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $work | Out-Null

# Removes the current junction itself, never the toolchain it points at;
# Remove-Item on a junction could descend into the target.
function Remove-CurrentLink([string]$Path) {
    if (Get-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue) { [IO.Directory]::Delete($Path) }
}

function Get-ReleaseFile([string]$File) {
    $destination = Join-Path $work $File
    if (Test-Path -LiteralPath $baseUrl) {
        Copy-Item -LiteralPath (Join-Path $baseUrl $File) -Destination $destination
    } else {
        Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$File" -OutFile $destination
    }
    $destination
}

try {
    Write-Host "Installing Actus $Version ($target)"
    $sums = Get-ReleaseFile 'SHA256SUMS'
    $archive = Get-ReleaseFile "$name.zip"
    $expected = Get-Content -LiteralPath $sums |
        ForEach-Object { $fields = $_ -split '\s+', 2; if ($fields.Count -eq 2 -and $fields[1].TrimStart('*') -eq "$name.zip") { $fields[0] } } |
        Select-Object -First 1
    if (-not $expected) { Fail "SHA256SUMS has no entry for $name.zip" }
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash
    if ($actual -ne $expected.ToUpperInvariant()) {
        Fail "checksum mismatch for ${name}.zip: expected $expected, got $($actual.ToLowerInvariant())"
    }

    $toolchains = Join-Path $Prefix 'toolchains'
    New-Item -ItemType Directory -Force -Path $toolchains | Out-Null
    $staging = Join-Path $toolchains (".install-" + [Guid]::NewGuid())
    try {
        Expand-Archive -LiteralPath $archive -DestinationPath $staging
        $compiler = Join-Path $staging "$name\bin\actus.exe"
        if (-not (Test-Path -LiteralPath $compiler)) { Fail "archive has no $name\bin\actus.exe" }
        & $compiler --version | Out-Null
        if ($LASTEXITCODE -ne 0) { Fail 'the installed compiler does not run' }
    } catch {
        Remove-Item -Recurse -Force -LiteralPath $staging -ErrorAction SilentlyContinue
        throw
    }

    $destination = Join-Path $toolchains $name
    $current = Join-Path $Prefix 'current'
    if (Test-Path -LiteralPath $destination) {
        Remove-CurrentLink $current
        Remove-Item -Recurse -Force -LiteralPath $destination
    }
    Move-Item -LiteralPath (Join-Path $staging $name) -Destination $destination
    Remove-Item -Recurse -Force -LiteralPath $staging
    Remove-CurrentLink $current
    New-Item -ItemType Junction -Path $current -Target $destination | Out-Null

    $binDir = Join-Path $current 'bin'
    if (-not $NoModifyPath) {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $entries = @($userPath -split ';' | Where-Object { $_ })
        if ($entries -notcontains $binDir) {
            [Environment]::SetEnvironmentVariable('Path', (@($binDir) + $entries) -join ';', 'User')
        }
    }
    if (($env:Path -split ';') -notcontains $binDir) { $env:Path = "$binDir;$env:Path" }

    & (Join-Path $destination 'bin\actus.exe') --version
    Write-Host ''
    Write-Host "Actus $Version is installed in $destination"
    if ($NoModifyPath) {
        Write-Host "Add $binDir to PATH to use actus in new terminals."
    } else {
        Write-Host 'Open a new terminal to use actus.'
    }
} finally {
    Remove-Item -Recurse -Force -LiteralPath $work -ErrorAction SilentlyContinue
}
