#!/usr/bin/env python3
"""Collect installed release dependency notices without contacting the network.

Run after npm ci and cargo fetch, from a shell where Cargo is available.
The output includes Cargo's Linux normal/build/procedural-macro dependency graph
and npm's production dependency closure. It does not assert that a compiler or
type-only package is linked into the final binary. --appdir adds the copyrights
of Debian packages whose shared libraries are actually present in that AppDir.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
from urllib.parse import quote


ROOT = Path(__file__).resolve().parents[1]
MARKER = "scripts/collect-third-party-licences.py"
PREFIXES = ("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT")


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def safe_name(value: str) -> str:
    return re.sub(r"[^A-Za-z0-9_.-]", "_", value)


def licence_files(directory: Path, declared: str | None = None) -> list[Path]:
    result = set()
    if declared:
        candidate = directory / declared
        if candidate.is_file() and candidate.resolve().is_relative_to(directory.resolve()):
            result.add(candidate)
    for path in directory.rglob("*"):
        relative = path.relative_to(directory)
        if len(relative.parts) > 3 or any(part in ("node_modules", "target", ".git") for part in relative.parts):
            continue
        if path.is_file() and not path.is_symlink() and path.name.upper().startswith(PREFIXES):
            result.add(path)
    return sorted(result)


def copied_files(paths: list[Path], directory: Path, destination: Path, stage: Path) -> list[dict]:
    records = []
    for path in paths:
        relative = path.relative_to(directory)
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        raw = path.read_bytes()
        target.write_bytes(raw)
        records.append({"path": target.relative_to(stage).as_posix(), "sha256": digest(raw), "size_bytes": len(raw)})
    return records


def cargo_inventory(stage: Path, metadata_file: Path | None) -> list[dict]:
    if metadata_file:
        metadata = json.loads(metadata_file.read_text())
    else:
        raw = subprocess.check_output([
            "cargo", "metadata", "--format-version", "1", "--locked", "--offline",
            "--filter-platform", "x86_64-unknown-linux-gnu", "--features", "patrimoine-desktop/custom-protocol",
        ], cwd=ROOT)
        metadata = json.loads(raw)
    packages = {package["id"]: package for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    root = next(package["id"] for package in packages.values() if package["name"] == "patrimoine-desktop")
    scopes: dict[str, set[str]] = defaultdict(set)
    pending = [(root, "runtime")]
    while pending:
        identity, scope = pending.pop()
        if scope in scopes[identity]:
            continue
        scopes[identity].add(scope)
        for dependency in nodes[identity]["deps"]:
            for kind in dependency["dep_kinds"]:
                if kind["kind"] == "dev":
                    continue
                child = packages[dependency["pkg"]]
                macro = any("proc-macro" in target["kind"] for target in child["targets"])
                child_scope = "build" if scope == "build" or kind["kind"] == "build" or macro else "runtime"
                pending.append((dependency["pkg"], child_scope))
    result = []
    for identity in sorted(scopes, key=lambda key: (packages[key]["name"], packages[key]["version"])):
        package = packages[identity]
        if not package["source"]:
            continue  # The application's own licence is not granted here.
        directory = Path(package["manifest_path"]).parent
        target = stage / "LICENSES" / "rust" / f"{package['name']}-{package['version']}"
        files = copied_files(licence_files(directory, package.get("license_file")), directory, target, stage)
        record = {
            "ecosystem": "cargo", "name": package["name"], "version": package["version"],
            "declared_licence": package.get("license"), "scope": sorted(scopes[identity]),
            "source": f"https://static.crates.io/crates/{package['name']}/{package['name']}-{package['version']}.crate",
            "repository": package.get("repository"), "files": files,
        }
        supplemental_root = ROOT / "resources/licence-sources"
        supplemental_manifest = supplemental_root / "manifest.json"
        if not files and supplemental_manifest.is_file():
            evidence = [item for item in json.loads(supplemental_manifest.read_text())["files"]
                        if item["ecosystem"] == "cargo" and item["name"] == package["name"] and item["version"] == package["version"]]
            for item in evidence:
                path = supplemental_root / item["path"]
                if not path.resolve().is_relative_to(supplemental_root.resolve()) or digest(path.read_bytes()) != item["sha256"]:
                    raise ValueError("Supplemental licence provenance/hash mismatch")
                files.extend(copied_files([path], path.parent, target, stage))
            if evidence:
                record["upstream_licence_evidence"] = evidence
        result.append(record)
    # The published selectors crate omits its generic MPL text. Copy that exact
    # standard text from another installed MPL crate, and disclose the origin.
    missing_selectors = next((p for p in result if p["name"] == "selectors" and not p["files"]), None)
    if missing_selectors:
        canonical = next((p for p in result if p["name"] == "cssparser" and p["declared_licence"] == "MPL-2.0" and p["files"]), None)
        if canonical:
            source = stage / canonical["files"][0]["path"]
            target = stage / "LICENSES/rust" / f"selectors-{missing_selectors['version']}" / "MPL-2.0.txt"
            target.parent.mkdir(parents=True, exist_ok=True)
            raw = source.read_bytes(); target.write_bytes(raw)
            missing_selectors["files"] = [{"path": target.relative_to(stage).as_posix(), "sha256": digest(raw), "size_bytes": len(raw)}]
            missing_selectors["licence_text_provenance"] = f"Published selectors crate has no licence file; generic MPL-2.0 text copied unchanged from cssparser {canonical['version']} LICENSE. Exact selectors source is available at the recorded crate URL."
    return result


def npm_inventory(stage: Path) -> list[dict]:
    lock = json.loads((ROOT / "package-lock.json").read_text())
    result = []
    for relative, package in sorted(lock["packages"].items()):
        if not relative or package.get("dev"):
            continue
        directory = ROOT / relative
        installed = json.loads((directory / "package.json").read_text())
        name = installed["name"]
        if installed["version"] != package["version"]:
            raise ValueError(f"Installed npm version differs from lockfile: {name}")
        target = stage / "LICENSES/npm" / f"{safe_name(name)}-{package['version']}"
        result.append({
            "ecosystem": "npm", "name": name, "version": package["version"],
            "declared_licence": package.get("license") or installed.get("license"),
            "scope": ["production dependency closure"],
            "type_only_package": name.startswith("@types/"),
            "source": package.get("resolved"), "lock_integrity": package.get("integrity"),
            "files": copied_files(licence_files(directory), directory, target, stage),
        })
    return result


def appimage_inventory(stage: Path, appdir: Path | None) -> list[dict]:
    """Copy pinned upstream runtime/launcher notices, including static libraries.

    These components are invisible to a scan of bundled .so files. Unknown
    transitive binary versions remain explicitly unknown in the evidence.
    """
    directory = ROOT / "resources/licence-sources/appimage"
    evidence = json.loads((directory / "manifest.json").read_text())
    runtime_hash = evidence["runtime_sha256"]
    if runtime_hash not in (ROOT / "scripts/fetch-appimage-runtime.sh").read_text():
        raise ValueError("AppImage runtime pin changed: review its static notices")
    if appdir:
        launcher = appdir / "AppRun.wrapped"
        if not launcher.is_file():
            launcher = appdir / "AppRun"
        if digest(launcher.read_bytes()) != evidence["launcher_sha256"]:
            raise ValueError("AppImage launcher changed: review its upstream notices")
    records = []
    for item in evidence["packages"]:
        record = {key: value for key, value in item.items() if key != "files"}
        record["upstream_notice_evidence"] = item["files"]
        record["files"] = []
        for entry in item["files"]:
            source = directory / entry["path"]
            if not source.resolve().is_relative_to(directory.resolve()) or digest(source.read_bytes()) != entry["sha256"]:
                raise ValueError("AppImage licence provenance/hash mismatch")
            destination = stage / "LICENSES/appimage" / item["name"]
            record["files"].extend(copied_files([source], source.parent, destination, stage))
        records.append(record)
    return records


def debian_inventory(stage: Path, appdir: Path, sysroot: Path) -> tuple[list[dict], list[str], list[dict]]:
    """Map actual bundled .so names to local Debian package file inventories."""
    libraries = sorted({path.name: path for path in appdir.rglob("*") if path.is_file() and ".so" in path.name}.values())
    # dpkg owns host files. The non-privileged sysroot was extracted from these
    # locally cached Debian archives, so read their file lists when needed.
    archives = sorted((sysroot.parent / "apt/archives").glob("*.deb"))
    owned: dict[str, list[tuple[str, str]]] = defaultdict(list)
    cached_metadata = {}
    for archive in archives:
        package = subprocess.check_output(["dpkg-deb", "-f", str(archive), "Package"], text=True).strip()
        version = subprocess.check_output(["dpkg-deb", "-f", str(archive), "Version"], text=True).strip()
        source_field = subprocess.check_output(["dpkg-deb", "-f", str(archive), "Source"], text=True).strip()
        source_match = re.fullmatch(r"([^ ()]+)(?: \(([^()]+)\))?", source_field)
        source_name = source_match[1] if source_match else package
        # A Debian binary-only rebuild adds +bN to the binary version while
        # retaining the original source version unless Source names it explicitly.
        source_version = source_match[2] if source_match and source_match[2] else re.sub(r"\+b[0-9]+$", "", version)
        cached_metadata[package] = {"version": version, "source_package": source_name, "source_version": source_version,
                                    "binary_metadata_origin": "locally cached Debian archive"}
        listing = subprocess.check_output(["dpkg-deb", "--contents", str(archive)], text=True)
        for line in listing.splitlines():
            fields = line.split()
            if len(fields) >= 6:
                path = fields[5]
                name = Path(path).name
                if ".so" in name:
                    owned[name].append((package, path.lstrip("./")))
    selected: dict[str, list[str]] = defaultdict(list)
    binary_evidence: dict[str, list[dict]] = defaultdict(list)
    selected_metadata = {}
    unmapped = []
    build_ids: dict[Path, str | None] = {}
    def build_id(path: Path) -> str | None:
        path = path.resolve()
        if path not in build_ids:
            process = subprocess.run(["readelf", "--notes", str(path)], capture_output=True, text=True)
            found = re.search(r"Build ID:\s*([0-9a-f]+)", process.stdout)
            build_ids[path] = found.group(1) if found else None
        return build_ids[path]
    for bundled_path in libraries:
        library = bundled_path.name
        matches = owned.get(library, [])
        # Host runtime files may not have been downloaded into the sysroot.
        found = subprocess.run(["dpkg-query", "-S", f"*/{library}"], capture_output=True, text=True)
        for line in found.stdout.splitlines():
            if ": " in line:
                package, source = line.split(": ", 1)
                matches.append((package.split(":", 1)[0], source.lstrip("/")))
        verified = set()
        for package, relative in matches:
            source = next((path for path in [sysroot / relative, Path("/") / relative] if path.is_file()), None)
            if source is None:
                continue
            first, second = build_id(bundled_path), build_id(source)
            method = "GNU build-id" if first and first == second else None
            if method is None and digest(bundled_path.read_bytes()) == digest(source.read_bytes()):
                method = "identical SHA-256"
            if method is None or package in verified:
                continue
            verified.add(package)
            if source.resolve().is_relative_to(sysroot.resolve()):
                metadata = cached_metadata.get(package)
                if metadata is None:
                    raise ValueError(f"No cached Debian metadata for {package}")
            else:
                data = subprocess.check_output(["dpkg-query", "-W", "-f=${Version}\t${source:Package}\t${source:Version}", package], text=True).strip().split("\t")
                if len(data) != 3 or not all(data):
                    raise ValueError(f"Incomplete installed Debian source metadata for {package}")
                metadata = {"version": data[0], "source_package": data[1], "source_version": data[2],
                            "binary_metadata_origin": "installed Debian package database"}
            if package in selected_metadata and selected_metadata[package] != metadata:
                raise ValueError(f"Conflicting Debian package versions in bundled libraries: {package}")
            selected_metadata[package] = metadata
            selected[package].append(library)
            binary_evidence[package].append({"bundled_path": bundled_path.relative_to(appdir).as_posix(), "package_file": relative,
                                             "method": method, "build_id": first, "bundled_sha256": digest(bundled_path.read_bytes())})
        if not verified:
            unmapped.append(library)
    records = []
    for package, bundled in sorted(selected.items()):
        paths = [sysroot / "usr/share/doc" / package / "copyright", Path("/usr/share/doc") / package / "copyright"]
        copyright_file = next((path for path in paths if path.is_file()), None)
        metadata = selected_metadata[package]
        files = []
        if copyright_file:
            files = copied_files([copyright_file], copyright_file.parent, stage / "LICENSES/debian" / safe_name(package), stage)
        records.append({"ecosystem": "debian", "name": package, **metadata,
                        "source_browser_url": f"https://sources.debian.org/src/{quote(metadata['source_package'], safe='')}/{quote(metadata['source_version'], safe='')}/",
                        "scope": ["shared libraries actually bundled in AppDir"],
                        "bundled_libraries": sorted(set(bundled)), "binary_evidence": binary_evidence[package],
                        "declared_licence": "See Debian machine-readable copyright text", "files": files})
    # Debian copyright files commonly refer to these complete standard texts.
    # Preserve them in the AppImage notices instead of relying on the target OS.
    common_directory = next((path for path in [sysroot / "usr/share/common-licenses", Path("/usr/share/common-licenses")] if path.is_dir()), None)
    common = []
    if common_directory:
        common = copied_files([path for path in sorted(common_directory.iterdir()) if path.is_file()], common_directory,
                              stage / "LICENSES/debian/common-licenses", stage)
    return records, unmapped, common


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "resources/notices")
    parser.add_argument("--cargo-metadata", type=Path)
    parser.add_argument("--appdir", type=Path)
    parser.add_argument("--sysroot", type=Path, default=Path(os.environ.get("PATRIMOINE_TOOL_ROOT", "/workspace/.tooling")) / "sysroot")
    args = parser.parse_args()
    output = args.output.resolve(); output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        marker = output / "manifest.json"
        if not marker.is_file() or json.loads(marker.read_text()).get("generated_by") != MARKER:
            raise ValueError("Refusing to replace a directory not generated by this script")
    with tempfile.TemporaryDirectory(prefix=".notices-", dir=output.parent) as temporary:
        stage = Path(temporary)
        packages = cargo_inventory(stage, args.cargo_metadata) + npm_inventory(stage)
        unmapped = []
        common = []
        if args.appdir:
            debian, unmapped, common = debian_inventory(stage, args.appdir.resolve(), args.sysroot.resolve())
            packages.extend(debian)
        packages.extend(appimage_inventory(stage, args.appdir))
        missing = [f"{p['ecosystem']}:{p['name']}@{p['version']}" for p in packages if not p["files"]]
        manifest = {"generated_by": MARKER, "format_version": 1, "target": "x86_64-unknown-linux-gnu", "offline": True,
                    "scope": "Cargo normal/build/procedural-macro release graph, npm production closure; excludes dev/test-only and does not claim every listed component is linked",
                    "application_licence": "UNLICENSED; no additional licence grant is made by these third-party notices",
                    "cargo_lock_sha256": digest((ROOT / "Cargo.lock").read_bytes()), "package_lock_sha256": digest((ROOT / "package-lock.json").read_bytes()),
                    "appdir_scanned": args.appdir is not None, "unmapped_bundled_libraries": unmapped, "missing_licence_files": missing,
                    "debian_common_licence_files": common, "packages": packages}
        (stage / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
        lines = ["PATRIMOINE — THIRD-PARTY NOTICES", "", manifest["scope"], "", manifest["application_licence"], "",
                 "Exact resolved versions, source URLs, evidence and file hashes: manifest.json.",
                 "Complete collected licence/copyright texts: LICENSES/.",
                 "MPL-covered crates are listed with versioned source archive URLs; local modifications are not made to their source in this application.",
                 "Bundled Debian libraries are identified by GNU build-id or identical bytes; their source package and exact source version are recorded with a Debian Sources link.",
                 "GTK/GLib/WebKit and other LGPL libraries remain dynamically linked. The AppImage can be extracted and those libraries replaced; their copyright and applicable LGPL texts are included below.",
                 "The pinned Type-2 runtime includes static musl, libfuse, squashfuse, zstd, zlib and mimalloc code; launcher/runtime provenance and licence texts are included separately from the Debian .so inventory. Unknown static dependency versions are disclosed in manifest.json; a licence-text reference version is not proof of the linked binary version.",
                 "Static libfuse relinking/source requirements also apply to the AppImage runtime; extraction alone does not satisfy those requirements.",
                 "These local validation notices do not include a complete corresponding-source archive or create a written source offer. Before external redistribution, the distributor must supply the corresponding sources or another offer permitted by each applicable licence, including Debian patches and any local modifications.", ""]
        lines += [f"{p['ecosystem']}: {p['name']} {p['version']} — {p.get('declared_licence') or 'not declared'}" for p in packages]
        if missing: lines += ["", "Missing packaged licence texts:", *missing]
        if unmapped: lines += ["", "Bundled shared libraries without Debian ownership evidence:", *unmapped]
        if not args.appdir: lines += ["", "The AppDir has not yet been scanned: this inventory alone does not cover shared libraries copied by the packager."]
        (stage / "NOTICE.txt").write_text("\n".join(lines) + "\n")
        for package in packages:
            for entry in package["files"]:
                assert digest((stage / entry["path"]).read_bytes()) == entry["sha256"]
        for entry in common:
            assert digest((stage / entry["path"]).read_bytes()) == entry["sha256"]
        if output.exists(): shutil.rmtree(output)
        shutil.copytree(stage, output)
    print(json.dumps({"output": str(output), "packages": len(packages), "missing_licence_files": missing, "unmapped_bundled_libraries": unmapped, "appdir_scanned": args.appdir is not None}, ensure_ascii=False))


if __name__ == "__main__":
    main()
