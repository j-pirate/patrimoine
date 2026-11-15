## Licence

Copyright (C) 2026 [j-pirate]

Ce projet est distribué sous licence
GNU General Public License version 3.0.

Voir le fichier LICENSE pour les conditions complètes.

# Patrimoine

Application **Linux locale de gestion patrimoniale familiale**, en français. Elle permet de suivre plusieurs membres d'une famille, leurs actifs, leurs biens immobiliers, leurs comptes et leurs participations dans des sociétés.

**Version disponible : 0.1.1.** Le dépôt contient le code source, les guides et les installateurs Linux.

## Télécharger et installer

| Système | Paquet et instructions |
| --- | --- |
| **Pop!_OS 22.04 / Ubuntu 22.04 (x86_64)** | [AppImage complète](distribution/0.1.1-popos22.04/README.md) |
| **Debian 13 (x86_64)** | [AppImage distribuée en six archives](distribution/0.1.1/README.md) |

[Voir tous les téléchargements](distribution/README.md). Les deux constructions ont des exigences de bibliothèques différentes ; leur compatibilité n'est pas universelle sur Linux.

## Ce que l'application permet de suivre

- **Famille et propriété** : bilan familial ou individuel, détentions en nom propre et via sociétés.
- **Immobilier et loyers** : biens, unités, baux, encaissements et historique.
- **Trésorerie et devises** : comptes bancaires, caisses physiques et taux de change saisis manuellement.
- **Autres actifs et engagements** : or, argent, sociétés, dettes et financements.
- **Documents et portabilité** : originaux, sauvegardes SQLite, export CSV intégral et XLSX de consultation.
- **Tableaux de bord** : valorisations connues, sources et alertes ; pas de patrimoine net prétendument complet sans couverture des données.

## Données et confidentialité

Patrimoine fonctionne localement sans compte externe et sans mise à jour automatique. Par défaut, les données sont enregistrées sous `${XDG_DATA_HOME:-$HOME/.local/share}/patrimoine`, séparément des fichiers de l'application. **Le coffre et les sauvegardes ne sont pas chiffrés par Patrimoine** : la protection du disque et des copies reste nécessaire.

## Documentation

- [Installation et guides d'utilisation](docs/utilisateur/README.md)
- [Architecture, développement et sécurité](docs/technique/README.md)
- [Cahier des charges et suivi du projet](docs/projet/README.md)
- [Archives des validations](docs/historique/README.md)

[Index complet de la documentation](docs/README.md)

## État de la version

La version 0.1.1 a été testée sur des jeux de données synthétiques. La variante Pop!_OS 22.04 a été contrôlée dans un environnement Ubuntu 22.04 avec Xvfb, pas sur toutes les configurations graphiques réelles. Les performances sur de très gros coffres sont limitées et des avertissements de dépendances restent documentés.

Lire les [preuves de validation de la phase 10](docs/historique/validation-phase-10.md) et la [revue de sécurité](docs/technique/revue-securite.md) pour le périmètre exact des tests et leurs limites.
