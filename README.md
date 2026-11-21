## Licence

Copyright (C) 2026 [j-pirate]

Ce projet est distribué sous licence
GNU General Public License version 3.0.

Voir le fichier LICENSE pour les conditions complètes.

# Patrimoine

Application **Linux locale de gestion patrimoniale familiale**, en français. Elle permet de suivre plusieurs membres d'une famille, leurs actifs, leurs biens immobiliers, leurs comptes et leurs participations dans des sociétés.

**Patrimoine 0.2.0 pour Pop!_OS 22.04 et Ubuntu 22.04.** La navigation donne accès directement aux premières saisies, notamment l’ajout d’un membre ou d’un bien immobilier. Un assistant facultatif et huit tutoriels illustrés sont disponibles hors ligne.

## Télécharger et installer

| Système | Paquet et instructions |
| --- | --- |
| **Pop!_OS 22.04 / Ubuntu 22.04 (x86_64)** | [AppImage 0.2.0 complète et kit de récupération](distribution/0.2.0-popos22.04/README.md) |
| **Debian 13 (x86_64)** | [Archive 0.1.1 pour Debian 13](distribution/0.1.1/README.md) |

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
- [Premiers pas et aide illustrée](docs/utilisateur/premiers-pas.md)
- [Architecture, développement et sécurité](docs/technique/README.md)
- [Cahier des charges et suivi du projet](docs/projet/README.md)
- [Archives des validations](docs/historique/README.md)

[Index complet de la documentation](docs/README.md)

## État de la version

La qualification utilise des coffres fictifs. Les essais graphiques de l’AppImage s’exécutent sous Ubuntu 22.04 avec Xvfb. Le rapport décrit les contrôles, le périmètre et les limites de la qualification.

[Refonte et validation 0.2.0](docs/projet/ergonomie-0.2.0.md) · [Revue de sécurité](docs/technique/revue-securite.md)
