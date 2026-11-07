#!/usr/bin/env python3
"""Create reproducible contributor archives for non-Linux hosts."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
from pathlib import PureWindowsPath
import stat
import tarfile
import zipfile


EPOCH = 0
ZIP_EPOCH = (1980, 1, 1, 0, 0, 0)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--format", choices=("tar.xz", "zip"), required=True)
    parser.add_argument(
        "--manifest",
        default="share/actus/toolchain.json",
        help="manifest path relative to input root",
    )
    return parser.parse_args()


def validate_relative_symlink(root: Path, path: Path, relative: str) -> None:
    target = os.readlink(path)
    if os.path.isabs(target) or PureWindowsPath(target).is_absolute() or PureWindowsPath(target).drive:
        raise SystemExit(f"absolute symlink target is not allowed: {relative} -> {target}")
    resolved_target = (path.parent / target).resolve(strict=False)
    if resolved_target == root or root not in resolved_target.parents:
        raise SystemExit(f"symlink target escapes archive root: {relative} -> {target}")
    if not resolved_target.exists():
        raise SystemExit(f"symlink target does not exist: {relative} -> {target}")


def archive_entries(root: Path, output: Path, archive_format: str) -> list[tuple[Path, str]]:
    root = root.resolve()
    output = output.resolve()
    if not root.is_dir():
        raise SystemExit(f"input root is not a directory: {root}")
    if output == root or root in output.parents:
        raise SystemExit("output archive must be outside input root")

    entries: list[tuple[Path, str]] = []
    for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix()):
        relative = path.relative_to(root).as_posix()
        if path.is_symlink():
            if archive_format == "zip":
                raise SystemExit(f"symlinks are not allowed in zip archives: {relative}")
            validate_relative_symlink(root, path, relative)
            entries.append((path, relative))
        if path.is_dir():
            entries.append((path, f"{relative}/"))
        elif path.is_file():
            entries.append((path, relative))
        else:
            raise SystemExit(f"unsupported archive entry: {relative}")
    return entries


def fixed_mode(path: Path, relative: str) -> int:
    if path.is_symlink():
        return 0o777
    if path.is_dir():
        return 0o755
    executable = bool(path.stat().st_mode & (stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH))
    executable = executable or relative.startswith("bin/") or relative.endswith(".exe")
    return 0o755 if executable else 0o644


def normalize_manifest(root: Path, manifest_name: str) -> None:
    manifest = root / manifest_name
    if not manifest.is_file():
        raise SystemExit(f"toolchain manifest is missing: {manifest_name}")
    try:
        payload = json.loads(manifest.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        raise SystemExit(f"invalid toolchain manifest: {error}") from error
    manifest.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def write_tar(output: Path, entries: list[tuple[Path, str]]) -> None:
    with tarfile.open(output, mode="w:xz", format=tarfile.PAX_FORMAT) as archive:
        for path, relative in entries:
            info = archive.gettarinfo(str(path), arcname=relative)
            info.mtime = EPOCH
            info.uid = 0
            info.gid = 0
            info.uname = ""
            info.gname = ""
            info.mode = fixed_mode(path, relative)
            if path.is_file() and not path.is_symlink():
                with path.open("rb") as source:
                    archive.addfile(info, source)
            else:
                archive.addfile(info)


def write_zip(output: Path, entries: list[tuple[Path, str]]) -> None:
    with zipfile.ZipFile(output, mode="w", compression=zipfile.ZIP_STORED) as archive:
        for path, relative in entries:
            info = zipfile.ZipInfo(relative, date_time=ZIP_EPOCH)
            info.create_system = 3
            info.external_attr = fixed_mode(path, relative) << 16
            if path.is_dir():
                info.external_attr |= 0x10
                archive.writestr(info, b"")
            else:
                archive.writestr(info, path.read_bytes())


def write_checksum(output: Path) -> str:
    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    output.with_name(f"{output.name}.sha256").write_text(
        f"{digest}  {output.name}\n", encoding="utf-8", newline="\n"
    )
    return digest


def main() -> int:
    args = parse_args()
    root = args.input_root.resolve()
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    normalize_manifest(root, args.manifest)
    entries = archive_entries(root, output, args.format)
    if args.format == "tar.xz":
        write_tar(output, entries)
    else:
        write_zip(output, entries)
    digest = write_checksum(output)
    print(f"created {output} ({output.stat().st_size} bytes)")
    print(f"sha256 {digest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
