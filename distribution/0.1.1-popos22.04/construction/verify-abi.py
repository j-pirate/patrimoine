#!/usr/bin/env python3
"""Read-only GLIBC/GLIBCXX/CXXABI audit against the running OS providers.

Run inside the target Ubuntu 22.04 container. No inspected ELF is executed and
ldd is never used. Only the trusted system readelf and ldconfig are invoked.
This tests versioned ABI imports, not the complete runtime dependency closure.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys


FAMILIES = ("GLIBC", "GLIBCXX", "CXXABI")
VERSION = re.compile(r"^(GLIBC|GLIBCXX|CXXABI)_(.+)$")
NUMBERED_VERSION = re.compile(r"^(GLIBC|GLIBCXX|CXXABI)_([0-9]+(?:\.[0-9]+)*)$")
NEED_FILE = re.compile(r"Version:\s+\d+\s+File:\s+(\S+)\s+Cnt:\s+\d+")
NEED_NAME = re.compile(r"Name:\s+(\S+)\s+Flags:\s+(.*?)\s+Version:\s+(\d+)")
DEFINITION = re.compile(r"Rev:\s+\d+\s+Flags:.*?\bName:\s+(\S+)")
VERSIONED_SYMBOL = re.compile(r"^(.*?)@{1,2}([^\s@]+)(?:\s+\((\d+)\))?$")
ENV = {key: value for key, value in os.environ.items()
       if key not in ("LD_LIBRARY_PATH", "LD_PRELOAD", "LD_AUDIT")}
ENV["LC_ALL"] = "C"


def run(command: list[str]) -> str:
    result = subprocess.run(command, env=ENV, capture_output=True, text=True,
                            timeout=120, check=False)
    if result.returncode:
        raise ValueError(f"{command[0]} exited {result.returncode}: {result.stderr.strip()}")
    return result.stdout


def sha256(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            value.update(block)
    return value.hexdigest()


def os_release() -> dict[str, str]:
    data = {}
    for line in Path("/etc/os-release").read_text().splitlines():
        if line and not line.startswith("#") and "=" in line:
            name, raw = line.split("=", 1)
            values = shlex.split(raw)
            data[name] = values[0] if values else ""
    return data


def maxima(names) -> dict[str, str | None]:
    found = {family: [] for family in FAMILIES}
    for name in names:
        match = NUMBERED_VERSION.fullmatch(name)
        if match:
            found[match[1]].append((tuple(map(int, match[2].split("."))), name))
    return {family: max(values)[1] if values else None
            for family, values in found.items()}


def version_key(name: str):
    match = NUMBERED_VERSION.fullmatch(name)
    return (match[1], 0, tuple(map(int, match[2].split(".")))) if match else (name, 1, ())


def inspect(path: Path) -> dict:
    output = run(["/usr/bin/readelf", "--wide", "--file-header", "--version-info",
                  "--dyn-syms", "--", str(path)])
    machine = re.search(r"^\s*Machine:\s+(.+)$", output, re.MULTILINE)
    elf_class = re.search(r"^\s*Class:\s+(.+)$", output, re.MULTILINE)
    requirements = []
    definitions = set()
    exports = defaultdict(set)
    imports = []
    section = None
    needed_file = None
    for line in output.splitlines():
        if line.startswith("Version needs section"):
            section = "needs"
            continue
        if line.startswith("Version definition section"):
            section = "definitions"
            continue
        if line.startswith("Version symbols section"):
            section = None
            continue
        if line.startswith("Symbol table '"):
            section = "dynsym" if "'.dynsym'" in line else None
            continue
        if section == "needs":
            found = NEED_FILE.search(line)
            if found:
                needed_file = found[1]
            found = NEED_NAME.search(line)
            if found and VERSION.fullmatch(found[1]):
                if needed_file is None:
                    raise ValueError("version need has no provider filename")
                requirements.append({"version": found[1], "needed_from": needed_file,
                                     "flags": found[2], "index": int(found[3]),
                                     "weak": "WEAK" in found[2].split()})
        elif section == "definitions":
            found = DEFINITION.search(line)
            if found and VERSION.fullmatch(found[1]):
                definitions.add(found[1])
        elif section == "dynsym":
            fields = line.split(None, 7)
            if len(fields) != 8 or not re.fullmatch(r"\d+:", fields[0]):
                continue
            found = VERSIONED_SYMBOL.fullmatch(fields[7].strip())
            if not found or not VERSION.fullmatch(found[2]):
                continue
            symbol, version, index = found.groups()
            if fields[6] == "UND":
                imports.append({"symbol": symbol, "version": version,
                                "index": int(index) if index else None,
                                "weak": fields[4] == "WEAK"})
            elif fields[4] in ("GLOBAL", "WEAK", "UNIQUE") and fields[5] in ("DEFAULT", "PROTECTED"):
                exports[version].add(symbol)
    if not machine or not elf_class:
        raise ValueError("cannot read ELF class/machine")
    return {"machine": machine[1].strip(), "class": elf_class[1].strip(),
            "requirements": requirements, "definitions": definitions,
            "exports": exports, "imports": imports}


def system_provider_paths() -> dict[str, list[Path]]:
    paths = defaultdict(list)
    ldconfig = next((path for path in ("/sbin/ldconfig", "/usr/sbin/ldconfig")
                     if Path(path).is_file()), None)
    if ldconfig:
        for line in run([ldconfig, "-p"]).splitlines():
            found = re.match(r"\s*(\S+)\s+\(([^)]*)\)\s+=>\s+(\S+)", line)
            if found and "x86-64" in found[2]:
                paths[found[1]].append(Path(found[3]))
    # The dynamic loader may not appear in ld.so.cache; include its native paths.
    for directory in ("/lib/x86_64-linux-gnu", "/usr/lib/x86_64-linux-gnu", "/lib64", "/usr/lib64"):
        for soname in ("libc.so.6", "libm.so.6", "libpthread.so.0", "libdl.so.2",
                       "librt.so.1", "libstdc++.so.6", "ld-linux-x86-64.so.2"):
            path = Path(directory) / soname
            if path.is_file():
                paths[soname].append(path)
    return paths


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("appdir", type=Path)
    parser.add_argument("--output", type=Path, help="JSON destination; stdout if omitted")
    parser.add_argument("--expected-os", default="ubuntu:22.04",
                        help="required ID:VERSION_ID, default ubuntu:22.04")
    args = parser.parse_args()
    appdir = args.appdir.resolve()
    if not appdir.is_dir():
        parser.error("AppDir does not exist")
    native_os = os_release()
    actual_os = f"{native_os.get('ID')}:{native_os.get('VERSION_ID')}"
    if actual_os != args.expected_os:
        parser.error(f"run inside target container: expected {args.expected_os}, found {actual_os}")
    providers = {}
    provider_details = {}
    provider_paths = system_provider_paths()
    failures = []
    optional_absences = []
    objects = []
    all_versions = {"strong": set(), "weak": set()}
    skipped_symlinks = 0

    def provider_for(soname: str):
        if soname in providers:
            return providers[soname]
        candidates = []
        seen = set()
        for candidate in provider_paths.get(soname, []):
            if not candidate.is_file():
                continue
            path = candidate.resolve()
            if path in seen:
                continue
            seen.add(path)
            details = inspect(path)
            if details["class"] == "ELF64" and details["machine"] == "Advanced Micro Devices X86-64":
                candidates.append((path, details))
        if not candidates:
            providers[soname] = None
            provider_details[soname] = {"path": None, "available": False}
            return None
        # Respect ldconfig's order for x86-64 providers; fallback paths follow it.
        path, details = candidates[0]
        providers[soname] = details
        provider_details[soname] = {
            "path": str(path), "available": True, "sha256": sha256(path),
            "machine": details["machine"], "class": details["class"],
            "provided_versions": sorted(details["definitions"], key=version_key),
            "max_provided": maxima(details["definitions"]),
            "versioned_export_count": sum(map(len, details["exports"].values())),
        }
        return details

    for path in sorted(appdir.rglob("*")):
        if path.is_symlink():
            skipped_symlinks += 1
            continue
        if not path.is_file():
            continue
        relative = path.relative_to(appdir).as_posix()
        try:
            with path.open("rb") as stream:
                if stream.read(4) != b"\x7fELF":
                    continue
            data = inspect(path)
            obj = {"path": relative, "sha256": sha256(path), "machine": data["machine"],
                   "class": data["class"], "requirements": {"strong": [], "weak": []}}
            objects.append(obj)
            if data["class"] != "ELF64" or data["machine"] != "Advanced Micro Devices X86-64":
                failures.append({"path": relative, "kind": "unexpected_elf_architecture"})
                continue
            needs_by_index = {item["index"]: item for item in data["requirements"]}
            symbol_imports = defaultdict(list)
            for item in data["imports"]:
                need = needs_by_index.get(item["index"])
                if need is None:
                    matches = [candidate for candidate in data["requirements"]
                               if candidate["version"] == item["version"]]
                    need = matches[0] if len(matches) == 1 else None
                if need is None or need["version"] != item["version"]:
                    raise ValueError(f"unresolved version provider for import {item['symbol']}@{item['version']}")
                symbol_imports[need["index"]].append(item)
            for need in data["requirements"]:
                strength = "weak" if need["weak"] else "strong"
                all_versions[strength].add(need["version"])
                provider = provider_for(need["needed_from"])
                available = provider is not None and need["version"] in provider["definitions"]
                symbols = {"strong": [], "weak": []}
                missing_symbols = {"strong": [], "weak": []}
                exports = provider["exports"].get(need["version"], set()) if provider else set()
                for item in symbol_imports[need["index"]]:
                    binding = "weak" if item["weak"] else "strong"
                    symbols[binding].append(item["symbol"])
                    if item["symbol"] not in exports:
                        missing_symbols[binding].append(item["symbol"])
                record = {"needed_from": need["needed_from"], "version": need["version"],
                          "version_need_flags": need["flags"], "version_available": available,
                          "symbols": {key: sorted(set(value)) for key, value in symbols.items()},
                          "missing_symbols": {key: sorted(set(value)) for key, value in missing_symbols.items()}}
                obj["requirements"][strength].append(record)
                context = {"path": relative, "needed_from": need["needed_from"], "version": need["version"]}
                if not available:
                    event = {**context, "kind": "missing_version"}
                    (optional_absences if need["weak"] else failures).append(event)
                if missing_symbols["strong"]:
                    failures.append({**context, "kind": "missing_strong_symbols",
                                     "symbols": sorted(set(missing_symbols["strong"]))})
                if missing_symbols["weak"]:
                    optional_absences.append({**context, "kind": "missing_weak_symbols",
                                             "symbols": sorted(set(missing_symbols["weak"]))})
            obj["max_required"] = {strength: maxima(item["version"] for item in requirements)
                                   for strength, requirements in obj["requirements"].items()}
        except (OSError, ValueError, subprocess.TimeoutExpired) as error:
            failures.append({"path": relative, "kind": "inspection_error", "error": str(error)})

    if not objects:
        failures.append({"kind": "no_elf_files"})
    report = {
        "schema_version": 1, "appdir": str(appdir), "target_os": native_os,
        "compatible": not failures,
        "scope": "GLIBC/GLIBCXX/CXXABI version definitions and corresponding dynamic symbol imports of all regular ELF files in AppDir",
        "provider_policy": "Native target OS providers from ldconfig, with standard native loader fallbacks; bundled providers do not widen the OS ABI baseline",
        "weak_policy": "Missing WEAK version needs or WEAK symbol imports are optional absences; a missing strong version need or strong symbol import is incompatible",
        "limitations": "Static ABI inspection only; does not prove runtime behavior, dlopen resolution, complete dependencies, or that optional calls have correct fallbacks",
        "summary": {"elf_files": len(objects), "symlinks_skipped": skipped_symlinks,
                    "max_required": {key: maxima(values) for key, values in all_versions.items()},
                    "failure_count": len(failures), "optional_absence_count": len(optional_absences)},
        "providers": provider_details, "failures": failures,
        "optional_absences": optional_absences, "objects": objects,
    }
    raw = json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(raw, encoding="utf-8")
        print(json.dumps({"output": str(args.output), "compatible": report["compatible"], **report["summary"]}))
    else:
        sys.stdout.write(raw)
    return 0 if report["compatible"] else 1


if __name__ == "__main__":
    sys.exit(main())
