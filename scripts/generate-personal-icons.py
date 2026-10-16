#!/usr/bin/env python3
"""Render the personal fork's vector mark; requires rsvg-convert, no installs."""
from pathlib import Path
import struct
import subprocess


def main():
    root = Path(__file__).resolve().parents[1]
    assets = root / "assets/personal"
    sizes = {}
    for size in (16, 32, 64, 128, 256, 512, 1024):
        sizes[size] = subprocess.run(
            ["rsvg-convert", "-w", str(size), "-h", str(size), str(assets / "icon.svg")],
            check=True, capture_output=True,
        ).stdout
    (assets / "icon.png").write_bytes(sizes[1024])
    chunks = []
    for tag, size in (
        (b"icp4", 16), (b"ic11", 32),
        (b"icp5", 32), (b"ic12", 64),
        (b"ic07", 128), (b"ic13", 256),
        (b"ic08", 256), (b"ic14", 512),
        (b"ic09", 512), (b"ic10", 1024),
    ):
        png = sizes[size]
        chunks.append(tag + struct.pack(">I", len(png) + 8) + png)
    payload = b"".join(chunks)
    (assets / "HerdrPersonal.icns").write_bytes(b"icns" + struct.pack(">I", len(payload) + 8) + payload)


if __name__ == "__main__":
    main()
