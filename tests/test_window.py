import traceback

import pytest

import ferrocade as fc


class StopTest(Exception):
    """Raised from a callback to end a display test's event loop."""


def make_window(**kwargs):
    kwargs.setdefault("filter", fc.NEAREST)
    return fc.Window(320, 240, "test", **kwargs)


# --- No display needed --------------------------------------------------------------------


def test_filter_is_required():
    with pytest.raises(TypeError, match="filter"):
        fc.Window(800, 600, "x")


def test_filter_must_be_a_filter():
    with pytest.raises(TypeError):
        fc.Window(800, 600, "x", filter="nearest")


@pytest.mark.parametrize(
    "kwargs",
    [dict(width=0), dict(height=0), dict(width=-5), dict(update_rate=0), dict(update_rate=-1.0)],
)
def test_invalid_config_raises_value_error(kwargs):
    args = dict(width=800, height=600, title="x", filter=fc.NEAREST)
    args.update(kwargs)
    with pytest.raises(ValueError):
        fc.Window(**args)


def test_subclass_super_init_and_properties_before_run():
    class Game(fc.Window):
        def __init__(self):
            super().__init__(800, 600, "Title", filter=fc.LINEAR, update_rate=1 / 30)
            # Available immediately, before run() opens the OS window.
            self.center = (self.width / 2, self.height / 2)

    game = Game()
    assert (game.width, game.height) == (800, 600)
    assert game.center == (400.0, 300.0)
    assert game.title == "Title"
    assert game.filter == fc.LINEAR
    assert game.update_rate == pytest.approx(1 / 30)


def test_defaults():
    window = fc.Window(filter=fc.NEAREST)
    assert (window.width, window.height, window.title) == (800, 600, "Ferrocade")
    assert window.update_rate == pytest.approx(1 / 60)


def test_clear_outside_on_draw_raises():
    with pytest.raises(RuntimeError, match="on_draw"):
        make_window().clear()


# --- Needs a display ----------------------------------------------------------------------


@pytest.mark.display
def test_callbacks_are_called():
    class Game(fc.Window):
        def __init__(self):
            super().__init__(320, 240, "callbacks", filter=fc.NEAREST, update_rate=1 / 60)
            self.draws = 0
            self.deltas = []

        def on_update(self, delta_time):
            self.deltas.append(delta_time)

        def on_draw(self):
            self.clear(fc.color.BLACK)
            self.clear((10, 20, 30, 255))
            self.draws += 1
            if self.draws >= 30:
                raise StopTest

    game = Game()
    with pytest.raises(StopTest):
        game.run()
    assert game.draws == 30
    assert game.deltas, "on_update was never called"
    assert all(d == pytest.approx(1 / 60) for d in game.deltas)


@pytest.mark.display
def test_exception_propagates_with_user_traceback():
    class Boom(fc.Window):
        def on_draw(self):
            raise ValueError("boom")

    with pytest.raises(ValueError, match="boom") as excinfo:
        Boom(320, 240, "boom", filter=fc.NEAREST).run()
    frames = traceback.extract_tb(excinfo.value.__traceback__)
    assert any(f.name == "on_draw" for f in frames)


@pytest.mark.display
def test_invalid_clear_color_raises_inside_frame():
    class Bad(fc.Window):
        def on_draw(self):
            self.clear((300, 0, 0))

    with pytest.raises(ValueError, match="0-255"):
        Bad(320, 240, "bad", filter=fc.NEAREST).run()


@pytest.mark.display
def test_run_can_be_called_again():
    class Short(fc.Window):
        def on_draw(self):
            raise StopTest

    for _ in range(2):
        with pytest.raises(StopTest):
            Short(320, 240, "again", filter=fc.NEAREST).run()
