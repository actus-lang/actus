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


if __name__ == "__main__":
    unittest.main()
