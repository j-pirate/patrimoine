#!/usr/bin/env python3
"""Créer un kit de secours autonome, sans binaire ni dépendance Python externe."""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    version = json.loads((root / 'src-tauri/tauri.conf.json').read_text())['version']
    args.output.mkdir(parents=True, exist_ok=True)
    archive = args.output / f'Patrimoine_{version}_recuperation.zip'
    if archive.exists():
        parser.error('La destination existe déjà ; conserver le kit précédent ou choisir un autre dossier.')
    files = {'scripts/recover-csv.py': (root / 'scripts/recover-csv.py').read_bytes()}
    migrations = sorted((root / 'crates/infrastructure/migrations').glob('*.sql'))
    if len(migrations) != 6:
        parser.error('Vérifier le format du kit : six migrations du schéma 6 attendues.')
    files.update({p.relative_to(root).as_posix(): p.read_bytes() for p in migrations})
    files['LISEZMOI.txt'] = f'''Kit de récupération Patrimoine {version} — schéma SQLite 6

Décompresser ce kit dans un dossier neuf. Python 3.10+ et SQLite 3.37+
suffisent ; aucun exécutable Patrimoine, outil Rust, Node ou accès réseau
n'est requis. Depuis le dossier du kit :

mkdir recuperation
python3 scripts/recover-csv.py /chemin/patrimoine-csv.zip --output recuperation/patrimoine.sqlite3 --documents-dir recuperation/documents

Les destinations doivent être inexistantes. Le lecteur vérifie ses migrations
locales par leurs empreintes épinglées et n'exécute aucun SQL reçu de l'archive.
La copie externe conserve exactement les données et le K source, sans nouvel
événement technique. Elle contrôle format, empreintes, types, références,
intégrité SQLite et originaux ; elle ne réexécute pas les invariants financiers
du moteur Rust et n'active aucun coffre. Pour remettre les données en service
dans l'application, utiliser l'analyse puis le réimport CSV applicatifs.

La sauvegarde ZIP SQLite reste consultable sans l'application : ouvrir
data/patrimoine.sqlite3 avec un lecteur SQLite standard et retrouver les
originaux dans documents/. Ne pas copier seulement une base active en WAL.

Les SHA-256 et CRC détectent une différence avec une valeur connue ; ce ne
sont pas des signatures de l'auteur. Conserver les archives sur un support
protégé : elles sont en clair. Aucun original n'est inclus dans ce kit.
Les sources de l'application ne reçoivent aucune licence supplémentaire.
'''.encode('utf-8')
    manifest = {'kit_version': 1, 'application_version': version, 'schema_version': 6,
                'files': [{'path': name, 'size_bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()} for name, data in sorted(files.items())]}
    files['CONTENU.json'] = (json.dumps(manifest, ensure_ascii=False, indent=2) + '\n').encode()
    with zipfile.ZipFile(archive, 'x', compression=zipfile.ZIP_DEFLATED) as opened:
        for name, data in sorted(files.items()):
            entry = zipfile.ZipInfo(name, date_time=(2026, 11, 13, 0, 0, 0))
            entry.create_system = 3
            entry.external_attr = 0o100600 << 16
            entry.compress_type = zipfile.ZIP_DEFLATED
            opened.writestr(entry, data)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_suffix('.zip.sha256').write_text(f'{digest}  {archive.name}\n', encoding='ascii')
    print(json.dumps({'archive': str(archive), 'sha256': digest, 'files': len(files), 'version': version}))


if __name__ == '__main__':
    main()
