#!/usr/bin/env python3
"""Verify an extracted Actus user archive without Cargo or LLVM_SYS."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile


def args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--incoming", type=Path, required=True)
    parser.add_argument("--support", type=Path, required=True)
    parser.add_argument("--target", required=True)
    return parser.parse_args()


def run(command: list[str], cwd: Path | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=cwd,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def find_single_archive(incoming: Path) -> Path:
    suffix = ".zip" if os.name == "nt" else ".tar.xz"
    found = sorted(
        path
        for path in incoming.iterdir()
        if path.is_file() and path.name.startswith("actus-") and path.name.endswith(suffix)
    )
    if len(found) != 1:
        raise SystemExit(f"expected one archive in {incoming}, found {[path.name for path in found]}")
    return found[0]


def find_single_startup(support: Path) -> Path:
    found = sorted(path for path in support.iterdir() if path.is_file() and path.name == "startup.o")
    if len(found) != 1:
        raise SystemExit(f"expected one startup.o in {support}, found {[path.name for path in found]}")
    return found[0]


def extract(archive: Path, directory: Path, expected_root: str) -> Path:
    if archive.suffix == ".zip":
        shutil.unpack_archive(archive, directory, "zip")
    else:
        subprocess.run(["tar", "-xJf", str(archive), "-C", str(directory)], check=True)
    entries = list(directory.iterdir())
    if len(entries) != 1 or not entries[0].is_dir() or entries[0].name != expected_root:
        raise SystemExit(
            f"archive must contain exactly one top-level directory named {expected_root}, "
            f"found {[entry.name for entry in entries]}"
        )
    return entries[0]


def verify_manifest(root: Path) -> dict[str, object]:
    manifest = json.loads((root / "share/actus/toolchain.json").read_text(encoding="utf-8"))
    if manifest.get("llvm_version") != "23.1.2":
        raise SystemExit("archive LLVM version is not 23.1.2")
    records = manifest.get("release_artifacts")
    if not isinstance(records, dict) or len(records) != 5:
        raise SystemExit("release_artifacts must contain exactly five records")
    for record in records.values():
        if set(record) != {"path", "sha256"}:
            raise SystemExit("release artifact records must contain path and sha256")
        path = root / record["path"]
        if not path.is_file():
            raise SystemExit(f"release artifact is missing: {path}")
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != record["sha256"]:
            raise SystemExit(f"artifact SHA mismatch: {path}")
    return manifest


def verify_identity(binary: Path) -> None:
    result = run([str(binary), "--version", "--json"])
    if result.returncode != 0:
        raise SystemExit(result.stderr)
    identity = json.loads(result.stdout)
    llvm = identity["llvm"]
    if llvm["version"] != "23.1.2" or llvm["status"] != "verified":
        raise SystemExit(f"invalid bundled identity: {llvm}")


def verify_hosted(binary: Path, work: Path) -> None:
    if os.name == "nt" and (shutil.which("link.exe") is None or os.environ.get("VCToolsInstallDir") is None):
        print("NOT RUN: hosted Windows hello requires the MSVC linker and Windows SDK")
        return
    source = work / "hello.act"
    source.write_text("verb main() -> Int { return 0; }\n", encoding="utf-8")
    output = work / ("hello.exe" if os.name == "nt" else "hello")
    result = run([str(binary), "build", "--strict", "--emit", "exe", "-o", str(output), str(source)])
    if result.returncode != 0:
        raise SystemExit(f"hosted build failed: {result.stderr}")
    result = run([str(output)])
    if result.returncode != 0:
        raise SystemExit(f"hosted hello failed: {result.stderr}")


def verify_runtime(root: Path, target: str) -> None:
    if not (root / "lib/actus/std/Actus.toml").is_file():
        raise SystemExit("installed standard library package is missing")
    if not (root / f"lib/actus/runtime/{target}/libactus_runtime.a").is_file():
        raise SystemExit("installed runtime archive is missing")
    linker = root / ("lib/actus/bin/ld.lld.exe" if os.name == "nt" else "lib/actus/bin/ld.lld")
    if not linker.is_file():
        raise SystemExit("installed linker is missing")
    version = run([str(linker), "--version"])
    if version.returncode != 0 or "LLD 23.1.2" not in version.stdout:
        raise SystemExit(f"bundled linker identity mismatch: {version.stdout}{version.stderr}")
    for abi in ("eabihf", "eabi"):
        builtin = root / f"lib/actus/targets/thumbv7em-none-{abi}/libclang_rt.builtins.a"
        if not builtin.is_file():
            raise SystemExit(f"installed compiler-rt builtin is missing: {builtin}")


def verify_platform(binary: Path) -> None:
    if sys.platform.startswith("linux"):
        ldd = run(["ldd", str(binary)]).stdout
        print(ldd, end="")
        if "libstdc++" in ldd:
            raise SystemExit("Linux archive still depends on libstdc++")
        versions = [tuple(map(int, value.split("."))) for value in re.findall(r"GLIBC_(\d+(?:\.\d+)*)", run(["objdump", "-T", str(binary)]).stdout)]
        if versions and max(versions) > (2, 28):
            raise SystemExit(f"GLIBC baseline exceeded: {max(versions)}")
    elif os.name == "nt":
        vswhere = Path(os.environ.get("ProgramFiles(x86)", "C:/Program Files (x86)")) / "Microsoft Visual Studio/Installer/vswhere.exe"
        if not vswhere.is_file():
            raise SystemExit(f"Windows archive verification requires vswhere.exe: {vswhere}")
        result = run([str(vswhere), "-latest", "-products", "*", "-find", r"VC\Tools\MSVC\**\bin\Hostx64\x64\dumpbin.exe"])
        candidates = [Path(line.strip()) for line in result.stdout.splitlines() if line.strip()]
        if result.returncode != 0 or not candidates:
            raise SystemExit("Windows archive verification requires dumpbin.exe")
        dumpbin = str(candidates[-1])
        result = subprocess.run([dumpbin, "/dependents", str(binary)], text=True)
        if result.returncode != 0:
            raise SystemExit("dumpbin failed")


def verify_windows_runtime(root: Path) -> None:
    if os.name != "nt":
        return
    import struct

    for name in ("VCRUNTIME140.dll", "VCRUNTIME140_1.dll", "MSVCP140.dll"):
        path = root / "bin" / name
        data = path.read_bytes()
        pe_offset = struct.unpack_from("<I", data, 0x3C)[0]
        if data[pe_offset:pe_offset + 4] != b"PE\0\0" or struct.unpack_from("<H", data, pe_offset + 4)[0] != 0x8664:
            raise SystemExit(f"{name} is not an x64 PE image")


def verify_arm(root: Path, binary: Path, startup: Path, work: Path) -> None:
    arm = work / "arm"
    arm.mkdir()
    (arm / "Actus.toml").write_text(
        "[package]\nname = \"archive_arm\"\nversion = \"0.1.0\"\nentry = \"boot\"\n\n[build]\ntarget = \"thumbv7em-none-eabihf\"\nlinker_script = \"memory.x\"\n",
        encoding="utf-8",
    )
    (arm / "main.act").write_text(
        "struct Pair { erg left: u64, erg right: u64, erg extra0: u64, erg extra1: u64, erg extra2: u64, erg extra3: u64, erg extra4: u64, erg extra5: u64, } verb dividend() -> u64 { return 80u64; } verb divisor() -> u64 { return 2u64; } verb boot() -> Int { erg source: Pair = Pair { left: 1u64, right: 1u64, extra0: 0u64, extra1: 0u64, extra2: 0u64, extra3: 0u64, extra4: 0u64, extra5: 0u64, }; erg copied: Pair = source; erg numerator: u64 = dividend(); erg denominator: u64 = divisor(); erg quotient: u64 = numerator / denominator; return (copied.left + copied.right + quotient) as Int; }\n",
        encoding="utf-8",
    )
    (arm / "memory.x").write_text(
        "ENTRY(Reset_Handler)\nMEMORY { FLASH (rx) : ORIGIN = 0x00000000, LENGTH = 4M RAM (rwx) : ORIGIN = 0x20000000, LENGTH = 512K }\n_estack = ORIGIN(RAM) + LENGTH(RAM);\nSECTIONS { . = ORIGIN(FLASH); .isr_vector : { LONG(_estack); LONG((Reset_Handler & ~1) + 1); KEEP(*(.isr_vector)) } > FLASH .text : { *(.text*) *(.rodata*) } > FLASH .data : { *(.data*) } > RAM AT > FLASH .bss : { *(.bss*) *(COMMON) } > RAM /DISCARD/ : { *(.ARM.exidx*) *(.ARM.extab*) } }\n",
        encoding="utf-8",
    )
    object_path = arm / "actus.o"
    result = run([str(binary), "build", "--strict", "--emit", "obj", "-o", str(object_path), str(arm / "main.act")], cwd=arm)
    if result.returncode != 0:
        raise SystemExit(f"freestanding Actus build failed: {result.stderr}")
    linker = root / ("lib/actus/bin/ld.lld.exe" if os.name == "nt" else "lib/actus/bin/ld.lld")
    builtins = root / "lib/actus/targets/thumbv7em-none-eabihf/libclang_rt.builtins.a"
    image = arm / "image.elf"
    result = run([str(linker), "-T", str(arm / "memory.x"), str(startup), str(object_path), str(builtins), "-o", str(image)])
    if result.returncode != 0:
        raise SystemExit(f"freestanding image link failed: {result.stderr}")
    if not image.is_file():
        raise SystemExit("freestanding linker did not produce an image")
    backup = linker.with_name(linker.name + ".missing")
    linker.rename(backup)
    try:
        missing = run([str(binary), "build", "--strict", "--emit", "obj", "-o", str(arm / "missing.o"), str(arm / "main.act")], cwd=arm)
        if missing.returncode == 0 or "E1880" not in f"{missing.stdout}{missing.stderr}":
            raise SystemExit("control failure did not reject the missing bundled linker with E1880")
    finally:
        backup.rename(linker)
    if sys.platform.startswith("linux"):
        qemu = shutil.which("qemu-system-arm")
        if qemu is None:
            raise SystemExit("qemu-system-arm is required for Linux user archive verification")
        child = subprocess.Popen([qemu, "-M", "mps2-an500", "-nographic", "-semihosting-config", "enable=on,target=native", "-kernel", str(image)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            stdout, stderr = child.communicate(timeout=30)
        except subprocess.TimeoutExpired:
            child.kill(); stdout, stderr = child.communicate()
            raise SystemExit(f"QEMU image did not exit within 30 s: {stdout}{stderr}")
        print(f"QEMU exit status: {child.returncode}")
        print(f"QEMU stdout: {stdout}")
        print(f"QEMU stderr: {stderr}")
        if child.returncode != 0:
            raise SystemExit("QEMU image failed")
    else:
        print("NOT RUN: QEMU (non-Linux host)")


def main() -> int:
    parsed = args()
    if not parsed.incoming.is_dir():
        raise SystemExit(f"incoming is not a directory: {parsed.incoming}")
    if not parsed.support.is_dir():
        raise SystemExit(f"support is not a directory: {parsed.support}")
    archive = find_single_archive(parsed.incoming)
    startup = find_single_startup(parsed.support)
    for variable in ("LLVM_SYS_231_PREFIX", "ACTUS_LLVM_CONFIG_PATH"):
        if os.environ.get(variable):
            raise SystemExit(f"build-time LLVM prefix must be absent: {variable} is set")
    print("build-time LLVM prefix: absent")
    with tempfile.TemporaryDirectory(prefix="actus-release-verify-") as temporary:
        archive_name = archive.name
        for suffix in (".tar.xz", ".zip"):
            if archive_name.endswith(suffix):
                archive_name = archive_name[: -len(suffix)]
                break
        expected_root = archive_name
        root = extract(archive, Path(temporary), expected_root)
        verify_manifest(root)
        binary = root / ("bin/actus.exe" if os.name == "nt" else "bin/actus")
        verify_identity(binary)
        verify_runtime(root, parsed.target)
        verify_platform(binary)
        verify_windows_runtime(root)
        verify_hosted(binary, Path(temporary))
        verify_arm(root, binary, startup, Path(temporary))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
