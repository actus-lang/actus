import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile


SCRIPT = Path(__file__).parents[2] / "scripts" / "llvm" / "package_archive.py"


class PackageArchiveTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name) / "input"
        self.root.joinpath("bin").mkdir(parents=True)
        self.root.joinpath("lib/actus").mkdir(parents=True)
        self.root.joinpath("share/actus").mkdir(parents=True)
        self.root.joinpath("bin/actus").write_bytes(b"actus")
        self.root.joinpath("lib/actus/ld.lld").write_bytes(b"lld")
        self.root.joinpath("share/actus/toolchain.json").write_text(
            json.dumps({"llvm_version": "23.1.2", "identity": "test"}),
            encoding="utf-8",
        )
        self.output = Path(self.tempdir.name) / "out"
        self.output.mkdir()

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def run_packager(self, archive_format: str, name: str) -> Path:
        archive = self.output / name
        subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--input-root",
                str(self.root),
                "--output",
                str(archive),
                "--format",
                archive_format,
            ],
            check=True,
            capture_output=True,
            text=True,
        )
        return archive

    def assert_deterministic(self, archive_format: str, suffix: str) -> None:
        first = self.run_packager(archive_format, f"first{suffix}")
        first_bytes = first.read_bytes()
        first_digest = hashlib.sha256(first_bytes).hexdigest()
        second = self.run_packager(archive_format, f"second{suffix}")
        second_bytes = second.read_bytes()
        second_digest = hashlib.sha256(second_bytes).hexdigest()
        self.assertEqual(first_bytes, second_bytes)
        self.assertEqual(first_digest, second_digest)
        self.assertEqual(
            first_digest,
            first.with_name(f"{first.name}.sha256").read_text(encoding="utf-8").split()[0],
        )
        self.assertEqual(
            second_digest,
            second.with_name(f"{second.name}.sha256").read_text(encoding="utf-8").split()[0],
        )

    def test_tar_xz_is_byte_deterministic(self) -> None:
        self.assert_deterministic("tar.xz", ".tar.xz")
        archive = self.output / "first.tar.xz"
        with tarfile.open(archive, mode="r:xz") as tar:
            self.assertEqual(
                tar.getnames(),
                [
                    "bin",
                    "bin/actus",
                    "lib",
                    "lib/actus",
                    "lib/actus/ld.lld",
                    "share",
                    "share/actus",
                    "share/actus/toolchain.json",
                ],
            )

    def test_zip_is_byte_deterministic(self) -> None:
        self.assert_deterministic("zip", ".zip")
        archive = self.output / "first.zip"
        with zipfile.ZipFile(archive) as zip_archive:
            self.assertEqual(
                zip_archive.namelist(),
                [
                    "bin/",
                    "bin/actus",
                    "lib/",
                    "lib/actus/",
                    "lib/actus/ld.lld",
                    "share/",
                    "share/actus/",
                    "share/actus/toolchain.json",
                ],
            )

    def test_tar_xz_preserves_valid_relative_symlink(self) -> None:
        (self.root / "bin/real-ld").write_bytes(b"lld")
        (self.root / "bin/ld.lld").symlink_to("real-ld")

        archive = self.run_packager("tar.xz", "links.tar.xz")
        with tarfile.open(archive, mode="r:xz") as tar:
            member = tar.getmember("bin/ld.lld")
            self.assertTrue(member.issym())
            self.assertEqual(member.linkname, "real-ld")
            self.assertEqual(member.mtime, 0)
            self.assertEqual(member.uid, 0)
            self.assertEqual(member.gid, 0)

    def test_external_symlink_is_rejected(self) -> None:
        outside = Path(self.tempdir.name) / "outside"
        outside.write_bytes(b"outside")
        (self.root / "bin/outside").symlink_to("../../outside")
        process = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--input-root",
                str(self.root),
                "--output",
                str(self.output / "outside.tar.xz"),
                "--format",
                "tar.xz",
            ],
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(process.returncode, 0)
        self.assertIn("symlink target escapes archive root", process.stderr)

    def test_zip_rejects_symlink(self) -> None:
        (self.root / "bin/alias").symlink_to("actus")
        process = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--input-root",
                str(self.root),
                "--output",
                str(self.output / "links.zip"),
                "--format",
                "zip",
            ],
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(process.returncode, 0)
        self.assertIn("symlinks are not allowed in zip archives", process.stderr)


if __name__ == "__main__":
    unittest.main()
