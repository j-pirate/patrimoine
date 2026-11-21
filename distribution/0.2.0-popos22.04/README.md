# Patrimoine 0.2.0 pour Pop!_OS 22.04 et Ubuntu 22.04

Nouvelle navigation, ajout direct des premières fiches, assistant facultatif et huit tutoriels illustrés intégrés. Cette AppImage est destinée aux ordinateurs Linux **x86_64** avec glibc 2.35.

## Télécharger et lancer

- [AppImage 0.2.0 complète](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.2.0-popos22.04/Patrimoine_0.2.0_amd64.AppImage)
- [Kit de récupération 0.2.0](Patrimoine_0.2.0_recuperation.zip)
- [Empreintes SHA-256](SHA256SUMS)
- [Notices et licences](notices-ubuntu22.04.zip)

Si GitHub demande une authentification, connectez-vous avec un compte qui a accès au dépôt. Depuis le dossier de téléchargement, rendez le fichier exécutable et lancez-le.

```bash
chmod u+x Patrimoine_0.2.0_amd64.AppImage
APPIMAGE_EXTRACT_AND_RUN=1 ./Patrimoine_0.2.0_amd64.AppImage
```

Le mode par extraction fonctionne sans montage FUSE. Pour vérifier l’AppImage, téléchargez aussi `SHA256SUMS` dans le même dossier et lancez `sha256sum --ignore-missing --check SHA256SUMS`.

## Mettre à jour depuis la version 0.1.2

1. Ouvrez **Sauvegarde et export → Créer une sauvegarde complète** dans la version 0.1.2. Conservez cette archive et l’ancienne AppImage.
2. Fermez Patrimoine, puis lancez la nouvelle AppImage sur le même compte Linux. Elle ouvre votre coffre courant avec le schéma SQLite 6.
3. Vérifiez votre tableau de bord et vos documents. La configuration guidée reste disponible dans **Aide** et **Paramètres**.

Le kit reconstruit hors ligne une copie SQLite et les documents à partir d’un export CSV intégral. Pour les fichiers ZIP issus de **Créer une sauvegarde complète**, utilisez **Choisir une sauvegarde à analyser** dans Patrimoine afin de faire vérifier l’archive avant restauration. [Guide de sauvegarde et récupération](../../docs/utilisateur/documents-et-portabilite.md)

## Guides et preuves

L’AppImage complète pèse **85 293 560 octets**. Son SHA-256 est `1cdfc77eb2169efe664ab681fdaf69677e1f0525ea893735a4d309e90b462eaf`. [Code source utilisé pour la construction](https://github.com/j-pirate/patrimoine/tree/3900bd0b0bafecdd23ec9b2e34704d1bbfb3f83a)

- [Premiers pas](../../docs/utilisateur/premiers-pas.md)
- [Démarche et rapport de validation](../../docs/projet/ergonomie-0.2.0.md)
- [Manifeste de construction](build-manifest.json)
- [Résultats de qualification de l’AppImage](validation-popos22.04.json)
- [Versions précédentes](../README.md)
