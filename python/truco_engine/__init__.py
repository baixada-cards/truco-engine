"""Python interface to Baixada's authoritative Rust Truco engine."""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from .rust_embed import RustApiMatch

__all__ = ["RustApiMatch"]
__version__ = "0.1.0"


def __getattr__(name: str):
    if name == "RustApiMatch":
        from .rust_embed import RustApiMatch

        return RustApiMatch

    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
