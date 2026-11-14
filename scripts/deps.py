import argparse
import json
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser(description="Generate cplib Cargo dependencies or local overrides")
mode = parser.add_mutually_exclusive_group()
mode.add_argument("--local", action="store_true", help="print path dependencies instead of Git dependencies")
mode.add_argument("--patch", action="store_true", help="print a local [patch] table for existing Git dependencies")
args = parser.parse_args()

manifest = Path(__file__).resolve().parent.parent / "Cargo.toml"
res = subprocess.run(
    ["cargo", "metadata", "--manifest-path", str(manifest), "--format-version", "1", "--no-deps"],
    capture_output=True,
    text=True,
    check=True,
)

meta = json.loads(res.stdout)
github_url = "https://github.com/cacampu/cplib-rs"

crates_root = Path(meta["workspace_root"]) / "crates"

if args.patch:
    print(f'[patch."{github_url}"]')

for item in meta.get("packages", []):
    name = item["name"]
    # manifest_path 内の crates/ 以下のディレクトリ構造をそのまま反映
    crate_dir = Path(item["manifest_path"]).parent
    if not crate_dir.is_relative_to(crates_root):
        continue  # playground は配布用ライブラリの一覧に含めない
    rel = crate_dir.relative_to(crates_root)
    alias = "cplib-" + "-".join(rel.parts)
    if args.patch:
        print(f'{name} = {{ path = {json.dumps(str(crate_dir))} }}')
    elif args.local:
        print(f'{alias} = {{ package = "{name}", path = {json.dumps(str(crate_dir))} }}')
    else:
        print(f'{alias} = {{ package = "{name}", git = "{github_url}" }}')
