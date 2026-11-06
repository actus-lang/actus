$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "Resolve-Builtin.ps1")
$fakeRoot = Join-Path $env:RUNNER_TEMP "actus-builtin-resolver-test"
Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $fakeRoot
New-Item -ItemType Directory -Force -Path $fakeRoot | Out-Null
try {
    $candidateRoot = Join-Path $fakeRoot "lib/clang/23/lib"
    $firstCandidate = Join-Path $candidateRoot "thumbv7em-none-eabi/libclang_rt.builtins.a"
    $secondCandidate = Join-Path $candidateRoot "thumbv7em-unknown-none-eabi/libclang_rt.builtins.a"
    New-Item -ItemType Directory -Force -Path (Split-Path $firstCandidate), (Split-Path $secondCandidate) | Out-Null

    try {
        Resolve-Builtin -InstallRoot $fakeRoot -Abi "eabi" | Out-Null
        throw "resolver accepted zero candidates"
    } catch {
        if ($_.Exception.Message -eq "resolver accepted zero candidates") { throw }
    }

    Set-Content -Path $firstCandidate -Value "x" -NoNewline
    if ((Resolve-Builtin -InstallRoot $fakeRoot -Abi "eabi") -ne $firstCandidate) {
        throw "resolver returned the wrong single candidate"
    }

    Set-Content -Path $secondCandidate -Value "x" -NoNewline
    try {
        Resolve-Builtin -InstallRoot $fakeRoot -Abi "eabi" | Out-Null
        throw "resolver accepted two candidates"
    } catch {
        if ($_.Exception.Message -eq "resolver accepted two candidates") { throw }
    }
    Write-Host "PowerShell builtin resolver cases passed"
} finally {
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $fakeRoot
}
