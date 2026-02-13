just_dir := justfile_directory()

new lib-name:
    cargo new --vcs none --lib {{lib-name}}

deps:
    python3 {{just_dir}}/scripts/deps.py
