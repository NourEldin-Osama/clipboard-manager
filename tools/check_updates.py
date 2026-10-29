"""Reports newer versions of the crates and GitHub Actions this repo uses.

The `check-updates` prek hook runs it before each push. It only reports and
never blocks. Run it any time with:
    prek run check-updates --hook-stage manual
"""

import json
import os
import re
import subprocess
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def run_cargo(*args: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["cargo", *args], cwd=ROOT, capture_output=True, text=True, timeout=300
    )


def direct_crates() -> set[str]:
    """The crates named in Cargo.toml, as opposed to their dependencies."""
    metadata = run_cargo("metadata", "--format-version", "1", "--no-deps")
    package = json.loads(metadata.stdout)["packages"][0]
    return {dependency["name"] for dependency in package["dependencies"]}


def crate_updates() -> tuple[list[str], list[str]]:
    """Newer crates: the ones `cargo update` takes, and the ones that need
    a new version requirement in Cargo.toml."""
    dry_run = run_cargo("update", "--dry-run", "--verbose")
    if dry_run.returncode != 0:
        raise RuntimeError(dry_run.stderr.strip().splitlines()[-1])
    direct = direct_crates()

    def label(name: str, old: str, new: str) -> str:
        mark = " (in Cargo.toml)" if name in direct else ""
        return f"{name} {old} -> {new}{mark}"

    compatible, breaking = [], []
    for line in dry_run.stderr.splitlines():
        line = line.strip()
        if match := re.match(r"Updating (\S+) v(\S+) -> v(\S+)", line):
            compatible.append(label(*match.groups()))
        elif match := re.match(r"Unchanged (\S+) v(\S+) \(available: v(\S+)\)", line):
            breaking.append(label(*match.groups()))
    return compatible, breaking


def get_json(url: str) -> dict:
    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json",
            # crates.io refuses requests without one.
            "User-Agent": "clipboard-manager check_updates.py",
        },
    )
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token and url.startswith("https://api.github.com/"):
        request.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(request, timeout=10) as response:
        return json.load(response)


def version(text: str) -> tuple[int, ...]:
    """`v7.0.1` or `0.11.0` as numbers, for comparing."""
    return tuple(int(number) for number in re.findall(r"\d+", text))


def action_updates() -> list[str]:
    """Actions pinned to a commit, with the release in a comment
    (`@<commit> # v7.0.1`), that have a newer release."""
    used: dict[str, str] = {}
    for workflow in (ROOT / ".github" / "workflows").glob("*.yml"):
        pins = re.findall(
            r"uses:\s*([\w.-]+/[\w.-]+)@[0-9a-f]{40}\s+#\s*(v\d[\w.]*)", workflow.read_text()
        )
        used.update(pins)
    found = []
    for repo, tag in sorted(used.items()):
        latest = get_json(f"https://api.github.com/repos/{repo}/releases/latest")["tag_name"]
        if version(latest) > version(tag):
            found.append(f"{repo} {tag} -> {latest}")
    return found


def git_crate_updates() -> list[str]:
    """Crates taken from a git commit until their next release: a newer
    release on crates.io means Cargo.toml can go back to it."""
    metadata = run_cargo("metadata", "--format-version", "1", "--locked")
    if metadata.returncode != 0:
        raise RuntimeError(metadata.stderr.strip().splitlines()[-1])
    found = []
    for package in json.loads(metadata.stdout)["packages"]:
        if not (package.get("source") or "").startswith("git+"):
            continue
        crate = get_json(f"https://crates.io/api/v1/crates/{package['name']}")["crate"]
        newest = crate.get("max_stable_version") or crate["max_version"]
        if version(newest) > version(package["version"]):
            found.append(f"{package['name']} {package['version']} from git -> {newest}")
    return found


def show(title: str, lines: list[str]) -> None:
    if lines:
        print(f"{title}:")
        for line in lines:
            print(f"  {line}")


def main() -> None:
    nothing_new = True
    try:
        compatible, breaking = crate_updates()
        show("Crates that `cargo update` would update", compatible)
        show("Crates that need a new version in Cargo.toml", breaking)
        nothing_new = not compatible and not breaking
    except (OSError, RuntimeError, subprocess.SubprocessError, ValueError) as error:
        print(f"Could not check the crates: {error}")
        nothing_new = False
    try:
        from_git = git_crate_updates()
        show("Crates from git with a newer release on crates.io", from_git)
        nothing_new = nothing_new and not from_git
    except (OSError, RuntimeError, subprocess.SubprocessError, ValueError) as error:
        print(f"Could not check the crates from git (offline?): {error}")
        nothing_new = False
    try:
        actions = action_updates()
        show("GitHub Actions with a newer release", actions)
        nothing_new = nothing_new and not actions
    except (OSError, ValueError) as error:
        print(f"Could not check the GitHub Actions (offline?): {error}")
        nothing_new = False
    if nothing_new:
        print("Every crate and GitHub Action is up to date.")


if __name__ == "__main__":
    main()
