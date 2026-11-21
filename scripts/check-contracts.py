#!/usr/bin/env python3
"""Check Rust/Serde DTOs, TypeScript DTOs and the complete Tauri command registry.

Uses syntax trees and the installed TypeScript checker, never regex extraction.
Run after dependency installation; the Rust helper builds offline with its lock.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
HELPER = ROOT / "scripts/contract-check"
ALIASES = {"PostDebtLinkInput": "PostedDebtCashLink", "RefundAllocationInput": "RefundRentAllocationInput"}
# Token fingerprints (formatting/comments do not affect them). These reviewed
# macros expand to the id/revision fields and invocation fields extracted by syn.
MACROS = {
    "record": "6a8822cbdb1c1f94a9ccf3d0ad2a625ed965f6400f5319dc3539957d9a7953c5",
    "property_record": "875dde6fb997a08df34fcee19d19491d577a536607a377d3813e4f4873ff97f5",
    **dict.fromkeys(("company_record", "debt_record", "finance_record", "metal_record", "rental_record", "treasury_record"),
                    "f0982ab3825849d6cbeec5bb71c7748b524093a52d31661ecdecdb47ed9cac5a"),
}


class ContractError(ValueError):
    pass


def run_json(command, root):
    result = subprocess.run(command, cwd=root, text=True, capture_output=True, check=False)
    if result.returncode:
        raise ContractError(f"Extractor failed ({command[0]}):\n{result.stderr.strip()}")
    # TS7's API server currently prints this shutdown message even on success.
    stderr = "\n".join(line for line in result.stderr.splitlines() if line != "context canceled")
    if stderr:
        print(stderr, file=sys.stderr)
    return json.loads(result.stdout)


def build_extractor():
    target = ROOT / ".local/contract-check-target"
    environment = {**os.environ, "CARGO_TARGET_DIR": str(target)}
    subprocess.run(["cargo", "build", "--manifest-path", str(HELPER / "Cargo.toml"),
                    "--locked", "--offline", "--quiet"], cwd=ROOT, env=environment, check=True)
    return target / "debug/patrimoine-contract-check"


def extract(root, binary=None):
    binary = binary or build_extractor()
    rust = run_json([str(binary), str(root)], root)
    enum_names = sorted(name for name, shape in rust["types"].items() if "variants" in shape)
    typescript = run_json(["node", str(HELPER / "typescript.mjs"), str(root), json.dumps(enum_names)], root)
    return rust, typescript


def renamed(name, rules, explicit=None, enum=False):
    if explicit:
        return explicit
    rule = rules.get("rename_all")
    if not rule:
        return name
    if not name.isascii():
        raise ContractError("Non-ASCII Serde rename_all identifiers require explicit rename")
    # Serde deliberately uses different rules for variants and struct fields.
    # Preserve acronyms and existing case exactly as serde_derive does.
    pascal = name if enum else "".join(word[:1].upper() + word[1:] for word in name.split("_"))
    snake = "".join(("_" if index and char.isupper() else "") + char.lower() for index, char in enumerate(name)) if enum else name
    transforms = {
        "snake_case": lambda: snake, "kebab-case": lambda: snake.replace("_", "-"),
        "SCREAMING_SNAKE_CASE": lambda: snake.upper(),
        "SCREAMING-KEBAB-CASE": lambda: snake.upper().replace("_", "-"),
        "camelCase": lambda: pascal[:1].lower() + pascal[1:],
        "PascalCase": lambda: pascal,
        "lowercase": lambda: name.lower() if enum else name, "UPPERCASE": lambda: name.upper(),
    }
    if rule not in transforms:
        raise ContractError(f"Unsupported Serde rename_all {rule}")
    return transforms[rule]()


def union(values):
    unique = {json.dumps(value, sort_keys=True): value for value in values}
    ordered = [unique[key] for key in sorted(unique)]
    return ordered[0] if len(ordered) == 1 else {"union": ordered}


class RustSchema:
    def __init__(self, types):
        self.types = types
        self.cache = {}
        self.visiting = set()

    def type(self, value):
        name, arguments = value["name"], value["args"]
        if name in ("String", "str"):
            return "string"
        if name == "bool":
            return "boolean"
        if re.fullmatch(r"[ui](8|16|32|64|128|size)|f(32|64)", name):
            return "number"
        if name == "void":
            return "void"
        if name == "Option" and len(arguments) == 1:
            return union(["null", self.type(arguments[0])])
        if name == "Vec" and len(arguments) == 1:
            return {"array": self.type(arguments[0])}
        if name in ("BTreeMap", "HashMap") and len(arguments) == 2:
            if self.type(arguments[0]) != "string":
                raise ContractError("Only string-keyed wire maps are supported")
            return {"map": self.type(arguments[1])}
        if arguments:
            raise ContractError(f"Unsupported Rust generic {name}")
        return self.named(name)

    def named(self, name):
        if name in self.cache:
            return self.cache[name]
        if name in self.visiting or name not in self.types:
            raise ContractError(f"Unsupported or unresolved Rust wire type {name}")
        self.visiting.add(name)
        value = self.types[name]
        if "variants" in value:
            result = {"enum": sorted(renamed(v["name"], value["serde"], v["serde"].get("rename"), enum=True) for v in value["variants"])}
        else:
            fields = {}
            for field in value["fields"]:
                key = renamed(field["name"], value["serde"], field["serde"].get("rename"))
                if key in fields:
                    raise ContractError(f"Duplicate Serde wire field {name}.{key}")
                fields[key] = {"required": "skip_serializing_if" not in field["serde"], "type": self.type(field["type"])}
            result = {"fields": fields}
        self.visiting.remove(name)
        self.cache[name] = result
        return result


def ts_shape(value, types, visiting=()):
    if isinstance(value, str):
        return value
    if "ref" in value:
        name = value["ref"]
        if name in visiting or name not in types:
            raise ContractError(f"Unsupported or unresolved TypeScript wire type {name}")
        return ts_shape(types[name], types, (*visiting, name))
    if "union" in value:
        return union([ts_shape(v, types, visiting) for v in value["union"]])
    if "enum" in value:
        return {"enum": sorted(value["enum"])}
    if "fields" in value:
        return {"fields": {name: {"required": f["required"], "type": ts_shape(f["type"], types, visiting)} for name, f in value["fields"].items()}}
    for kind in ("array", "map"):
        if kind in value:
            return {kind: ts_shape(value[kind], types, visiting)}
    raise ContractError(f"Unsupported TypeScript schema {value}")


def differences(rust, typescript, path):
    if rust == typescript:
        return []
    if isinstance(rust, dict) and isinstance(typescript, dict):
        if "fields" in rust and "fields" in typescript:
            left, right = rust["fields"], typescript["fields"]
            issues = [f"{path}: field keys differ (Rust only {sorted(left.keys() - right.keys())}; TS only {sorted(right.keys() - left.keys())})"] if left.keys() != right.keys() else []
            for name in sorted(left.keys() & right.keys()):
                if left[name]["required"] != right[name]["required"]:
                    issues.append(f"{path}.{name}: required/optional differs")
                issues += differences(left[name]["type"], right[name]["type"], f"{path}.{name}")
            return issues
        for kind in ("array", "map"):
            if kind in rust and kind in typescript:
                return differences(rust[kind], typescript[kind], path + ("[]" if kind == "array" else "[key]"))
    # Do not print complete report objects or duplicate every nested difference.
    return [f"{path}: wire type/nullability differs"]


def camel(name):
    parts = name.split("_")
    return parts[0] + "".join(part.capitalize() for part in parts[1:])


def compare(rust, typescript, verify_macros=True):
    issues = []
    if verify_macros:
        actual = {name: hashlib.sha256(tokens.encode()).hexdigest() for name, tokens in rust["macros"].items()}
        if actual != MACROS:
            issues.append("record macro definitions changed: review their wire expansion and token fingerprints")
    schema = RustSchema(rust["types"])
    shared = sorted(rust["types"].keys() & typescript["types"].keys())
    aliases = 0
    fields = sum(len(rust["types"][name].get("fields", [])) for name in shared)
    for name in sorted(typescript["types"]):
        if name == "RegistryRecord":
            continue  # TS-only base interface; its fields are checked on every row.
        target = ALIASES.get(name, name)
        if target not in rust["types"]:
            issues.append(f"{name}: missing Rust DTO")
            continue
        aliases += name in ALIASES
        issues += differences(schema.named(target), ts_shape(typescript["types"][name], typescript["types"]), name)
    error_codes = schema.named("ErrorCode")["enum"]
    if error_codes != sorted(typescript["error_codes"]):
        issues.append("ErrorCode: Serde enum values and TypeScript literals differ")
    registered = rust["registered"]
    wrappers = typescript["commands"]
    names = [command["name"] for command in wrappers]
    for label, values in (("Tauri registry", registered), ("invoke wrappers", names)):
        if len(values) != len(set(values)):
            issues.append(f"{label}: duplicate command")
    if set(registered) != set(rust["commands"]) or set(names) != set(registered):
        issues.append("commands: invoke wrappers, Rust handlers and Tauri registry differ")
    for command in wrappers:
        name = command["name"]
        if name not in rust["commands"]:
            continue
        handler = rust["commands"][name]
        args = {camel(key): {"required": True, "type": schema.type(value)} for key, value in handler["args"].items()}
        supplied = ts_shape(command["args"], typescript["types"])
        # A wrapper may deliberately supply a default value (description='') to
        # a nullable Rust parameter. Nested DTOs still require exact nullability.
        for key in args.keys() & supplied["fields"].keys():
            value = args[key]["type"]
            if isinstance(value, dict) and "union" in value and "null" in value["union"] and supplied["fields"][key]["type"] in value["union"]:
                args[key]["type"] = supplied["fields"][key]["type"]
        issues += differences({"fields": args}, supplied, f"command {name} arguments")
        issues += differences(schema.type(handler["return"]), ts_shape(command["return"], typescript["types"]), f"command {name} result")
    stats = {"shared_dtos": len(shared), "shared_fields": fields, "aliased_dtos": aliases,
             "invoke_wrappers": len(wrappers), "rust_handlers": len(rust["commands"]),
             "registered_commands": len(registered), "error_codes": len(error_codes)}
    return issues, stats


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT, help="source fixture/repository root")
    args = parser.parse_args()
    try:
        issues, stats = compare(*extract(args.root.resolve()))
    except (ContractError, subprocess.CalledProcessError) as error:
        print(f"Contract check failed: {error}", file=sys.stderr)
        return 1
    if issues:
        print("Contract check failed:\n" + "\n".join(f"- {issue}" for issue in issues), file=sys.stderr)
        return 1
    print("Contracts OK: " + json.dumps(stats, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
