"""Helpers for loading the Rust Python extension during development."""

from __future__ import annotations

import argparse
import importlib
import importlib.machinery
import os
import shutil
import subprocess
import sys
from pathlib import Path
from types import ModuleType

MODULE_NAME = "_truco_engine"
ROOT = Path(__file__).resolve().parents[2]
TARGET_DIR = ROOT / "target"
STAGED_EXTENSION_DIR = TARGET_DIR / "python-extension"


def build_native_module(*, release: bool = False) -> Path:
    """Build the Rust extension crate and return the native library path."""

    command = [
        "cargo",
        "build",
        "--offline",
        "-p",
        "truco-engine-py",
    ]
    if release:
        command.append("--release")

    env = os.environ.copy()
    env["PYO3_PYTHON"] = sys.executable

    subprocess.run(
        command,
        cwd=ROOT,
        check=True,
        text=True,
        capture_output=True,
        env=env,
    )
    return native_library_path(release=release)


def native_library_path(*, release: bool = False) -> Path:
    """Return the platform-specific dynamic library produced by Cargo."""

    profile_dir = TARGET_DIR / ("release" if release else "debug")
    candidates = []

    if sys.platform == "darwin":
        candidates.append(profile_dir / "lib_truco_engine.dylib")
    elif sys.platform.startswith("linux"):
        candidates.append(profile_dir / "lib_truco_engine.so")
    elif os.name == "nt":
        candidates.extend(
            [
                profile_dir / "_truco_engine.dll",
                profile_dir / "truco_engine.dll",
                profile_dir / "lib_truco_engine.dll",
            ]
        )

    candidates.extend(
        [
            profile_dir / "lib_truco_engine.so",
            profile_dir / "lib_truco_engine.dylib",
        ]
    )

    for candidate in candidates:
        if candidate.exists():
            return candidate

    raise FileNotFoundError(
        "native Rust extension not found; build it with "
        "`python -m truco_engine._native_loader --build`"
    )


def staged_extension_path() -> Path:
    """Return the importable extension path used for local development."""

    suffixes = importlib.machinery.EXTENSION_SUFFIXES
    suffix = ".abi3.so" if ".abi3.so" in suffixes else suffixes[0]
    return STAGED_EXTENSION_DIR / f"{MODULE_NAME}{suffix}"


def stage_native_module(*, build: bool = False, release: bool = False) -> Path:
    """Stage the native library under a Python-importable filename."""

    source = (
        build_native_module(release=release)
        if build
        else native_library_path(release=release)
    )
    staged_path = staged_extension_path()
    staged_path.parent.mkdir(parents=True, exist_ok=True)

    if staged_path.exists() or staged_path.is_symlink():
        staged_path.unlink()

    try:
        staged_path.symlink_to(source)
    except OSError:
        shutil.copy2(source, staged_path)

    return staged_path


def import_native_module(*, build: bool = False, release: bool = False) -> ModuleType:
    """Import the staged or installed native module."""

    try:
        return importlib.import_module(MODULE_NAME)
    except ImportError as import_error:
        try:
            if build or native_library_path(release=release).exists():
                staged_path = stage_native_module(build=build, release=release)
                staged_dir = str(staged_path.parent)
                if staged_dir not in sys.path:
                    sys.path.insert(0, staged_dir)
                importlib.invalidate_caches()
                return importlib.import_module(MODULE_NAME)
        except FileNotFoundError as file_error:
            raise ImportError(str(file_error)) from import_error

        raise


def load_api_match_class():
    """Load the native ApiMatch class or raise ImportError."""

    module = import_native_module()
    return module.ApiMatch


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Stage the Rust Truco Python extension for local imports."
    )
    parser.add_argument(
        "--build",
        action="store_true",
        help="build the Rust extension before staging it",
    )
    parser.add_argument(
        "--release",
        action="store_true",
        help="use the release profile instead of debug",
    )
    args = parser.parse_args(argv)

    staged_path = stage_native_module(build=args.build, release=args.release)
    print(staged_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
