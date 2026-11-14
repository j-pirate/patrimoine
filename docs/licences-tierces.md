# Licences et notices tierces

Revue locale du **13 novembre 2026**, pour Patrimoine **0.1.1**, Linux x86_64 Debian 13. Les notices sont embarquées via la configuration de ressources Tauri : [NOTICE.txt](../resources/notices/NOTICE.txt), [manifest.json](../resources/notices/manifest.json) et les textes sous `resources/notices/LICENSES/`. Elles accordent les droits des composants concernés ; elles ne changent pas le statut `UNLICENSED` du code propre à l’application.

## Périmètre collecté

| Ensemble | Composants | Méthode |
|---|---:|---|
| Cargo | 301 | Graphe Linux de production, compilation et macros procédurales, issu des métadonnées verrouillées ; les dépendances uniquement de test sont exclues. |
| npm | 46 | Fermeture des dépendances de production de `package-lock.json`, y compris les paquets de types présents dans cette fermeture. |
| Debian | 127 | Paquets identifiés pour les bibliothèques partagées réellement copiées dans l’AppDir ; correspondance par GNU build-id ou octets identiques, jamais par le seul nom du fichier. |
| Runtime et lanceur AppImage | 8 | Type-2 runtime, musl, libfuse, squashfuse, zstd, zlib, mimalloc et AppRun ; ils complètent l’inventaire des `.so`, notamment pour le code lié statiquement. |

Le manifeste contient **482 composants**, sans texte de licence manquant dans ce périmètre et sans bibliothèque partagée non identifiée dans l’AppDir examiné. Un composant de compilation ou de types répertorié n’est pas nécessairement présent dans le binaire final. Le manifeste enregistre les empreintes des deux lockfiles et des textes copiés. L’AppDir de référence utilisé pour identifier les bibliothèques est celui du prototype local 0.1.0. Le [contrôle d’extraction final](../.local/releases/0.1.1/extraction-check.json) vérifie les 747 fichiers des notices dans l’AppImage 0.1.1 identifiée par son empreinte, avec ces 482 composants.

Les licences principales comprennent notamment MIT et Apache-2.0, BSD, Zlib, MPL-2.0 et les familles LGPL/GPL décrites dans les copyrights Debian. Les expressions exactes et éventuels choix de licences restent ceux des textes de chaque composant ; cette liste de familles ne les remplace pas.

## Sources et limites de provenance

Chaque crate dispose de son URL d’archive versionnée. Les quelques textes absents des archives publiées sont conservés sous [resources/licence-sources](../resources/licence-sources), avec URL amont et SHA-256. Le texte générique MPL-2.0 manquant de `selectors` est copié sans modification depuis le fichier de licence de `cssparser` ; cette origine est indiquée. Aucun correctif local n’est appliqué au code de ces crates.

Pour les paquets Debian, le manifeste conserve le nom et la version **du paquet source**, distincts de la version binaire après une reconstruction `+bN`, ainsi qu’un lien Debian Sources. Les copyrights complets des paquets et les textes communs Debian sont copiés. Ces liens et textes facilitent la recherche des sources ; ils ne constituent pas une archive de sources correspondantes préparée par le distributeur.

Le runtime officiel est épinglé par [fetch-appimage-runtime.sh](../scripts/fetch-appimage-runtime.sh) au SHA-256 `617aa77314d57c8b8e4fae9c3f0b35af7b30d5529cb69e98f64f92510ed98a46`. Son identifiant embarqué renvoie au commit `fedfad51acc2860b2916a1eef32ba4a500ff31ef` de `AppImage/type2-runtime`. Son Makefile, son environnement Alpine 3.21, ses téléchargements de dépendances et son patch libfuse sont conservés parmi les preuves. Le runtime annonce cinq bibliothèques dans sa licence ; `mimalloc`, également présent dans le Makefile et les chaînes du binaire, est ajouté explicitement.

Les sources de construction épinglent libfuse **3.15.0**, avec le patch amont du runtime, et squashfuse **0.5.2**, avec les SHA-256 de leurs archives. Les chaînes du binaire identifient zstd **1.5.6** et zlib **1.3.2**, sans établir la révision du paquet Alpine. Les versions exactes liées de **musl et mimalloc ne sont pas récupérées**. Leurs textes de licence sont référencés à musl `v1.2.5` et mimalloc `v2.1.7` ; ces références de textes ne prouvent pas les versions du code lié. Cette limite reste visible dans le manifeste.

`AppRun.wrapped` est identique octet pour octet au téléchargement Tauri `apprun-old` et au lanceur AppImageKit examiné, SHA-256 `f30140a43a0a59e46db21bdefdf749b9e9f2c6946e92afabbacf98b8ae73fb4f`. Son avis MIT conserve les copyrights Simon Peter et RazZziel, à partir d’un fichier source AppImageKit épinglé. Le commit historique de construction du binaire n’est pas connu ; le commit du texte de licence est une preuve distincte.

## Obligations de redistribution

Les notices constituent un dossier de **qualification locale**, sans certification juridique de conformité. Elles ne contiennent ni l’ensemble des sources correspondantes ni une offre écrite de fourniture de sources. Avant une redistribution externe, le distributeur doit préparer les sources ou l’offre permise par chaque licence applicable, avec les versions correspondantes, les patchs Debian et les modifications amont ou locales concernées.

GTK, GLib, WebKitGTK et les autres bibliothèques LGPL du programme restent liées dynamiquement. L’extraction de l’AppImage permet de remplacer leurs fichiers ; les textes et obligations propres à ces licences restent applicables. **Libfuse est liée statiquement au runtime AppImage** : son obligation de permettre la modification et la réédition de liens doit aussi être satisfaite selon la LGPL applicable. La seule extraction de l’AppImage ne suffit pas à cette obligation. Ce dossier n’annonce pas la disponibilité d’un jeu complet d’objets relogeables ou de sources de reconstruction déjà qualifié.

## Régénération hors réseau

Après installation des dépendances correspondant aux lockfiles et préparation de l’AppDir :

```bash
python3 scripts/collect-third-party-licences.py --appdir /chemin/squashfs-root
```

Le [collecteur](../scripts/collect-third-party-licences.py) travaille sans réseau, vérifie les hashes des textes supplémentaires, contrôle l’identité du lanceur et refuse une modification de l’épingle du runtime sans nouvelle revue. Une nouvelle base Debian, un autre lanceur ou un nouveau runtime imposent d’actualiser les preuves. La vérification des avis de sécurité et ses deux avertissements Rust conservés sont décrits dans [24-revue-securite.md](24-revue-securite.md).
