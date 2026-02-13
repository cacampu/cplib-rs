import json
import subprocess

res = subprocess.run(
    ["cargo", "metadata", "--format-version", "1", "--no-deps"],
    capture_output=True,
    text=True,
    check=True,
)

meta = json.loads(res.stdout)
package_names = [item["name"] for item in meta.get("packages", [])]

github_url = "https://github.com/cacampu/cplib-rs"

for name in package_names:
    print(f'cplib-{name} = {{ package = "{name}", git = "{github_url}" }}')
