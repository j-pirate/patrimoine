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


def bundled_libraries(appdir: Path) -> list[dict]:
    return [{"path": path.relative_to(appdir).as_posix(), "sha256": digest(path.read_bytes())}
            for path in sorted(appdir.rglob("*")) if path.is_file() and ".so" in path.name]


def debian_inventory(stage: Path, appdir: Path, sysroot: Path, hostroot: Path = Path("/")) -> tuple[list[dict], list[str], list[dict]]:
    """Bind binary, version and copyright to the same verified environment.

    hostroot is injectable only for local fixture tests; dpkg-query always reads
    the installed host database. A stale sysroot candidate cannot mask the host.
    """
    with tempfile.TemporaryDirectory(prefix="patrimoine-debian-evidence-") as temporary:
        return _debian_inventory(stage, appdir, sysroot, hostroot, Path(temporary))


def _debian_inventory(stage, appdir, sysroot, hostroot, extracted):
    libraries = [appdir / item["path"] for item in bundled_libraries(appdir)]
    archives = sorted((sysroot.parent / "apt/archives").glob("*.deb"))
    owned = defaultdict(list)
    cached_metadata = {}
    for archive in archives:
        package = subprocess.check_output(["dpkg-deb", "-f", str(archive), "Package"], text=True).strip()
        version = subprocess.check_output(["dpkg-deb", "-f", str(archive), "Version"], text=True).strip()
        source_field = subprocess.check_output(["dpkg-deb", "-f", str(archive), "Source"], text=True).strip()
        source_match = re.fullmatch(r"([^ ()]+)(?: \(([^()]+)\))?", source_field)
        source_name = source_match[1] if source_match else package
        source_version = source_match[2] if source_match and source_match[2] else re.sub(r"\+b[0-9]+$", "", version)
        cached_metadata[archive] = {"version": version, "source_package": source_name, "source_version": source_version,
                                    "binary_metadata_origin": "locally cached Debian archive",
                                    "debian_archive_sha256": digest(archive.read_bytes())}
        listing = subprocess.check_output(["dpkg-deb", "--contents", str(archive)], text=True)
        for line in listing.splitlines():
            fields = line.split()
            if len(fields) >= 6 and ".so" in Path(fields[5]).name:
                owned[Path(fields[5]).name].append((package, fields[5].lstrip("./"), "sysroot", archive))
    roots = {"sysroot": sysroot, "host": hostroot}
    selected = {}
    unmapped = []
    build_ids = {}
    archive_roots = {}

    def archive_root(archive):
        if archive not in archive_roots:
            directory = extracted / str(len(archive_roots))
            subprocess.check_output(["dpkg-deb", "--extract", str(archive), str(directory)])
            archive_roots[archive] = directory
        return archive_roots[archive]

    def build_id(path):
        path = path.resolve()
        if path not in build_ids:
            process = subprocess.run(["readelf", "--notes", str(path)], capture_output=True, text=True)
            found = re.search(r"Build ID:\s*([0-9a-f]+)", process.stdout) if process.returncode == 0 else None
            build_ids[path] = found.group(1) if found else None
        return build_ids[path]

    for bundled_path in libraries:
        library = bundled_path.name
        matches = list(owned.get(library, []))
        found = subprocess.run(["dpkg-query", "-S", f"*/{library}"], capture_output=True, text=True)
        for line in found.stdout.splitlines():
            if ": " in line:
                package, source = line.split(": ", 1)
                # Retain the architecture qualifier when querying installed metadata.
                matches.append((package, source.lstrip("/"), "host", None))
        verified = False
        for package, relative, origin, archive in matches:
            source = roots[origin] / relative
            if not source.is_file() or not source.resolve().is_relative_to(roots[origin].resolve()):
                continue  # A sysroot symlink to the host does not prove archive provenance.
            first, second = build_id(bundled_path), build_id(source)
            method = "GNU build-id" if first and first == second else None
            if method is None and digest(bundled_path.read_bytes()) == digest(source.read_bytes()):
                method = "identical SHA-256"
            if method is None:
                continue
            if origin == "sysroot":
                evidence_root = archive_root(archive)
                archive_file = evidence_root / relative
                # Multiple versions can coexist in apt/archives. A sysroot
                # filename alone cannot bind its bytes to any one of them.
                if (not archive_file.is_file() or not archive_file.resolve().is_relative_to(evidence_root.resolve())
                        or digest(source.read_bytes()) != digest(archive_file.read_bytes())):
                    continue
                metadata = cached_metadata[archive]
            else:
                evidence_root = hostroot
                data = subprocess.check_output(["dpkg-query", "-W", "-f=${Version}\t${source:Package}\t${source:Version}", package], text=True).strip().split("\t")
                if len(data) != 3 or not all(data):
                    raise ValueError(f"Incomplete installed Debian source metadata for {package}")
                metadata = {"version": data[0], "source_package": data[1], "source_version": data[2],
                            "binary_metadata_origin": "installed Debian package database"}
            name = package.split(":", 1)[0]
            os_release = roots[origin] / "etc/os-release"
            ubuntu = os_release.is_file() and re.search(r'^ID=["\']?ubuntu["\']?$', os_release.read_text(), re.M)
            ecosystem = "ubuntu" if ubuntu else "debian"
            source_name = quote(metadata['source_package'], safe='')
            source_version = quote(metadata['source_version'], safe='')
            source_url = (f"https://launchpad.net/ubuntu/+source/{source_name}/{source_version}" if ubuntu
                          else f"https://sources.debian.org/src/{source_name}/{source_version}/")
            key = (name, origin, metadata["version"])
            record = selected.setdefault(key, {"ecosystem": ecosystem, "name": name, **metadata,
                "package_environment": origin,
                "source_browser_url": source_url,
                "scope": ["shared libraries actually bundled in AppDir"], "bundled_libraries": [], "binary_evidence": [],
                "declared_licence": "See Debian machine-readable copyright text", "files": [], "_evidence_root": evidence_root})
            if record["source_package"] != metadata["source_package"] or record["source_version"] != metadata["source_version"]:
                raise ValueError(f"Conflicting Debian source metadata for {package}")
            record["bundled_libraries"].append(library)
            record["binary_evidence"].append({"bundled_path": bundled_path.relative_to(appdir).as_posix(), "package_file": relative,
                "package_environment": origin, "method": method, "build_id": first, "bundled_sha256": digest(bundled_path.read_bytes())})
            verified = True
            break
        if not verified:
            unmapped.append(bundled_path.relative_to(appdir).as_posix())
    records = []
    for (package, origin, version), record in sorted(selected.items()):
        evidence_root = record.pop("_evidence_root")
        copyright_file = evidence_root / "usr/share/doc" / package / "copyright"
        # Never substitute the other environment's copyright, even if it exists.
        if copyright_file.is_file() and copyright_file.resolve().is_relative_to(evidence_root.resolve()):
            record["files"] = copied_files([copyright_file], copyright_file.parent,
                stage / "LICENSES/debian" / origin / f"{safe_name(package)}-{safe_name(version)}", stage)
        record["copyright_environment"] = origin
        record["bundled_libraries"] = sorted(set(record["bundled_libraries"]))
        records.append(record)
    common = []
    for origin in sorted({record["package_environment"] for record in records}):
        directory = roots[origin] / "usr/share/common-licenses"
        if directory.is_dir():
            common += copied_files([path for path in sorted(directory.iterdir()) if path.is_file()], directory,
                stage / "LICENSES/debian/common-licenses" / origin, stage)
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
                    "application_licence": "GPL-3.0-only",
                    "application_licence_file": {"path": "APPLICATION-LICENSE.txt", "sha256": digest((ROOT / "LICENSE").read_bytes()), "size_bytes": (ROOT / "LICENSE").stat().st_size},
                    "cargo_lock_sha256": digest((ROOT / "Cargo.lock").read_bytes()), "package_lock_sha256": digest((ROOT / "package-lock.json").read_bytes()),
                    "appdir_scanned": args.appdir is not None,
                    "appdir_bundled_libraries": bundled_libraries(args.appdir.resolve()) if args.appdir else [],
                    "unmapped_bundled_libraries": unmapped, "missing_licence_files": missing,
                    "debian_common_licence_files": common, "packages": packages}
        if missing or unmapped:
            raise ValueError(f"Incomplete notices: missing={missing}; unmapped={unmapped}")
        (stage / "APPLICATION-LICENSE.txt").write_bytes((ROOT / "LICENSE").read_bytes())
        (stage / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
        lines = ["PATRIMOINE — THIRD-PARTY NOTICES", "", manifest["scope"], "", manifest["application_licence"], "",
                 "Exact resolved versions, source URLs, evidence and file hashes: manifest.json.",
                 "Complete collected licence/copyright texts: LICENSES/.",
                 "MPL-covered crates are listed with versioned source archive URLs; local modifications are not made to their source in this application.",
                 "Bundled Debian/Ubuntu libraries are identified by GNU build-id or identical bytes; their source package and exact source version are recorded with a Debian Sources or Launchpad link.",
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
