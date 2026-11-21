# Patrimoine 0.1.2 pour Pop!_OS 22.04 et Ubuntu 22.04

Cette version intègre les corrections de calcul, de stockage, d’interface et de documentation issues de la revue. Elle est destinée aux systèmes **x86_64** avec glibc 2.35, tels que Pop!_OS 22.04. [Suivi des corrections](../../docs/projet/suivi-corrections.md)

## Télécharger et lancer

- [AppImage 0.1.2 complète](https://github.com/j-pirate/patrimoine/raw/refs/heads/main/distribution/0.1.2-popos22.04/Patrimoine_0.1.2_amd64.AppImage)
- [Kit de récupération 0.1.2](Patrimoine_0.1.2_recuperation.zip)
- [Empreintes SHA-256](SHA256SUMS)
- [Notices et licences](notices-ubuntu22.04.zip)

Se connecter à GitHub si le dépôt demande une authentification. Le fichier AppImage est complet et se lance depuis son dossier de téléchargement avec les commandes suivantes.

```bash
chmod u+x Patrimoine_0.1.2_amd64.AppImage
APPIMAGE_EXTRACT_AND_RUN=1 ./Patrimoine_0.1.2_amd64.AppImage
```

Le mode par extraction convient aux postes où FUSE n’est pas disponible.

## Mettre à jour un coffre existant

Créer une sauvegarde complète depuis **Sauvegarde et export** dans l’ancienne version, puis fermer cette version et lancer la nouvelle AppImage. Les données sont séparées du fichier téléchargé. Le schéma SQLite reste en version 6 et le coffre courant reste à son emplacement habituel. Si le bilan signale une ancienne association incohérente entre créance et financement, ouvrir la créance concernée pour régulariser son lien. [Guide des financements](../../docs/utilisateur/comptes-et-financements.md)

Le kit de récupération contient un lecteur indépendant pour les archives CSV intégrales et les six migrations vérifiées. Le ZIP SQLite reste utilisable dans le parcours de restauration de Patrimoine. [Guide de sauvegarde et récupération](../../docs/utilisateur/documents-et-portabilite.md)

## Sources et qualification

Le paquet est construit sur Ubuntu 22.04 à partir du commit `e9d3d14dbc3b867c44b25ce59cf265f5f9858b3d`. Les fichiers du dossier indiquent les empreintes, les bibliothèques et les résultats des contrôles. [Sources correspondantes](https://github.com/j-pirate/patrimoine/tree/e9d3d14dbc3b867c44b25ce59cf265f5f9858b3d) · [Rapport de validation](../../docs/projet/validation-corrections-0.1.2.md)

Le fichier complet pèse **85 055 992 octets**. Son SHA-256 est `5e14fd85d7fb5129a5e19da7bbbef495db16fd2fe97f69980971b3caa56a3d47`. La qualification sous Ubuntu 22.04.5 a réussi 50 étapes graphiques avec dialogues GTK et le remplacement 0.1.1 → 0.1.2 suivi d’un retour à 0.1.1. [Manifeste](build-manifest.json) · [Contrôles AppImage et mise à jour](validation-popos22.04.json)
