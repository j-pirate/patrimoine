# Télécharger et utiliser Patrimoine 0.1.1

**Vous utilisez Pop!_OS 22.04 ou Ubuntu 22.04 ? [Téléchargez le paquet compatible ici](../0.1.1-popos22.04/README.md)** : `.deb` de 7,5 Mo ou AppImage entière de 84,9 Mo, sans six ZIP à reconstituer. La procédure ci-dessous concerne le paquet Debian 13 d’origine.

Application locale pour **Linux Debian 13 x86_64**, avec session graphique et bibliothèques GTK 3/WebKitGTK 4.1. Node et Rust ne sont pas nécessaires pour utiliser le programme.

## Télécharger l’application

Télécharger les **six ZIP** ci-dessous. Chaque fichier fait moins de 20 Mo. Sur un dépôt privé, se connecter à GitHub avec un compte autorisé.

- [Télécharger la partie 1/6](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.1/Patrimoine_0.1.1_application_partie_1_sur_6.zip)

- [Télécharger la partie 2/6](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.1/Patrimoine_0.1.1_application_partie_2_sur_6.zip)

- [Télécharger la partie 3/6](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.1/Patrimoine_0.1.1_application_partie_3_sur_6.zip)

- [Télécharger la partie 4/6](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.1/Patrimoine_0.1.1_application_partie_4_sur_6.zip)

- [Télécharger la partie 5/6](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.1/Patrimoine_0.1.1_application_partie_5_sur_6.zip)

- [Télécharger la partie 6/6](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.1/Patrimoine_0.1.1_application_partie_6_sur_6.zip)

## Installer et démarrer

1. Extraire les six ZIP dans le **même dossier parent**, en acceptant la fusion du dossier Patrimoine_0.1.1_appimage.
2. Ouvrir un terminal dans ce dossier et reconstituer le fichier :

```bash
cd Patrimoine_0.1.1_appimage
bash reconstituer-appimage.sh
```

3. Le script vérifie taille et SHA-256, puis crée le fichier exécutable sans remplacer un fichier différent. Il ne lance pas l’application. Pour démarrer avec le mode testé sans FUSE :

```bash
APPIMAGE_EXTRACT_AND_RUN=1 ./Patrimoine_0.1.1_amd64.AppImage
```

Prévoir environ 350 Mo pour les téléchargements, morceaux et fichier reconstitué, puis de l’espace supplémentaire pour l’extraction d’exécution et les données. Le coffre est vide au premier lancement ; les données sont stockées hors installation, sous `${XDG_DATA_HOME:-$HOME/.local/share}/patrimoine`.

Fichier obtenu : **112 196 088 octets**. SHA-256 :

```text
6abd1f90a70baa001560787f46e5b10ced7e3fd364b428c99541c454b5d9c240
```

## Documents et récupération

- [Guide d’installation, mise à jour et récupération (PDF)](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.1/Guide-installation.pdf)
- [Rapport de validation et limites (PDF)](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.1/Rapport-validation.pdf)
- [Kit de récupération autonome (ZIP)](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.1/Patrimoine_0.1.1_recuperation.zip)
- [Sources, tous les guides, notices et preuves (ZIP, 6,2 Mo)](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.1/Patrimoine_0.1.1_sources-et-documents.zip)
- [Empreintes des téléchargements](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.1/SHA256SUMS)
- [Manifeste de construction](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.1/build-manifest.json)
- [Preuve de reconstruction des six ZIP](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.1/verification-parties.json)

Le kit demande Python 3.10+ et SQLite 3.37+, sans paquet Python externe ; son LISEZMOI décrit le secours indépendant. Il ne contient aucune sauvegarde utilisateur. Conserver ses propres sauvegardes sur un support protégé.

## Portée et limites

Cette AppImage ne constitue pas un programme Windows ou macOS. Les autres distributions et anciennes glibc ne sont pas qualifiées. Les très grands coffres restent lents : cible de deux secondes non atteinte, tableau de bord interrompu après 120 secondes sur le gros profil. Deux avertissements de dépendances restent documentés dans la [revue de sécurité](../../docs/technique/revue-securite.md). Les [notices tierces](../../docs/technique/licences-tierces.md) précisent les limites de provenance et de redistribution ; aucune licence supplémentaire n’est accordée au code propre.

## Pourquoi les fichiers sont dans le dépôt

La tentative de publication via GitHub Releases a échoué : `Get "https://api.github.com/repos/j-pirate/patrimoine/releases": Forbidden`. Les téléchargements ci-dessus sont hébergés dans le dépôt via Git, dont l’accès fonctionne. Le message seul ne prouve pas l’origine précise du refus API. La [référence de l’erreur](../erreur-github-releases.txt) conserve l’opération et son résultat. La visibilité du dépôt n’est pas modifiée.
