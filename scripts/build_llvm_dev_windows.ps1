$ErrorActionPreference = "Stop"

$llvmVersion = "23.1.2"
$sourceRevision = if ($env:LLVM_SOURCE_REVISION) { $env:LLVM_SOURCE_REVISION } else { "85ac560262434c9ccfc0c183ec22d4138ed647fb" }
$archiveTarget = "x86_64-pc-windows-msvc"
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$workRoot = if ($env:LLVM_BUILD_ROOT) { $env:LLVM_BUILD_ROOT } else { Join-Path $env:RUNNER_TEMP "actus-llvm" }
$sourceRoot = Join-Path $workRoot "llvm-project"
$buildRoot = Join-Path $workRoot "build"
$installRoot = Join-Path $workRoot "install"
$stagingRoot = Join-Path $workRoot "stage"
$outputRoot = Join-Path $repositoryRoot "dist"
$archiveName = "llvm-dev-$llvmVersion-$archiveTarget.zip"
$cmakeCache = Join-Path $repositoryRoot "toolchains/llvm/llvm-$llvmVersion.cmake"

function Require-Command([string]$Name) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "required command is missing: $Name"
    }
}

foreach ($commandName in @("git", "cmake", "ninja", "sccache", "python", "clang", "dumpbin")) {
    Require-Command $commandName
}
. (Join-Path $repositoryRoot "scripts/llvm/Resolve-Builtin.ps1")
if (-not (Test-Path $cmakeCache)) {
    throw "LLVM CMake cache is missing: $cmakeCache"
}

New-Item -ItemType Directory -Force -Path $workRoot, $outputRoot | Out-Null
$env:SCCACHE_DIR = if ($env:SCCACHE_DIR) { $env:SCCACHE_DIR } else { Join-Path $workRoot "sccache" }
& sccache --zero-stats

if (-not (Test-Path (Join-Path $sourceRoot ".git"))) {
    & git clone --filter=blob:none --no-checkout https://github.com/llvm/llvm-project.git $sourceRoot
}
& git -C $sourceRoot fetch --depth=1 origin $sourceRevision
& git -C $sourceRoot checkout --detach $sourceRevision
$resolvedSourceRevision = (& git -C $sourceRoot rev-parse HEAD).Trim()

Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $buildRoot, $installRoot, $stagingRoot
$cmakeArguments = @(
    "-S", (Join-Path $sourceRoot "llvm"),
    "-B", $buildRoot,
    "-G", "Ninja",
    "-C", $cmakeCache,
    "-DCMAKE_BUILD_TYPE=Release",
    "-DCMAKE_INSTALL_PREFIX=$installRoot",
    "-DCMAKE_INSTALL_LIBDIR=lib",
    "-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreadedDLL",
    "-DLLVM_PARALLEL_LINK_JOBS=1"
)
if ($env:ACTUS_CMAKE_LAUNCHER) {
    $cmakeArguments += "-DCMAKE_C_COMPILER_LAUNCHER=$env:ACTUS_CMAKE_LAUNCHER"
    $cmakeArguments += "-DCMAKE_CXX_COMPILER_LAUNCHER=$env:ACTUS_CMAKE_LAUNCHER"
}
& cmake @cmakeArguments
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
if ($env:ACTUS_LLVM_CONFIGURE_ONLY -eq "1") {
    Write-Host "LLVM CMake configure completed; build intentionally skipped"
    exit 0
}

& cmake --build $buildRoot --target install-distribution --parallel ($env:CMAKE_BUILD_PARALLEL_LEVEL ?? "4")
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$freshClang = Join-Path $buildRoot "bin/clang.exe"
if (-not (Test-Path $freshClang)) { throw "fresh in-tree clang.exe was not built" }
$freshClangVersion = (& $freshClang --version | Select-Object -First 1).Trim()
$llvmConfig = Join-Path $installRoot "bin/llvm-config.exe"
$ldLld = Join-Path $installRoot "bin/ld.lld.exe"
if (-not (Test-Path $llvmConfig)) { throw "llvm-config.exe was not installed" }
if (-not (Test-Path $ldLld)) { throw "ld.lld.exe was not installed" }
if ((& $llvmConfig --version).Trim() -ne $llvmVersion) { throw "unexpected LLVM version" }
$targetsBuilt = (& $llvmConfig --targets-built).Trim()
foreach ($targetName in @("AArch64", "ARM", "X86")) {
    if ($targetsBuilt -notmatch "(^|[; ])$targetName($|[; ])") {
        throw "LLVM target is missing: $targetName; got: $targetsBuilt"
    }
}

$eabihfBuiltin = Resolve-Builtin -InstallRoot $installRoot -Abi "eabihf"
$eabiBuiltin = Resolve-Builtin -InstallRoot $installRoot -Abi "eabi"
New-Item -ItemType Directory -Force -Path `
    (Join-Path $stagingRoot "lib/actus/bin"), `
    (Join-Path $stagingRoot "lib/actus/targets/thumbv7em-none-eabihf"), `
    (Join-Path $stagingRoot "lib/actus/targets/thumbv7em-none-eabi"), `
    (Join-Path $stagingRoot "share/actus") | Out-Null
Copy-Item -Path (Join-Path $installRoot "*") -Destination $stagingRoot -Recurse -Force
Copy-Item $ldLld (Join-Path $stagingRoot "lib/actus/bin/ld.lld.exe") -Force
Copy-Item $eabihfBuiltin (Join-Path $stagingRoot "lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a") -Force
Copy-Item $eabiBuiltin (Join-Path $stagingRoot "lib/actus/targets/thumbv7em-none-eabi/libclang_rt.builtins.a") -Force
$licenseRoot = Join-Path $stagingRoot "licenses"
New-Item -ItemType Directory -Force -Path $licenseRoot | Out-Null
Copy-Item (Join-Path $sourceRoot "LICENSE.TXT") (Join-Path $licenseRoot "LLVM-LICENSE.TXT") -Force
Copy-Item (Join-Path $sourceRoot "clang/LICENSE.TXT") (Join-Path $licenseRoot "LLVM-CLANG-LICENSE.TXT") -Force
Copy-Item (Join-Path $sourceRoot "compiler-rt/LICENSE.TXT") (Join-Path $licenseRoot "LLVM-COMPILER-RT-LICENSE.TXT") -Force
@(
    "LLVM 23.1.2 third-party license index",
    "Source revision: $resolvedSourceRevision",
    "Component license texts are included in this directory."
) | Set-Content -Encoding utf8 (Join-Path $licenseRoot "THIRD-PARTY-NOTICES.txt")

$manifest = [ordered]@{
    llvm_version = $llvmVersion
    source_revision = $resolvedSourceRevision
    targets = $targetsBuilt -split "\s+"
    cmake_cache = (Split-Path $cmakeCache -Leaf)
    cmake_cache_sha256 = (Get-FileHash $cmakeCache -Algorithm SHA256).Hash.ToLowerInvariant()
    runner_image = "windows-2022"
    msvc_toolset = $env:VCToolsVersion
    builtins_compiler = "build/bin/clang.exe"
    builtins_compiler_version = $freshClangVersion
    host_triple = $archiveTarget
    cxx_runtime = "msvc-dynamic-/MD"
    cmake_flags = [ordered]@{
        LLVM_ENABLE_PROJECTS = @("clang", "lld")
        LLVM_TARGETS_TO_BUILD = @("X86", "ARM", "AArch64")
        LLVM_BUILTIN_TARGETS = @("thumbv7em-none-eabihf", "thumbv7em-none-eabi")
        LLVM_STATIC_LINK_CXX_STDLIB = $false
        LLVM_BUILD_LLVM_DYLIB = $false
        LLVM_ENABLE_ASSERTIONS = $false
        LLVM_PARALLEL_LINK_JOBS = 1
        CMAKE_MSVC_RUNTIME_LIBRARY = "MultiThreadedDLL"
    }
    lld = "lib/actus/bin/ld.lld.exe"
    compiler_rt_builtins = [ordered]@{
        "thumbv7em-none-eabihf" = "lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a"
        "thumbv7em-none-eabi" = "lib/actus/targets/thumbv7em-none-eabi/libclang_rt.builtins.a"
    }
    identity = "published-archive-sha256"
}
$manifest | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 (Join-Path $stagingRoot "share/actus/toolchain.json")

$archivePath = Join-Path $outputRoot $archiveName
Remove-Item -Force -ErrorAction SilentlyContinue $archivePath, "$archivePath.sha256"
& python (Join-Path $repositoryRoot "scripts/llvm/package_archive.py") `
    --input-root $stagingRoot --output $archivePath --format zip
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& sccache --show-stats
Write-Host "created $archivePath"
