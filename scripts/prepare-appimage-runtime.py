#!/usr/bin/env python3
"""Prepare explicit WebKit subprocess copies for a Linux AppImage build.

The cloud compiler uses a private sysroot, while the GTK bundler must inspect
the runtime's /usr paths. Include WebKit helpers explicitly rather than shipping
build-machine paths in the AppDir. This script neither downloads nor installs.
"""
import hashlib
import json
import os
from pathlib import Path


root = Path(__file__).resolve().parents[1]
tool_root = Path(os.environ.get("PATRIMOINE_TOOL_ROOT", "/workspace/.tooling"))
relative = Path("usr/lib/x86_64-linux-gnu/webkit2gtk-4.1")
sources = [("system", Path("/") / relative), ("cloud_sysroot", tool_root / "sysroot" / relative)]
origin, source = next(((name, directory) for name, directory in sources if (directory / "WebKitWebProcess").is_file()), (None, None))
if source is None:
    raise SystemExit("WebKitGTK 4.1 introuvable : installer ses paquets de développement/runtime avant le packaging.")

files = {}
recorded = []
for name in ("WebKitWebProcess", "WebKitNetworkProcess", "WebKitGPUProcess", "injected-bundle/libwebkit2gtkinjectedbundle.so"):
    path = source / name
    if not path.is_file():
        if name == "WebKitGPUProcess":
            continue  # Older WebKitGTK builds have no separate GPU subprocess.
        raise SystemExit(f"Composant WebKitGTK manquant : {path}")
    destination = str(Path("/") / relative / name)
    files[destination] = str(path)
    recorded.append({"destination": destination, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "origin": origin})

folder = root / ".local/build"
folder.mkdir(parents=True, exist_ok=True)
config = folder / "appimage-runtime.json"
config.write_text(json.dumps({"bundle": {"linux": {"appimage": {"files": files}}}}, indent=2) + "\n")
(folder / "appimage-runtime-manifest.json").write_text(json.dumps(recorded, indent=2) + "\n")
print(config)
