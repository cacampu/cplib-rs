just_dir := justfile_directory()

new lib-name:
    cargo new --vcs none --lib {{lib-name}}

deps:
    python3 {{just_dir}}/scripts/deps.py

local-deps:
    python3 {{just_dir}}/scripts/deps.py --local

local-patch:
    python3 {{just_dir}}/scripts/deps.py --patch

problem url alias:
    cd {{just_dir}}/playground && cargo compete add '{{url}}' '{{alias}}'

try name:
    cd {{just_dir}}/playground && cargo compete test '{{name}}'

bundle name:
    cd {{just_dir}}/playground && CARGO_MANIFEST_DIR="{{just_dir}}/playground" cargo equip --exclude-atcoder-202510-crates --remove docs --minify libs --bin '{{name}}' --output ../target/{{name}}-bundled.rs

test:
    cargo test --workspace
