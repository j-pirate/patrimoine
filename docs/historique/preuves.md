# Index des preuves historiques

Inventaire vérifié le 20 novembre 2026. Les rapports décrivent les campagnes à leur date ; cet inventaire n’est pas une nouvelle exécution des tests.

Les pièces marquées **livrée dans le ZIP** sont des membres effectivement présents de l’[archive sources et documents 0.1.1](../../distribution/0.1.1/Patrimoine_0.1.1_sources-et-documents.zip). Télécharger le ZIP depuis GitHub puis ouvrir le chemin indiqué dans `Patrimoine_0.1.1/Preuves/`. Les ancres ci-dessous sont navigables sur GitHub ; les fichiers internes ne sont pas des fichiers isolés du dépôt. L’archive reste inchangée, avec ses anciennes copies documentaires. Ses empreintes sont dans [SHA256SUMS](../../distribution/0.1.1/SHA256SUMS).

La [matrice des campagnes](campagnes-validation.md) distingue chaque programme testé, son empreinte et les sources connues. La [matrice de réception](../projet/matrice-reception.md) relie exigences, résultats et limites. Pour rejouer les contrôles sur les sources actuelles, consulter [Développement Linux](../technique/developpement-linux.md) ; un résultat actuel ne remplace pas une pièce historique.

## Disponibilité par campagne

| Campagne | Disponibilité | Pièce ou limite |
|---|---|---|
| Phase 2, desktop du 4 novembre | Conservée seulement dans l’environnement | `.local/desktop-smoke.WToKYmZZ/desktop-result.json`, présent lors de l’inventaire ; absent du ZIP et de Git. Le [compte rendu](validation-phase-2.md#parcours-desktop-réel-sans-réseau) est versionné. |
| Phase 3, desktop du 4 novembre | Conservée seulement dans l’environnement | `.local/desktop-smoke.Gbwwmc9C/desktop-result.json`, présent lors de l’inventaire ; absent du ZIP et de Git. Le [compte rendu](validation-phase-3.md#parcours-desktop-réel-réseau-coupé) est versionné. |
| Phases 4 à 9, desktop | Livrées dans le ZIP | Dossiers `desktop-smoke.*` ci-dessous ; rapports, captures et inspections citées vérifiés. |
| Phase 10, tests, AppImage, mise à jour, accessibilité, performances et sécurité | Livrées dans le ZIP | Pièces ci-dessous ; limites et échecs conservés dans le [rapport](validation-phase-10.md). |
| Reconstruction Pop!_OS/Ubuntu du 14 novembre | Livrée dans le dépôt | [Validation](../../distribution/0.1.1-popos22.04/validation-popos22.04.json), [identité source](../../distribution/0.1.1-popos22.04/preuves/source-identity.json) et [preuves natives](../../distribution/0.1.1-popos22.04/preuves/native-appimage.json). Le `.deb` historique a été retiré ; ses preuves restent archivées. |
| Commit/arbre source des campagnes anciennes, quand non enregistré | Non retrouvé dans les pièces de ces campagnes | Aucune identité de source n’est déduite d’une date, d’un résultat réussi ou d’une empreinte binaire. Voir la matrice des campagnes. |

## Pièces livrées dans le ZIP

Chaque ligne donne un chemin **relatif à `Patrimoine_0.1.1/Preuves/`**, dont la présence a été contrôlée dans l’archive versionnée. Les noms de répertoires restent ceux des campagnes d’origine.

| Repère | Chemin dans le ZIP | Disponibilité |
|---|---|---|
| <a id="preuve-001"></a>preuve-001 | `accessibility-final-1280-phase10/accessibility-result.json` | Livrée dans le ZIP |
| <a id="preuve-002"></a>preuve-002 | `accessibility-final-900-phase10/accessibility-result.json` | Livrée dans le ZIP |
| <a id="preuve-003"></a>preuve-003 | `accessibility-final-900-phase10/minimum-window.png` | Livrée dans le ZIP |
| <a id="preuve-004"></a>preuve-004 | `appimage-upgrade.Cu5eKTbX/appimage-upgrade-result.json` | Livrée dans le ZIP |
| <a id="preuve-005"></a>preuve-005 | `desktop-smoke.CNTvHv2v/csv-roundtrip-reports.json` | Livrée dans le ZIP |
| <a id="preuve-006"></a>preuve-006 | `desktop-smoke.CNTvHv2v/desktop-result.json` | Livrée dans le ZIP |
| <a id="preuve-007"></a>preuve-007 | `desktop-smoke.CNTvHv2v/documents-current.png` | Livrée dans le ZIP |
| <a id="preuve-008"></a>preuve-008 | `desktop-smoke.CNTvHv2v/documents-historical.png` | Livrée dans le ZIP |
| <a id="preuve-009"></a>preuve-009 | `desktop-smoke.CNTvHv2v/independent-csv-recovery.json` | Livrée dans le ZIP |
| <a id="preuve-010"></a>preuve-010 | `desktop-smoke.CNTvHv2v/independent-phase8-inspection.json` | Livrée dans le ZIP |
| <a id="preuve-011"></a>preuve-011 | `desktop-smoke.CNTvHv2v/independent-phase8-inspection.py` | Livrée dans le ZIP |
| <a id="preuve-012"></a>preuve-012 | `desktop-smoke.CNTvHv2v/independent-xlsx-inspection.json` | Livrée dans le ZIP |
| <a id="preuve-013"></a>preuve-013 | `desktop-smoke.CNTvHv2v/independent-xlsx-inspection.py` | Livrée dans le ZIP |
| <a id="preuve-014"></a>preuve-014 | `desktop-smoke.LP7NLrph/desktop-result.json` | Livrée dans le ZIP |
| <a id="preuve-015"></a>preuve-015 | `desktop-smoke.LP7NLrph/exchange-preview.png` | Livrée dans le ZIP |
| <a id="preuve-016"></a>preuve-016 | `desktop-smoke.LP7NLrph/historical-fx.png` | Livrée dans le ZIP |
| <a id="preuve-017"></a>preuve-017 | `desktop-smoke.LP7NLrph/independent-backup-inspection.json` | Livrée dans le ZIP |
| <a id="preuve-018"></a>preuve-018 | `desktop-smoke.LP7NLrph/variation.png` | Livrée dans le ZIP |
| <a id="preuve-019"></a>preuve-019 | `desktop-smoke.PofeQrAi/dashboard-company.png` | Livrée dans le ZIP |
| <a id="preuve-020"></a>preuve-020 | `desktop-smoke.PofeQrAi/dashboard-family.png` | Livrée dans le ZIP |
| <a id="preuve-021"></a>preuve-021 | `desktop-smoke.PofeQrAi/dashboard-filtered-report.json` | Livrée dans le ZIP |
| <a id="preuve-022"></a>preuve-022 | `desktop-smoke.PofeQrAi/dashboard-filtered.png` | Livrée dans le ZIP |
| <a id="preuve-023"></a>preuve-023 | `desktop-smoke.PofeQrAi/dashboard-oracles.json` | Livrée dans le ZIP |
| <a id="preuve-024"></a>preuve-024 | `desktop-smoke.PofeQrAi/desktop-result.json` | Livrée dans le ZIP |
| <a id="preuve-025"></a>preuve-025 | `desktop-smoke.PofeQrAi/independent-csv-recovery.json` | Livrée dans le ZIP |
| <a id="preuve-026"></a>preuve-026 | `desktop-smoke.PofeQrAi/independent-dashboard-inspection.json` | Livrée dans le ZIP |
| <a id="preuve-027"></a>preuve-027 | `desktop-smoke.PofeQrAi/independent-dashboard-inspection.py` | Livrée dans le ZIP |
| <a id="preuve-028"></a>preuve-028 | `desktop-smoke.PofeQrAi/independent-phase8-inspection.json` | Livrée dans le ZIP |
| <a id="preuve-029"></a>preuve-029 | `desktop-smoke.PofeQrAi/independent-xlsx-inspection.json` | Livrée dans le ZIP |
| <a id="preuve-030"></a>preuve-030 | `desktop-smoke.iRzTjwTL/application-under-test.json` | Livrée dans le ZIP |
| <a id="preuve-031"></a>preuve-031 | `desktop-smoke.iRzTjwTL/desktop-result.json` | Livrée dans le ZIP |
| <a id="preuve-032"></a>preuve-032 | `desktop-smoke.iRzTjwTL/independent-dashboard-inspection.json` | Livrée dans le ZIP |
| <a id="preuve-033"></a>preuve-033 | `desktop-smoke.iRzTjwTL/independent-phase8-inspection.json` | Livrée dans le ZIP |
| <a id="preuve-034"></a>preuve-034 | `desktop-smoke.iRzTjwTL/independent-xlsx-inspection.json` | Livrée dans le ZIP |
| <a id="preuve-035"></a>preuve-035 | `desktop-smoke.ku6m530N/company-report.png` | Livrée dans le ZIP |
| <a id="preuve-036"></a>preuve-036 | `desktop-smoke.ku6m530N/desktop-result.json` | Livrée dans le ZIP |
| <a id="preuve-037"></a>preuve-037 | `desktop-smoke.ku6m530N/financial-restore-comparison.json` | Livrée dans le ZIP |
| <a id="preuve-038"></a>preuve-038 | `desktop-smoke.ku6m530N/independent-backup-inspection.json` | Livrée dans le ZIP |
| <a id="preuve-039"></a>preuve-039 | `desktop-smoke.ku6m530N/independent-backup-inspection.py` | Livrée dans le ZIP |
| <a id="preuve-040"></a>preuve-040 | `desktop-smoke.ku6m530N/metal-company-oracles.json` | Livrée dans le ZIP |
| <a id="preuve-041"></a>preuve-041 | `desktop-smoke.ku6m530N/metal-company-restore-comparison.json` | Livrée dans le ZIP |
| <a id="preuve-042"></a>preuve-042 | `desktop-smoke.ku6m530N/metal-report.png` | Livrée dans le ZIP |
| <a id="preuve-043"></a>preuve-043 | `desktop-smoke.tXhT17zX/desktop-result.json` | Livrée dans le ZIP |
| <a id="preuve-044"></a>preuve-044 | `desktop-smoke.tXhT17zX/independent-backup-inspection.json` | Livrée dans le ZIP |
| <a id="preuve-045"></a>preuve-045 | `desktop-smoke.tXhT17zX/rental-indicators.png` | Livrée dans le ZIP |
| <a id="preuve-046"></a>preuve-046 | `desktop-smoke.yLQnPQrW/desktop-result.json` | Livrée dans le ZIP |
| <a id="preuve-047"></a>preuve-047 | `desktop-smoke.yLQnPQrW/financial-oracles.json` | Livrée dans le ZIP |
| <a id="preuve-048"></a>preuve-048 | `desktop-smoke.yLQnPQrW/financial-restore-comparison.json` | Livrée dans le ZIP |
| <a id="preuve-049"></a>preuve-049 | `desktop-smoke.yLQnPQrW/independent-backup-inspection.json` | Livrée dans le ZIP |
| <a id="preuve-050"></a>preuve-050 | `performance-full/archive-capacity.json` | Livrée dans le ZIP |
| <a id="preuve-051"></a>preuve-051 | `performance-full/performance-summary.json` | Livrée dans le ZIP |
| <a id="preuve-052"></a>preuve-052 | `performance-full/performance.json` | Livrée dans le ZIP |
| <a id="preuve-053"></a>preuve-053 | `performance-full/roundtrip-exact.json` | Livrée dans le ZIP |
| <a id="preuve-054"></a>preuve-054 | `performance-full/source-header-diagnosis.json` | Livrée dans le ZIP |
| <a id="preuve-055"></a>preuve-055 | `performance-pilot/performance.json` | Livrée dans le ZIP |
| <a id="preuve-056"></a>preuve-056 | `performance-pilot/roundtrip-exact.json` | Livrée dans le ZIP |
| <a id="preuve-057"></a>preuve-057 | `phase10-security-review/glib-reachability-final.json` | Livrée dans le ZIP |
| <a id="preuve-058"></a>preuve-058 | `recovery-kit-full/recovery-kit-result.json` | Livrée dans le ZIP |
| <a id="preuve-059"></a>preuve-059 | `releases/0.1.1-before-import-order/candidate-note.json` | Livrée dans le ZIP |
| <a id="preuve-060"></a>preuve-060 | `releases/0.1.1/extraction-check.json` | Livrée dans le ZIP |
| <a id="preuve-061"></a>preuve-061 | `outils/check-phase10-final.log` | Livrée dans le ZIP |
| <a id="preuve-062"></a>preuve-062 | `outils/check-phase6.log` | Livrée dans le ZIP |
| <a id="preuve-063"></a>preuve-063 | `outils/check-phase7.log` | Livrée dans le ZIP |
| <a id="preuve-064"></a>preuve-064 | `outils/check-phase8.log` | Livrée dans le ZIP |
| <a id="preuve-065"></a>preuve-065 | `outils/check-phase9.log` | Livrée dans le ZIP |
| <a id="preuve-066"></a>preuve-066 | `outils/phase10-appimage-final-import-order.log` | Livrée dans le ZIP |
| <a id="preuve-067"></a>preuve-067 | `outils/phase10-appimage-startup-probe.log` | Livrée dans le ZIP |
| <a id="preuve-068"></a>preuve-068 | `outils/phase10-js-offline-audit.json` | Livrée dans le ZIP |
| <a id="preuve-069"></a>preuve-069 | `outils/phase10-recovery-cli-final.log` | Livrée dans le ZIP |
| <a id="preuve-070"></a>preuve-070 | `outils/phase10-rustsec-audit.json` | Livrée dans le ZIP |

## Critères de réception du socle et du registre

Les liens ci-dessous renvoient aux observations historiques et aux suites versionnées qui permettent de les examiner. La présence actuelle d’un test n’atteste pas son exécution passée ; les nombres et résultats historiques restent ceux des rapports. Les originaux desktop des phases 2/3 ne sont pas publiés dans le ZIP.

| Critère initial | Observation/protocole conservé | Nature et limite |
|---|---|---|
| P2-D01 | Première ouverture et réouverture ; migration idempotente : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Tests exécutés rapportés ; refus adversariaux selon le cas. |
| P2-D02 | Seconde instance refusée par le verrou : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Tests exécutés rapportés ; refus adversariaux selon le cas. |
| P2-D03 | CURRENT invalide et génération absente refusés : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Tests exécutés rapportés ; refus adversariaux selon le cas. |
| P2-D04 | Paramètres invalides refusés : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Tests exécutés rapportés ; refus adversariaux selon le cas. |
| P2-B01 | Sauvegarde SQLite autonome et original contrôlé : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Tests exécutés rapportés ; refus adversariaux selon le cas. |
| P2-B02 | Empreinte, chemins et original compressible : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Tests exécutés rapportés ; refus adversariaux selon le cas. |
| P2-B03 | Restauration en génération neuve avec identités et pièces : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Tests exécutés rapportés ; refus adversariaux selon le cas. |
| P2-R01 | Analyse puis annulation sans remplacement : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Tests exécutés rapportés ; refus adversariaux selon le cas. |
| P2-R02 | Jeton périmé après mutation refusé : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Tests exécutés rapportés ; refus adversariaux selon le cas. |
| P2-R03 | Candidate modifiée ou WAL ajouté refusés : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Tests exécutés rapportés ; refus adversariaux selon le cas. |
| P2-R04 | États interrompus préparés sur disque puis rouverts : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | États de panne simulés ; aucune coupure électrique exercée. |
| P2-A01 | Archives et versions futures refusées : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Tests exécutés rapportés ; refus adversariaux selon le cas. |
| P2-A02 | Empreintes, tailles, lignes et structure contrôlées : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Tests exécutés rapportés ; refus adversariaux selon le cas. |
| P2-A03 | Chemins dangereux, doublons et liens refusés : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Tests exécutés rapportés ; refus adversariaux selon le cas. |
| P2-A04 | Fichiers supplémentaires/absents refusés ; limites relues : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Contrôle statique des plafonds ; aucun remplissage de disque. |
| P2-A05 | Vue/trigger, intégrité et FK contrôlés : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Tests exécutés rapportés ; refus adversariaux selon le cas. |
| P2-L01 | Compilation et neuf étapes desktop déclarées : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Résultat desktop déclaré, artefacts seulement locaux. |
| P2-L02 | Desktop réel dans Debian, réseau désactivé : [phase 2](validation-phase-2.md#résultats-observés), [suite de récupération](../../crates/infrastructure/tests/recovery_adversarial.rs) | Résultat desktop déclaré, artefacts seulement locaux. |
| P3-R01 | Identités et révisions préservées après réouverture : [phase 3](validation-phase-3.md#preuves-obtenues-par-cette-suite), [suite versionnée](../../crates/infrastructure/tests/registry_adversarial.rs) | Exécution rapportée ; migrations consignées séparément dans la validation finale. |
| P3-R02 | Révision périmée et nature incompatible refusées atomiquement : [phase 3](validation-phase-3.md#preuves-obtenues-par-cette-suite), [suite versionnée](../../crates/infrastructure/tests/registry_adversarial.rs) | Exécution rapportée ; migrations consignées séparément dans la validation finale. |
| P3-G01 | Cycles passés/futurs refusés ; périodes réciproques disjointes admises : [phase 3](validation-phase-3.md#preuves-obtenues-par-cette-suite), [suite versionnée](../../crates/infrastructure/tests/registry_adversarial.rs) | Exécution rapportée ; migrations consignées séparément dans la validation finale. |
| P3-G02 | Chevauchements et somme supérieure à un refusés aux frontières : [phase 3](validation-phase-3.md#preuves-obtenues-par-cette-suite), [suite versionnée](../../crates/infrastructure/tests/registry_adversarial.rs) | Exécution rapportée ; migrations consignées séparément dans la validation finale. |
| P3-N01 | Droit économique inconnu conservé : [phase 3](validation-phase-3.md#preuves-obtenues-par-cette-suite), [suite versionnée](../../crates/infrastructure/tests/registry_adversarial.rs) | Exécution rapportée ; migrations consignées séparément dans la validation finale. |
| P3-N02 | Valeur étrangère conservée sans conversion supposée : [phase 3](validation-phase-3.md#preuves-obtenues-par-cette-suite), [suite versionnée](../../crates/infrastructure/tests/registry_adversarial.rs) | Exécution rapportée ; migrations consignées séparément dans la validation finale. |
| P3-H01 | Archivage et fin exclusive préservant les états historiques : [phase 3](validation-phase-3.md#preuves-obtenues-par-cette-suite), [suite versionnée](../../crates/infrastructure/tests/registry_adversarial.rs) | Exécution rapportée ; migrations consignées séparément dans la validation finale. |
| P3-B01 | Registre et consolidation retrouvés après restauration : [phase 3](validation-phase-3.md#preuves-obtenues-par-cette-suite), [suite versionnée](../../crates/infrastructure/tests/registry_adversarial.rs) | Exécution rapportée ; migrations consignées séparément dans la validation finale. |
| P3-M01 | Vrai schéma 1 migré sur copie, source conservée : [phase 3](validation-phase-3.md#preuves-obtenues-par-cette-suite), [suite versionnée](../../crates/infrastructure/tests/registry_migrations.rs) | Exécution rapportée ; migrations consignées séparément dans la validation finale. |
| P3-M02 | Ancienne structure invalide refusée avant activation : [phase 3](validation-phase-3.md#preuves-obtenues-par-cette-suite), [suite versionnée](../../crates/infrastructure/tests/registry_migrations.rs) | Exécution rapportée ; migrations consignées séparément dans la validation finale. |

Les refus d’archives hostiles réellement exécutés sont distincts des lectures statiques des limites. Aucune campagne historique ne revendique une panne physique, une coupure électrique ou un disque rempli. Les nouvelles corrections sont suivies dans le [suivi courant](../projet/suivi-corrections.md), sans modifier rétroactivement ces résultats.
