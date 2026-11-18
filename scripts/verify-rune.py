"""Replay timer controls in the compiled Rune app and check the rendered tree."""

import json
import subprocess
import sys
import tempfile
from pathlib import Path


def accessible_nodes(node):
    yield node
    for child in node.get("children", []):
        yield from accessible_nodes(child)


def check(binary, directory, name, actions, at, expected, scene="timer"):
    script = directory / f"{name}-input.json"
    tree = directory / f"{name}-tree.json"
    script.write_text(json.dumps(actions), encoding="utf-8")
    subprocess.run(
        [binary, "--scene", scene, "--script", script, "--at", str(at), "--tree", tree],
        check=True,
        timeout=60,
    )
    snapshot = json.loads(tree.read_text(encoding="utf-8"))
    controls = {element.get("key"): element for element in snapshot["elements"]}
    accessible = {node.get("key"): node for node in accessible_nodes(snapshot["accessibility"])}
    for key, field, value in expected:
        if field == "present":
            actual = key in controls
        elif field == "checked":
            actual = "checked" in accessible[key]["states"]
        else:
            actual = controls[key].get(field)
        if actual != value:
            raise AssertionError(f"{name}: {key}.{field} expected {value!r}, got {actual!r}")
    print(f"Passed {name}")


def check_motion(binary, directory):
    script = directory / "avatar-motion.json"
    png = directory / "chamber.png"
    portraits = set()
    for pilot in ("tomato", "luma", "clack"):
        choose = [
            {"t": 0.1, "key": "choose-avatar"}, {"t": 0.25, "key": f"avatar-{pilot}"},
            {"t": 0.4, "type": "key", "name": "Escape"},
        ]
        for motion in (True, False):
            actions = choose.copy()
            if not motion:
                actions += [
                    {"t": 0.55, "key": "settings"}, {"t": 0.7, "key": "tab-options"},
                    {"t": 0.85, "key": "visual-motion"}, {"t": 1, "key": "settings"},
                ]
            script.write_text(json.dumps(actions), encoding="utf-8")
            # Both captures fall in the same break, after its automatic start.
            for scene in ("timer", "focus-finish"):
                snapshots = []
                for at in ("4.25", "4.75"):
                    subprocess.run([
                        binary, "--scene", scene, "--script", script, "--at", at,
                        "--scale", "1", "--crop", "reactor", "--snap", png,
                    ], check=True, timeout=60, capture_output=True)
                    snapshots.append(png.read_bytes())
                if (snapshots[0] != snapshots[1]) != motion:
                    raise AssertionError(f"{pilot} movement in {scene} disagrees with Animation={motion}")
                if not motion and scene == "focus-finish":
                    portraits.add(snapshots[0])
    if len(portraits) != 3:
        raise AssertionError("Two pilots rendered the same celebration")
    print("Passed moving and reduced-motion idle poses and celebrations for all three pilots")


def main():
    binary = Path(sys.argv[1]).resolve(strict=True)
    with tempfile.TemporaryDirectory() as folder:
        directory = Path(folder)
        check(binary, directory, "Boop the tomato without starting the clock", [
            {"t": 0.2, "key": "boop"},
        ], 1.5, [("time-left", "text", "25:00"), ("start-pause", "label", "Start timer")])
        check(binary, directory, "Set motion and interface sounds independently", [
            {"t": 0.1, "key": "settings"},
            {"t": 0.35, "key": "tab-options"},
            {"t": 0.6, "key": "visual-motion"},
            {"t": 0.85, "key": "interface-sounds"},
            {"t": 1.1, "key": "visual-motion"},
        ], 1.5, [("visual-motion", "checked", True), ("interface-sounds", "checked", False)])
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
                 ("start-pause", "label", "Pause timer"), ("focus-complete", "present", False)])
        gallery = [{"t": 0.1, "key": "choose-avatar"}, {"t": 0.3, "key": "avatar-luma"}]
        check(binary, directory, "Choose a pilot", gallery, 0.6, [
            ("avatar-luma", "checked", True), ("avatar-tomato", "checked", False)])
        use_luma = gallery + [{"t": 0.55, "type": "key", "name": "Escape"}]
        check(binary, directory, "Earn a celebration with the selected pilot", use_luma, 3, [
            ("boop", "label", "Boop Luma"), ("focus-complete", "text", "Focus complete!"),
            ("phase", "text", "SHORT BREAK")], "focus-finish")
        check(binary, directory, "Keep the break playable after the celebration", use_luma, 8, [
            ("focus-complete", "present", False), ("start-pause", "label", "Pause timer"),
            ("phase", "text", "SHORT BREAK")], "focus-finish")
        check_motion(binary, directory)


if __name__ == "__main__":
    main()
