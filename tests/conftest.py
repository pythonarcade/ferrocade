import os
import struct
import zlib

import pytest


def pytest_collection_modifyitems(config, items):
    if os.environ.get("FERROCADE_HEADLESS") != "1":
        return
    skip = pytest.mark.skip(reason="FERROCADE_HEADLESS=1: no display available")
    for item in items:
        if "display" in item.keywords:
            item.add_marker(skip)


def write_png(path, width, height, rgba=(255, 0, 0, 255)):
    """Write a solid-color RGBA PNG using only the standard library."""

    def chunk(kind, data):
        crc = struct.pack(">I", zlib.crc32(kind + data))
        return struct.pack(">I", len(data)) + kind + data + crc

    row = b"\x00" + bytes(rgba) * width  # filter byte 0 (None) per row
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(row * height))
    png += chunk(b"IEND", b"")
    path.write_bytes(png)
    return path


@pytest.fixture
def make_png():
    return write_png
