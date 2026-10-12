import os

import pytest


def pytest_collection_modifyitems(config, items):
    if os.environ.get("FERROCADE_HEADLESS") != "1":
        return
    skip = pytest.mark.skip(reason="FERROCADE_HEADLESS=1: no display available")
    for item in items:
        if "display" in item.keywords:
            item.add_marker(skip)
