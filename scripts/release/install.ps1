$ErrorActionPreference = 'Stop'

$repository = '__ACTUS_REPOSITORY__'
$version = '__ACTUS_VERSION__'
$target = 'x86_64-pc-windows-msvc'
$asset = "actus-$version-$target.zip"
$baseUrl = "https://github.com/$repository/releases/download/$version"
$temporaryArchive = Join-Path $env:TEMP $asset
$temporaryChecksum = Join-Path $env:TEMP "$asset.sha256"
$installRoot = Join-Path $env:LOCALAPPDATA "Actus\$version"

Invoke-WebRequest "$baseUrl/$asset" -OutFile $temporaryArchive
Invoke-WebRequest "$baseUrl/$asset.sha256" -OutFile $temporaryChecksum

$expectedChecksum = (Get-Content $temporaryChecksum).Split()[0].ToLowerInvariant()
$actualChecksum = (Get-FileHash $temporaryArchive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($expectedChecksum -ne $actualChecksum) {
  throw 'Actus release checksum verification failed.'
}

if (Test-Path $installRoot) {
  Remove-Item $installRoot -Recurse -Force
}
New-Item -ItemType Directory -Path (Split-Path $installRoot) -Force | Out-Null
Expand-Archive -Path $temporaryArchive -DestinationPath $env:TEMP -Force
$bundleRoot = Join-Path $env:TEMP "actus-$version-$target"
Move-Item $bundleRoot $installRoot

$binPath = Join-Path $installRoot 'bin'
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (($userPath -split ';') -notcontains $binPath) {
  [Environment]::SetEnvironmentVariable('Path', "$userPath;$binPath", 'User')
}

Remove-Item $temporaryArchive, $temporaryChecksum -Force
Write-Host "Actus $version installed at $binPath\actus.exe"
Write-Host 'Open a new terminal before using actus.'
