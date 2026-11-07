function Resolve-Builtin {
    param(
        [Parameter(Mandatory = $true)]
        [string]$InstallRoot,
        [Parameter(Mandatory = $true)]
        [string]$Abi
    )

    $triple = "thumbv7em-none-$Abi"
    $runtimeRoot = Join-Path $InstallRoot "lib/clang/23/lib"
    $candidates = @(
        (Join-Path $runtimeRoot "$triple/libclang_rt.builtins.a"),
        (Join-Path $runtimeRoot "thumbv7em-unknown-none-$Abi/libclang_rt.builtins.a")
    )
    $existing = @($candidates | Where-Object {
        (Test-Path -LiteralPath $_ -PathType Leaf) -and ((Get-Item -LiteralPath $_).Length -gt 0)
    })
    if ($existing.Count -ne 1) {
        Write-Error "expected exactly one compiler-rt builtin for $triple, found $($existing.Count)"
        Write-Host "all discovered compiler-rt builtins:"
        Get-ChildItem -Path $InstallRoot -Recurse -File -Filter "libclang_rt.builtins*" |
            ForEach-Object { Write-Host $_.FullName }
        throw "compiler-rt builtin resolution failed"
    }
    return $existing[0]
}
