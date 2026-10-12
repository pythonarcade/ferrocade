import ferrocade as fc
from ferrocade import color


def test_148_css_colors_all_exposed_as_constants():
    assert len(color.CSS_COLORS) == 148
    for keyword, value in color.CSS_COLORS.items():
        names = [n for n in dir(color) if n.isupper() and n.replace("_", "").lower() == keyword]
        assert names, f"no constant for {keyword}"
        assert getattr(color, names[0]) == value


def test_values_are_rgb_int_tuples():
    for value in color.CSS_COLORS.values():
        assert len(value) == 3
        assert all(isinstance(c, int) and 0 <= c <= 255 for c in value)


def test_gray_grey_pairs_match():
    grays = [n for n in dir(color) if "GRAY" in n]
    assert grays
    for name in grays:
        assert getattr(color, name.replace("GRAY", "GREY")) == getattr(color, name)


def test_spec_values():
    assert color.CORNFLOWER_BLUE == (100, 149, 237)
    assert color.REBECCA_PURPLE == (102, 51, 153)
    assert color.GREEN == (0, 128, 0)
    assert color.LIME == (0, 255, 0)
    assert color.DARK_GRAY == (169, 169, 169)
    assert color.TRANSPARENT == (0, 0, 0, 0)
    assert color.DEFAULT_BACKGROUND == (32, 32, 32)


def test_exported_from_package():
    assert fc.color is color
