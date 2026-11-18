"""Replay timer controls in the compiled Rune app and check the rendered tree."""

import json
import subprocess
import sys
import tempfile
from pathlib import Path


def check(binary, directory, name, actions, at, expected):
    script = directory / f"{name}-input.json"
    tree = directory / f"{name}-tree.json"
    script.write_text(json.dumps(actions), encoding="utf-8")
    subprocess.run(
        [binary, "--script", script, "--at", str(at), "--tree", tree],
        check=True,
        timeout=60,
    )
    elements = json.loads(tree.read_text(encoding="utf-8"))["elements"]
    controls = {element.get("key"): element for element in elements}
    for key, field, value in expected:
        actual = controls[key].get(field)
        if actual != value:
            raise AssertionError(f"{name}: {key}.{field} expected {value!r}, got {actual!r}")
    print(f"Passed {name}")


def main():
    binary = Path(sys.argv[1]).resolve(strict=True)
    with tempfile.TemporaryDirectory() as folder:
        directory = Path(folder)
        check(binary, directory, "Pause using the global shortcut", [
            {"t": 0.1, "key": "start-pause"},
            {"t": 2.4, "shortcut": "ctrl+f1"},
        ], 4, [("time-left", "text", "24:58"), ("start-pause", "label", "Resume timer")])
        check(binary, directory, "Edit focus duration with the keyboard", [
            {"t": 0.1, "key": "settings"},
            {"t": 0.35, "key": "duration-focus"},
            {"t": 0.6, "type": "key", "name": "Home"},
            {"t": 0.8, "type": "key", "name": "Right"},
            {"t": 1, "type": "key", "name": "Escape"},
        ], 1.5, [("time-left", "text", "2:00"), ("phase", "text", "FOCUS")])
        check(binary, directory, "Skip focus and auto-start the break", [
            {"t": 0.1, "key": "skip"},
        ], 3.0, [("phase", "text", "SHORT BREAK"), ("time-left", "text", "4:59"),
                 ("start-pause", "label", "Pause timer")])


if __name__ == "__main__":
    main()
