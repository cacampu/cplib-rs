import json
import subprocess
from pathlib import Path

res = subprocess.run(
    ["cargo", "metadata", "--format-version", "1", "--no-deps"],
    capture_output=True,
    text=True,
    check=True,
)

meta = json.loads(res.stdout)
github_url = "https://github.com/cacampu/cplib-rs"

crates_root = Path(meta["workspace_root"]) / "crates"

for item in meta.get("packages", []):
    name = item["name"]
    # manifest_path 内の crates/ 以下のディレクトリ構造をそのまま反映
    rel = Path(item["manifest_path"]).parent.relative_to(crates_root)
    alias = "cplib-" + "-".join(rel.parts)
    print(f'{alias} = {{ package = "{name}", git = "{github_url}" }}')
