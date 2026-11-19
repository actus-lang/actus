#!/usr/bin/env python3
"""Benchmark gate (ADR-0114): Actus kernels against their C twins.

Each kernel in `benchmarks/kernels` is built twice: the Actus source with
the `release` profile on the LLVM backend, and the C source with
`clang -O3`. Both run under Callgrind, which counts executed instructions
deterministically, collecting from `main`. The gate fails when the two
programs exit differently, when Actus executes more than `threshold` times
the instructions of C, or when an Actus kernel executes more than
`ratchet_tolerance` above its recorded baseline.
"""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--actus", type=Path, required=True, help="actus compiler binary")
    parser.add_argument("--cc", default=os.environ.get("CC_GATE", "clang"), help="C compiler")
    parser.add_argument(
        "--kernels",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "benchmarks/kernels",
    )
    parser.add_argument(
        "--update-baseline",
        action="store_true",
        help="write the measured Actus counts as the new baseline",
    )
    return parser.parse_args()


def run(command: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, capture_output=True, text=True, check=False)


def build_actus(actus: Path, source: Path, output: Path) -> None:
    result = run(
        [str(actus), "build", str(source), "--profile", "release", "--backend", "llvm"]
        + ["--emit", "exe", "-o", str(output)]
    )
    if result.returncode != 0:
        raise SystemExit(f"actus build of {source.name} failed:\n{result.stderr}")


def build_c(cc: str, source: Path, output: Path) -> None:
    result = run([cc, "-O3", str(source), "-o", str(output)])
    if result.returncode != 0:
        raise SystemExit(f"{cc} build of {source.name} failed:\n{result.stderr}")


def measure(program: Path) -> tuple[int, int]:
    """The exit status of `program` and the instructions it executed from `main`."""
    result = run(
        ["valgrind", "--tool=callgrind", "--toggle-collect=main", "--callgrind-out-file=/dev/null"]
        + [str(program)]
    )
    collected = re.search(r"Collected : (\d+)", result.stderr)
    if collected is None:
        raise SystemExit(f"callgrind reported no instruction count for {program.name}:\n{result.stderr}")
    return result.returncode, int(collected.group(1))


def write_baseline(path: Path, counts: dict[str, int]) -> None:
    """Rewrites the `[baseline]` table, which closes the file."""
    text = path.read_text(encoding="utf-8")
    head = text[: text.index("[baseline]")]
    table = "".join(f"{name} = {count}\n" for name, count in sorted(counts.items()))
    path.write_text(f"{head}[baseline]\n{table}", encoding="utf-8")


def main() -> int:
    args = arguments()
    for tool in ("valgrind", args.cc):
        if shutil.which(tool) is None:
            raise SystemExit(f"benchmark gate requires `{tool}` on PATH")
    config = tomllib.loads((args.kernels / "gate.toml").read_text(encoding="utf-8"))
    threshold = float(config["threshold"])
    tolerance = float(config["ratchet_tolerance"])
    baseline = config["baseline"]
    rows = []
    failures = []
    with tempfile.TemporaryDirectory(prefix="actus-benchmark-gate-") as temporary:
        directory = Path(temporary)
        for name in config["kernels"]:
            actus_program = directory / f"{name}-actus"
            c_program = directory / f"{name}-c"
            build_actus(args.actus.resolve(), args.kernels / f"{name}.act", actus_program)
            build_c(args.cc, args.kernels / f"{name}.c", c_program)
            actus_status, actus_count = measure(actus_program)
            c_status, c_count = measure(c_program)
            ratio = actus_count / c_count
            change = actus_count / baseline[name] - 1
            rows.append((name, actus_count, c_count, ratio, change, actus_status))
            if actus_status != c_status:
                failures.append(f"{name}: Actus exited {actus_status}, C exited {c_status}")
            if ratio > threshold:
                failures.append(f"{name}: {ratio:.3f}x the instructions of C exceeds {threshold}x")
            if change > tolerance and not args.update_baseline:
                failures.append(f"{name}: {change:+.1%} against its baseline exceeds {tolerance:+.0%}")
    report = [
        f"## Benchmark gate (threshold {threshold}x C `-O3`)",
        "",
        "| kernel | Actus instructions | C instructions | ratio | baseline | exit |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    report += [f"| {n} | {a:,} | {c:,} | {r:.3f} | {b:+.1%} | {s} |" for n, a, c, r, b, s in rows]
    text = "\n".join(report) + "\n"
    print(text)
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as handle:
            handle.write(text)
    if args.update_baseline:
        write_baseline(args.kernels / "gate.toml", {row[0]: row[1] for row in rows})
    for failure in failures:
        print(f"FAIL {failure}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
