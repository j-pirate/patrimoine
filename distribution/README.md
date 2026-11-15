# Télécharger Patrimoine 0.1.1

Choisir le paquet selon **la distribution Linux**. Les variantes ont le même code métier mais ont été construites avec des bibliothèques système différentes.

| Système | Formats disponibles | Instructions |
| --- | --- | --- |
| **Pop!_OS 22.04 / Ubuntu 22.04, x86_64** | Paquet `.deb` et AppImage complète | [Télécharger et installer](0.1.1-popos22.04/README.md) |
| **Debian 13, x86_64** | AppImage répartie en six ZIP | [Télécharger et reconstituer](0.1.1/README.md) |

Le paquet Debian 13 ne fonctionne pas directement sur Pop!_OS 22.04 : les versions de bibliothèques sont incompatibles. **Ne pas remplacer manuellement la glibc du système.**

Pour les sauvegardes et la récupération, voir le [guide d'installation et de récupération](../docs/utilisateur/installation-linux.md). Les [tests de qualification](../docs/historique/validation-phase-10.md) et la [revue de sécurité](../docs/technique/revue-securite.md) expliquent les limites connues.

**Remarque sur les téléchargements :** la publication dans GitHub Releases a échoué avec une erreur d'accès API. Les paquets restent donc dans le dépôt. Voir [l'erreur documentée](erreur-github-releases.txt). Ils ne seront pas supprimés sans solution de remplacement vérifiée.
