#!/usr/bin/env python3
"""Build the relocatable Actus user archive and its separate debug artifact."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tomllib


ARTIFACTS = (
    "bin/actus",
    "lib/actus/bin/ld.lld",
    "lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a",
    "lib/actus/targets/thumbv7em-none-eabi/libclang_rt.builtins.a",
)


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository-root", type=Path, required=True)
    parser.add_argument("--prefix", type=Path, required=True)
    parser.add_argument("--build-root", type=Path, required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--vc-redist-root", type=Path)
    return parser.parse_args()


def copy_file(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def sha256_path(path: Path) -> str:
    digest = hashlib.sha256()
    if path.is_file():
        digest.update(path.read_bytes())
    else:
        for child in sorted(path.rglob("*")):
            if child.is_file():
                digest.update(child.relative_to(path).as_posix().encode())
                digest.update(b"\0")
                digest.update(child.read_bytes())
    return digest.hexdigest()


def find_runtime(build_root: Path) -> Path:
    candidates = sorted(build_root.rglob("libactus_runtime.a"))
    if len(candidates) != 1:
        raise SystemExit(f"expected exactly one runtime archive, found {len(candidates)}: {candidates}")
    return candidates[0]


def prepare_binary(source: Path, destination: Path, debug_output: Path) -> None:
    copy_file(source, destination)
    if sys.platform.startswith("linux"):
        objcopy = shutil.which("objcopy")
        strip = shutil.which("strip")
        if not objcopy or not strip:
            raise SystemExit("Linux release packaging requires objcopy and strip")
        subprocess.run([objcopy, "--only-keep-debug", str(source), str(debug_output)], check=True)
        subprocess.run([strip, "--strip-unneeded", str(destination)], check=True)
        subprocess.run([objcopy, f"--add-gnu-debuglink={debug_output}", str(destination)], check=True)
    elif sys.platform == "darwin":
        dsymutil = shutil.which("dsymutil")
        strip = shutil.which("strip")
        if not dsymutil or not strip:
            raise SystemExit("macOS release packaging requires dsymutil and strip")
        subprocess.run([dsymutil, str(source), "-o", str(debug_output)], check=True)
        subprocess.run([strip, "-S", str(destination)], check=True)


def copy_windows_runtime(stage: Path, redist_root: Path | None) -> None:
    if redist_root is None:
        raise SystemExit("Windows packaging requires --vc-redist-root")
    if redist_root.parent.name != "x64" or not redist_root.name.startswith("Microsoft.VC") or redist_root.name.endswith("Debug.NonRedist"):
        raise SystemExit(f"invalid x64 VC redist directory: {redist_root}")
    files = [redist_root / name for name in ("VCRUNTIME140.dll", "VCRUNTIME140_1.dll", "MSVCP140.dll")]
    if any(not path.is_file() for path in files):
        raise SystemExit(f"exactly the x64 VC runtime set is required: {files}")
    for path in files:
        copy_file(path, stage / "bin" / path.name)


def write_release_manifest(stage: Path, target: str, suffix: str) -> None:
    manifest_path = stage / "share/actus/toolchain.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    runtime = f"lib/actus/runtime/{target}/libactus_runtime.a"
    paths = [f"bin/actus{suffix}", f"lib/actus/bin/ld.lld{suffix}", *ARTIFACTS[2:], runtime]
    manifest["release_artifacts"] = {
        path: {"path": path, "sha256": hashlib.sha256((stage / path).read_bytes()).hexdigest()}
        for path in paths
    }
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    args = arguments()
    root = args.repository_root.resolve()
    prefix = args.prefix.resolve()
    build_root = args.build_root.resolve()
    output = args.output_dir.resolve()
    package = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["package"]
    if args.version != package["version"]:
        raise SystemExit(f"Cargo.toml version mismatch: expected {package['version']}, got {args.version}")
    if not (prefix / "share/actus/toolchain.json").is_file():
        raise SystemExit(f"bundled toolchain manifest is missing under {prefix}")
    source = root / ("target/release/actus.exe" if os.name == "nt" else "target/release/actus")
    if not source.is_file():
        raise SystemExit(f"release binary is missing: {source}")
    runtime = find_runtime(build_root)
    name = f"actus-{args.version}-{args.target}"
    stage = output / name
    archive = output / (f"{name}.zip" if os.name == "nt" else f"{name}.tar.xz")
    debug_suffix = ".pdb" if os.name == "nt" else (".dSYM" if sys.platform == "darwin" else ".debug")
    debug = output / f"{name}{debug_suffix}"
    if stage.exists():
        shutil.rmtree(stage)
    stage.mkdir(parents=True)
    (stage / "bin").mkdir()
    prepare_binary(source, stage / ("bin/actus.exe" if os.name == "nt" else "bin/actus"), debug)
    suffix = ".exe" if os.name == "nt" else ""
    copy_file(prefix / f"lib/actus/bin/ld.lld{suffix}", stage / f"lib/actus/bin/ld.lld{suffix}")
    for abi in ("eabihf", "eabi"):
        copy_file(prefix / f"lib/actus/targets/thumbv7em-none-{abi}/libclang_rt.builtins.a", stage / f"lib/actus/targets/thumbv7em-none-{abi}/libclang_rt.builtins.a")
    shutil.copytree(root / "library/std", stage / "lib/actus/std")
    copy_file(runtime, stage / f"lib/actus/runtime/{args.target}/libactus_runtime.a")
    copy_file(prefix / "share/actus/toolchain.json", stage / "share/actus/toolchain.json")
    for license_name in ("README.md", "LICENSE-APACHE", "LICENSE-MIT", "NOTICE"):
        copy_file(root / license_name, stage / license_name)
    shutil.copytree(prefix / "licenses", stage / "licenses")
    copy_file(root / "LICENSE-APACHE", stage / "licenses/LICENSE-APACHE")
    copy_file(root / "LICENSE-MIT", stage / "licenses/LICENSE-MIT")
    if os.name == "nt":
        pdb = source.with_suffix(".pdb")
        if not pdb.is_file():
            raise SystemExit(f"Windows debug artifact is missing: {pdb}")
        copy_file(pdb, debug)
        copy_windows_runtime(stage, args.vc_redist_root)
    write_release_manifest(stage, args.target, suffix)
    package = root / "scripts/llvm/package_archive.py"
    archive_format = "zip" if os.name == "nt" else "tar.xz"
    subprocess.run(
        [
            sys.executable,
            str(package),
            "--input-root",
            str(stage),
            "--output",
            str(archive),
            "--format",
            archive_format,
            "--top-level-dir",
            name,
        ],
        check=True,
    )
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    (archive.with_name(f"{archive.name}.sha256")).write_text(f"{digest}  {archive.name}\n", encoding="utf-8")
    debug_digest = sha256_path(debug)
    debug.with_name(f"{debug.name}.sha256").write_text(f"{debug_digest}  {debug.name}\n", encoding="utf-8")
    print(f"archive={archive}")
    print(f"debug_artifact={debug}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
