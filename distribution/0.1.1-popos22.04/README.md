# Patrimoine pour Pop!_OS 22.04

Reconstruction du **14 novembre 2026, Europe/Paris**, pour **Pop!_OS 22.04 / Ubuntu 22.04 x86_64, glibc 2.35**. Les calculs et le schéma SQLite 6 restent ceux de Patrimoine 0.1.1.

## Télécharger

| Format | Téléchargement | Taille |
|---|---|---:|
| AppImage entière, sans ZIP à reconstituer | [Télécharger l’AppImage](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.1-popos22.04/Patrimoine_0.1.1_popos22.04_amd64.AppImage) | 84,9 Mo |

Le nom de ce fichier contient **`popos22.04`**. Sur un dépôt privé, se connecter à GitHub avec un compte autorisé avant de télécharger.

## Utiliser l’AppImage

Dans un terminal ouvert dans le dossier du téléchargement :

```bash
chmod u+x Patrimoine_0.1.1_popos22.04_amd64.AppImage
APPIMAGE_EXTRACT_AND_RUN=1 ./Patrimoine_0.1.1_popos22.04_amd64.AppImage
```

Ce mode est testé sans FUSE. Il extrait temporairement le paquet pour l’exécuter. Node et Rust ne sont pas nécessaires pour utiliser l’AppImage.

Le coffre est conservé hors de l’installation, sous `${XDG_DATA_HOME:-$HOME/.local/share}/patrimoine`. L’AppImage utilise un verrou de coffre ; fermer une instance avant d’en lancer une autre. Le remplacement du programme ne supprime pas les données. Conserver ses propres sauvegardes.

## Pourquoi le paquet précédent échouait

Le paquet Debian 13 embarquait des bibliothèques exigeant notamment `GLIBC_2.39`, `GLIBCXX_3.4.32` et `CXXABI_1.3.15`. Pop!_OS 22.04 dispose de glibc 2.35. L’échec se produisait avant l’ouverture de l’application, même après une reconstitution correcte des six ZIP.

Cette reconstruction utilise les bibliothèques d’Ubuntu 22.04. Il n’est pas nécessaire de remplacer manuellement la glibc du poste.

## Vérifications effectuées

- Les 175 fichiers ELF du nouvel AppDir passent le contrôle des versions et symboles requis sur Ubuntu 22.04 : **aucune exigence forte ou faible manquante**. Maximum GLIBC requis : **2.35**.
- L’AppImage entière passe les **50 étapes natives** du parcours graphique, avec dialogues, saisies, calculs, documents, sauvegardes et restaurations.
- Lors de la qualification du 14 novembre, un `.deb` avait également passé 50 étapes dans un conteneur distinct. Ce format a été retiré de la distribution le 15 novembre, à la demande de l’utilisateur ; ses preuves restent des archives historiques.
- Les parcours de qualification s’exécutent hors réseau, sous utilisateur non privilégié, sur des coffres synthétiques neufs.
- Les **733 fichiers** de notices embarqués sont vérifiés par empreinte, pour **468 composants**, dont 113 paquets Ubuntu. Aucun texte manquant ni bibliothèque partagée non identifiée dans ce périmètre.
- Le code applicatif, les dépendances verrouillées et les migrations sont inchangés ; seules les adaptations de construction et les notices de la copie isolée diffèrent.

Les essais sont réalisés sur la base **Ubuntu 22.04 avec Xvfb**. Ils vérifient la compatibilité système correspondant à Pop!_OS 22.04, sans reproduire la session graphique ni le pilote GPU du poste de l’utilisateur. Les limites de grand volume et les deux avertissements amont du [rapport antérieur](../../docs/22-validation-phase10.md) restent applicables.

## Empreintes et preuves

[SHA256SUMS](SHA256SUMS) conserve les empreintes des fichiers de ce dossier. Depuis le dossier contenant les fichiers téléchargés :

```bash
sha256sum Patrimoine_0.1.1_popos22.04_amd64.AppImage
```

| Fichier | SHA-256 |
|---|---|
| AppImage | `53ec6527842cbfeac228489af081f1602e5f2c6deb98f7d11980924beaa45e2b` |

- [Rapport de validation de cette reconstruction](validation-popos22.04.json)
- [Manifeste de construction et versions système](build-manifest.json)
- [Parcours AppImage](preuves/native-appimage.json) ; [archive du parcours .deb retiré](preuves/native-deb.json)
- [Contrôle ABI du nouveau paquet](preuves/new-ubuntu22-abi.json) et [incompatibilités de l’ancien](preuves/old-debian-abi.json)
- [Identité des sources](preuves/source-identity.json) et [contrôle des notices extraites](preuves/extraction-check.json)
- [Notices Ubuntu et licences tierces](notices-ubuntu22.04.zip)

Le [guide de récupération](../../docs/23-installation-et-recuperation-linux.md) et le [kit autonome](../0.1.1/Patrimoine_0.1.1_recuperation.zip) restent utilisables pour le schéma 6. Les notices ne changent pas la licence du code propre à l’application ; leurs textes et les limites de redistribution restent à consulter.

## Construction et archives

Le sous-dossier `construction/` conserve le Dockerfile de la base Ubuntu, les commandes internes, les adaptations de packaging et le contrôleur ABI. Sa recette ne produit désormais que l’AppImage. Les [commandes exécutées le 14 novembre](preuves/inside-build-2026-11-14.sh) et le [manifeste original](preuves/build-manifest-2026-11-14.json), qui incluaient aussi le `.deb`, restent archivés pour expliquer les preuves antérieures. Le manifeste courant décrit uniquement les fichiers encore distribués. La construction utilise les sources du commit `b0027e3052d478b913edb5c22303922814961a00`, Rust 1.99.1, Node 24.22.1 et les caches de dépendances déjà préparés. Elle se déroule hors réseau après préparation de l’image système. Les adaptations ne concernent pas les calculs : elles identifient la cible Ubuntu et collectent ses notices depuis sa propre base de paquets, sans réutiliser le sysroot Debian 13.

Ces fichiers décrivent l’environnement de cette construction ; les chemins de montage des caches doivent être adaptés sur une autre machine. L’identité binaire de deux reconstructions n’est pas revendiquée.
