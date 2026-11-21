"""Run cargo-equip for a bin selected by cargo-compete."""

import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    if len(sys.argv) < 2:
        sys.exit("usage: equip-submit.py BIN [CARGO-EQUIP OPTIONS...]")
    bin_name, *options = sys.argv[1:]

    # cargo-compete runs this command in the workspace root and passes only the bin name.
    metadata = subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"], text=True
    )
    manifests = [
        package["manifest_path"]
        for package in json.loads(metadata)["packages"]
        if any(
            target["name"] == bin_name and "bin" in target["kind"]
            for target in package["targets"]
        )
    ]
    if len(manifests) != 1:
        sys.exit(f"expected one workspace bin named {bin_name!r}, found {len(manifests)}")
    manifest = manifests[0]
    env = os.environ | {"CARGO_MANIFEST_DIR": str(Path(manifest).parent)}
    args = ["cargo", "equip", "--manifest-path", manifest, "--bin", bin_name, *options]
    os.execvpe(args[0], args, env)


if __name__ == "__main__":
    main()
