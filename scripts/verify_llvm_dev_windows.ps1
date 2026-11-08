$ErrorActionPreference = "Stop"

$llvmVersion = "23.1.2"
$archiveTarget = "x86_64-pc-windows-msvc"
$archiveName = "llvm-dev-$llvmVersion-$archiveTarget.zip"
$verificationRoot = Join-Path $env:RUNNER_TEMP "actus-llvm-windows-verification"
$extractedRoot = Join-Path $verificationRoot "extracted"
$archivePath = Join-Path $verificationRoot $archiveName
$checksumPath = "$archivePath.sha256"
$maximumArchiveSize = [int64]332593539

foreach ($commandName in @("python", "dumpbin")) {
    if (-not (Get-Command $commandName -ErrorAction SilentlyContinue)) {
        throw "required verification command is missing: $commandName"
    }
}

Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $verificationRoot
New-Item -ItemType Directory -Force -Path $extractedRoot | Out-Null
Copy-Item (Join-Path "dist" $archiveName) $archivePath
Copy-Item "$(Join-Path "dist" $archiveName).sha256" $checksumPath
$expectedDigest = (Get-Content $checksumPath -Raw).Split()[0]
$actualDigest = (Get-FileHash $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualDigest -ne $expectedDigest) { throw "archive checksum mismatch" }
$archiveSize = (Get-Item -LiteralPath $archivePath).Length
if ($archiveSize -gt $maximumArchiveSize) {
    throw "archive exceeds size limit: $archiveSize > $maximumArchiveSize"
}
Expand-Archive -Path $archivePath -DestinationPath $extractedRoot -Force

$prefix = $extractedRoot
$llvmConfig = Join-Path $prefix "bin/llvm-config.exe"
$llvmNm = Join-Path $prefix "bin/llvm-nm.exe"
$llvmObjdump = Join-Path $prefix "bin/llvm-objdump.exe"
$llc = Join-Path $prefix "bin/llc.exe"
$ldLld = Join-Path $prefix "lib/actus/bin/ld.lld.exe"
$eabihfBuiltin = Join-Path $prefix "lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a"
$eabiBuiltin = Join-Path $prefix "lib/actus/targets/thumbv7em-none-eabi/libclang_rt.builtins.a"
$manifestPath = Join-Path $prefix "share/actus/toolchain.json"
foreach ($toolPath in @($llvmConfig, $llvmNm, $llvmObjdump, $llc, $ldLld)) {
    if (-not (Test-Path -LiteralPath $toolPath -PathType Leaf)) {
        throw "bundled LLVM tool is missing: $toolPath"
    }
}
if ((& $llvmConfig --version).Trim() -ne $llvmVersion) { throw "llvm-config version mismatch" }
$targetsBuilt = (& $llvmConfig --targets-built).Trim()
foreach ($targetName in @("AArch64", "ARM", "X86")) {
    if ($targetsBuilt -notmatch "(^|[; ])$targetName($|[; ])") { throw "LLVM target is missing: $targetName" }
}
foreach ($builtin in @($eabihfBuiltin, $eabiBuiltin)) {
    if (-not (Test-Path $builtin)) { throw "missing builtin archive: $builtin" }
    if ((& $llvmNm --quiet --defined-only $builtin | Select-String "__aeabi_uldivmod").Count -eq 0) {
        throw "$builtin does not define __aeabi_uldivmod"
    }
}

foreach ($toolName in @("ld.lld.exe", "lld-link.exe")) {
    $toolPath = Join-Path $prefix "bin/$toolName"
    if (-not (Test-Path -LiteralPath $toolPath -PathType Leaf)) {
        throw "bundled linker tool is missing: $toolPath"
    }
    if ((Get-Item -LiteralPath $toolPath).LinkType) {
        throw "Windows archive contains a symlink: $toolPath"
    }
}
foreach ($forbiddenPath in @(
    (Join-Path $prefix "bin/lld.exe"),
    (Join-Path $prefix "bin/ld64.lld.exe"),
    (Join-Path $prefix "bin/wasm-ld.exe"),
    (Join-Path $prefix "bin/LTO.dll"),
    (Join-Path $prefix "lib/LTO.lib")
)) {
    if (Test-Path -LiteralPath $forbiddenPath) {
        throw "forbidden archive artifact is present: $forbiddenPath"
    }
}

foreach ($binary in @($llvmConfig, $ldLld)) {
    $dependencies = (& dumpbin /dependents $binary | Out-String)
    $dlls = [regex]::Matches($dependencies, "(?im)^\s+([A-Za-z0-9_.-]+\.dll)\s*$") |
        ForEach-Object { $_.Groups[1].Value.ToLowerInvariant() }
    foreach ($dll in $dlls) {
        $allowed = $dll -match "^(api-ms-win-|ext-ms-win-|kernel32|advapi32|user32|gdi32|shell32|ole32|oleaut32|ws2_32|bcrypt|crypt32|dbghelp|iphlpapi|ntdll|rpcrt4|version|winmm|vcruntime140|msvcp140|ucrtbase)"
        if (-not $allowed) { throw "non-system or non-MSVC dependency in ${binary}: $dll" }
    }
}

$thumbObject = Join-Path $verificationRoot "thumb-zero-float.o"
& $llc -mtriple=thumbv7em-none-eabihf -mcpu=cortex-m4 -filetype=obj `
    (Join-Path "tests/toolchain" "thumb_zero_float.ll") -o $thumbObject
$disassembly = (& $llvmObjdump -d $thumbObject | Out-String)
if ($disassembly -match "(?i)\b(vadd|vsub|vmul|vdiv|vmov|vldr|vstr|vpush|vpop|vmla|vmsr|vmrs)[A-Za-z0-9_.]*\b") {
    throw "Thumb zero-float fixture contains a VFP instruction"
}

$manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
if ($manifest.llvm_version -ne $llvmVersion) { throw "manifest LLVM version mismatch" }
if ($manifest.host_triple -ne $archiveTarget) { throw "manifest host triple mismatch" }
if ($manifest.cxx_runtime -ne "msvc-dynamic-/MD") { throw "manifest MSVC runtime mismatch" }
if ($manifest.builtins_compiler -ne "build/bin/clang.exe") { throw "manifest builtin compiler provenance mismatch" }
if ($manifest.identity -ne "published-archive-sha256") { throw "manifest identity mismatch" }
if ($manifest.cmake_flags.CMAKE_MSVC_RUNTIME_LIBRARY -ne "MultiThreadedDLL") {
    throw "manifest CMake MSVC runtime mismatch"
}
Write-Host "verified $archiveName with archive-prefix, MSVC dependency, builtin, and zero-float checks"
