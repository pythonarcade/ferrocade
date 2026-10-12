import shutil
from pathlib import Path

import pytest

import ferrocade as fc
from ferrocade import _ferrocade

SHIP = "assets/images/ship.png"


def test_load_ship():
    tex = fc.Texture(SHIP)
    assert (tex.width, tex.height) == (99, 75)
    assert Path(tex.path).is_absolute()
    assert tex.path.endswith("ship.png")
    assert tex.filter is None


def test_accepts_pathlike():
    assert fc.Texture(Path(SHIP)) == fc.Texture(SHIP)


def test_missing_file_names_the_file():
    with pytest.raises(FileNotFoundError, match="no_such_image.png"):
        fc.Texture("no_such_image.png")


def test_corrupt_file_names_the_file(tmp_path):
    bad = tmp_path / "broken.png"
    bad.write_bytes(b"not really a png")
    with pytest.raises(ValueError, match="broken.png"):
        fc.Texture(bad)


def test_same_file_is_equal_and_hashes_equal():
    a, b = fc.Texture(SHIP), fc.Texture(SHIP)
    assert a == b
    assert hash(a) == hash(b)


def test_filter_override_is_part_of_identity():
    plain = fc.Texture(SHIP)
    smooth = fc.Texture(SHIP, filter=fc.LINEAR)
    assert smooth.filter == fc.LINEAR
    assert plain != smooth
    assert smooth == fc.Texture(SHIP, filter=fc.LINEAR)


def test_filter_must_be_a_filter():
    with pytest.raises(TypeError):
        fc.Texture(SHIP, filter="linear")


def test_file_read_once_for_many_sprites_and_overrides(tmp_path):
    # A fresh copy, so earlier tests can't have cached it already.
    path = tmp_path / "ship_copy.png"
    shutil.copy(SHIP, path)
    before = _ferrocade._debug_texture_disk_reads()
    sprites = [fc.Sprite(path) for _ in range(1000)]
    fc.Texture(path, filter=fc.LINEAR)
    fc.Texture(path, filter=fc.NEAREST)
    assert _ferrocade._debug_texture_disk_reads() - before == 1
    assert len(sprites) == 1000
