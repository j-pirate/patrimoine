# Validation — phase 7 : métaux et sociétés

Campagne du **11 novembre 2026, Europe/Paris**, sur des coffres isolés et des données synthétiques. Les lots physiques complètent les actifs existants ; les observations de sociétés restent hors bilan. Le [guide](17-metaux-et-societes.md) précise les formulaires, conventions et limites.

## Vérification complète

`source scripts/cloud-env.sh` puis `CARGO_BUILD_JOBS=3 bash scripts/check.sh` : **réussi, sortie 0**. Le [journal](/workspace/.tooling/check-phase7.log) conserve les commandes effectivement exécutées.

| Campagne | Résultat |
|---|---|
| TypeScript / Vite | Types et compilation réussis ; nouveaux modules chargés à la demande, embarqués localement |
| Vitest | **118 tests**, 13 fichiers, aucun échec ni ignoré |
| Domaine Rust | **205 tests** : 152 précédents, 21 métaux, 18 sociétés, 14 intégrations financières |
| Infrastructure Rust | **147 tests** : 129 précédents, 9 métaux, 7 sociétés et 2 migrations supplémentaires |
| Workspace Rust | **352 tests réussis**, aucun échec ni ignoré ; autres cibles et doctests sans cas supplémentaires |
| Formatage et Clippy | `rustfmt` conforme ; toutes cibles et fonctionnalités, `-D warnings`, réussi |
| Tauri Linux | Binaire `custom-protocol` construit avec interface embarquée |

Les tests UI emploient des réponses IPC contrôlées. Ils vérifient les décimaux textuels, inconnus distincts de zéro, ouvertures, cotations, primes, choix de journal, corrections, transferts, observations et distributions. Les réponses périmées, échecs de chargement K, formulaires conservés après conflit et lecture seule à K=0 sont couverts. Le parcours natif constitue une vérification séparée des commandes réellement reliées à SQLite.

## Calculs et intégrations contrôlés

| Cas | Résultat vérifié |
|---|---|
| O10 : deux pièces de 31,1034768 g à pureté 0,9 ; cours 60 EUR/g fin | Poids fin exact 55,98625824 g ; intrinsèque 3 359,1754944 EUR, affiché 3 359,18 |
| O12 : kilogramme et once troy | 1 000 g et 31,1034768 g exactement ; aucune once ordinaire implicite |
| Vente de la moitié d’un lot homogène | Quantité, poids et coût restant divisés par deux ; allocation du coût historique distincte du prix reçu |
| Coût initial 2 400, moitié vendue 1 800, frais 10 | Coût vendu 1 200, recette nette 1 790, gain réalisé 590, coût restant 1 200 |
| Achat supplémentaire dans un lot homogène | Coût exact ajouté puis sorties proportionnelles ; aucun FIFO global caché |
| Coûts et primes multidevises | Change historique aux entrées de coût, change de stock à D ; prime convertie une seule fois dans la devise du cours |
| Intrinsèque ou revente | Méthode explicite ; prime/décote absentes ne valent pas zéro ; intrinsèque connu sans estimation de revente complète |
| Ouverture absente / quantité confirmée zéro | Stock inconnu / valeur nulle connue, même sans cotation ni cours FX inutile |
| Cours futur, coût absent, devise manquante | Aucun cours futur ; coût absent ne supprime pas une valeur intrinsèque calculable ; change indispensable absent conservé inconnu |
| Ancienne estimation générique 999 999 sur l’actif métal | Remplacée par le stock exact dans le contrôle et le bilan, sans valeur synthétique stockée |
| Vente réelle non rapprochée | Stock réduit, aucune banque supposée, net incomplet malgré une couverture déclarée |
| Transfert du lot entre membres ou de lieu | Pas de nouvelle quantité, coût ou banque ; répartition individuelle modifiée, stock familial conservé |
| Société globale détentrice du métal | Sous-jacent caché par la valeur globale, sans deuxième position métal |
| Observations de trésorerie/dette/créance/valeur globale | Données de pilotage conservées sans nouvel actif, passif ou banque |
| Périodes mensuelles, trimestrielles et annuelles | Fréquences parallèles non sommées ; tendances seulement sur périodes adjacentes compatibles ; base nulle non calculable |
| O13 : dividende déclaré puis réellement payé | Déclaration sans banque ; paiement explicite, deux faces et une opération, net familial inchangé à hauteur interne |
| Paiement de dividende incomplet | Chaque face visible exige son lien bancaire exact ; aucun résultat présumé reçu |
| Opération traversant une valeur globale ancienne | Bilan incomplet jusqu’à une estimation tenant compte de l’opération, sans correction implicite des parts |
| Ligne bancaire partagée entre anciens modules, métal et dividende | Plafond global exact ; pas de capacité dépensée plusieurs fois |

Les 14 tests d’intégration utilisent des résultats rationnels indépendants des helpers de valorisation : remplacement de l’estimation, arrondi final, propriété distincte du payeur, sociétés globales, primes inconnues, zéro sans FX, ventes non rapprochées et dividendes. Les variations de droits et les flux sont examinés par périmètre opaque, avant compensation des mouvements internes au même périmètre. Une estimation antérieure au transfert ne permet pas d’annoncer un net complet.

Le pont historique des estimations exclut les stocks métal dérivés. Un test SQLite conserve une ancienne estimation générique et vérifie qu’elle ne réapparaît pas dans ce pont ; l’API et l’interface en exposent la portée partielle. Ce pont ne prétend pas expliquer la performance globale du nouveau bilan.

## Persistance, corrections et migration

Les transactions atomiques conservent la séquence globale et les références exactes des sources, journaux et lignes. Corriger une vente ou un dividende lié réutilise son opération d’origine et révise ses liens ensemble. Une nouvelle opération, une autre opération existante, une source ou un journal périmés et une correction isolée sont refusés sans mutation partielle. Les deux faces d’un dividende et les plafonds des déclarations sont contrôlés. Les lignes enfants historiques survivent aux corrections, annulations et sauvegardes.

Le transfert de propriété ferme explicitement tous les droits actifs et ouvre les nouveaux dans un même changement ; les sommes juridiques et économiques restent conservées à toutes les dates affectées. Une ligne passée n’est pas réécrite comme si le nouveau titulaire avait toujours possédé le lot. Les changements de poids, métal, pureté, actif, devise de coût et début du suivi ne réinterprètent pas l’inventaire historique.

La migration `0006_metals_companies.sql` ajoute huit tables typées au schéma 6 et étend le registre d’objets sur une candidate. Les cinq anciens scripts restent inchangés. Les 14 tests de migration incluent un vrai coffre et une vraie archive du schéma 5 avec liens et révisions ; la copie migre, les sources et anciennes générations restent conservées. Les anciennes archives 1–4 restent exercées. Le format ZIP demeure en version 1.

## Parcours Linux natif

`bash scripts/test-desktop-cloud.sh` : **réussi, sortie 0** sur le binaire final. Le [rapport JSON](../.local/desktop-smoke.ku6m530N/desktop-result.json) contient **42 étapes réussies**, aucune échouée ni non exécutée, avec dialogues natifs activés. L’application fonctionne dans Debian 13 sous WebKitGTK/Xvfb, avec utilisateur non privilégié, réseau désactivé et coffre neuf. Les scénarios des phases précédentes restent exécutés.

Le nouveau parcours vérifie :

- Lot créé depuis l’interface, rattaché à l’actif existant ; une ancienne estimation de 999 999 EUR ne s’ajoute pas à sa valeur. Sans ouverture, la quantité reste inconnue. Deux pièces d’une once troy, pureté française `0,9`, coût initial 2 400 EUR et cours de 60 EUR/g donnent exactement 55,98625824 g fin et 3 359,18 EUR.
- Cotation corrigée de 60 à 61 avec révision motivée : les anciennes connaissances donnent encore 3 359,18 EUR, les actuelles 3 415,16 EUR avant vente. La consultation à l’ancien K est en lecture seule.
- Vente d’une pièce : une seule recette nette de 1 790 EUR, coût sorti 1 200, gain réalisé 590, coût restant 1 200. Avec le cours corrigé, le stock restant vaut exactement 1 707,58087632 EUR, affiché 1 707,58.
- Transfert du lot entier vers un coffre suisse, sans quantité ni banque supplémentaire. Transfert de droits d’Alice à Benoît : sa valeur sort du sous-total individuel d’Alice et entre dans celui de Benoît pour le même montant exact, sans changer le sous-total familial.
- Observation annuelle créée depuis le formulaire : chiffre d’affaires 120 000, charges 80 000, résultat d’exploitation 40 000 et trésorerie indicative 999 999. Cette observation ne modifie aucun sous-total patrimonial.
- Déclaration de dividende de 300 EUR puis paiement : une commande écrit le débit société et le crédit bénéficiaire avec leurs deux liens exacts. Les banques société et bénéficiaire passent respectivement à 700 et 2 430 EUR ; le sous-total familial ne gagne pas 300 EUR. Le contrôle de société et le [rapport métal](../.local/desktop-smoke.ku6m530N/metal-report.png) sont affichés depuis leurs commandes Rust réelles ; le [contrôle société](../.local/desktop-smoke.ku6m530N/company-report.png) conserve les observations séparées des stocks.
- Import/export d’un original, sauvegarde complète, analyse et annulation sans mutation, confirmation avec sauvegarde préalable, activation d’une nouvelle génération et restitution des huit ensembles métier. Les [rapports métal et société avant/après](../.local/desktop-smoke.ku6m530N/metal-company-restore-comparison.json) et le [bilan financier avant/après](../.local/desktop-smoke.ku6m530N/financial-restore-comparison.json) sont intégralement identiques à K=111. L’ancien stock et l’ancienne cotation sont aussi reproduits après restauration.

Le bilan de cette fixture reste partiel : actifs connus 355 949,88 EUR, passifs connus 100 242 EUR, différence connue 255 707,88 EUR ; les anciennes sources locatives non rapprochées empêchent de présenter cette différence comme un net exhaustif. Les [oracles du parcours](../.local/desktop-smoke.ku6m530N/metal-company-oracles.json) sont conservés.

Le pilote attend deux images avant un clic pour laisser les effets de rendu s’établir ; il réouvre le contrôle du lot après un changement de K qui remonte le module sur son onglet initial. Le délai de restauration de la fixture enrichie a été porté de 25 à 60 secondes après observation de l’état « en cours » : la campagne réussie mesure **27,89 secondes**, sauvegarde préalable et contrôles compris. Aucune assertion de montant, source ou restauration n’a été retirée. Ces mesures ne constituent pas la qualification de performance prévue en phase 10.

Les saisies textuelles passent par le clavier WebDriver ; les champs date natifs émettent `input/change` puis attendent leur stabilisation. Les fixtures complémentaires emploient l’IPC réel. Aucun état React ou résultat de calcul simulé n’est injecté dans ce parcours. Les tests UI isolés restent une campagne séparée.

## Inspection indépendante des archives

Le [script du scénario synthétique](../.local/desktop-smoke.ku6m530N/independent-backup-inspection.py), utilisant seulement `zipfile`, `hashlib`, `sqlite3` et `fractions.Fraction`, lit les deux ZIP sans appeler le backend. Son [résultat indépendant](../.local/desktop-smoke.ku6m530N/independent-backup-inspection.json) est **réussi**.

Les bases sont au schéma 6, à K=111 et K=112, avec 62 tables, 128 objets et 135 révisions. Les quatre fichiers manifestés dans chaque archive ont les tailles et empreintes annoncées ; `integrity_check` répond `ok` et aucune violation FK n’est trouvée. Les six checksums de migrations correspondent aux fichiers SQL, et les migrations 1–5 sont identiques à celles de l’archive précédente de phase 6.

Le recalcul rationnel du journal, des mouvements, coûts et poids retrouve les banques −42 / 2 430 / 700 EUR, la caisse 330 TND, principal externe 99 000 EUR, dépôt restant 1 200 et impayé de janvier 700. La quantité métal restante est 1, son poids fin 27,99312912 g, son intrinsèque 1 707,58087632 EUR, son coût restant 1 200 et le gain réalisé 590. Le cours 60 et les deux unités restent disponibles au K initial. Le dividende réel de 300 a deux côtés opposés, sans paiement locatif créé par le dépôt.

Les huit liens bancaires courants référencent les sources, révisions et lignes attendues, avec dates, devises, titulaires, signes et capacités communes contrôlés. Les changements de source et de journal sont atomiques ; les droits de propriété clôturés et ouverts partagent le même changement. Toutes les données métier et lignes enfants sont identiques entre les deux ZIP ; seules les mutations attendues de réglages, changements et audit diffèrent.

## Limites

Un lot est homogène et suivi dans un seul lieu à chaque date. Une répartition physique simultanée entre lieux demande des lots identifiables distincts ; aucune scission implicite ni généalogie de lots enfants n’est inventée. La méthode de coût est proportionnelle au lot identifié, sans règles fiscales de FIFO. Les corrections d’inventaire sont des faits motivés, pas des ventes supposées. Les anciennes ouvertures ne créent pas de nouveaux paiements.

Les dividendes déclarés sont des décisions de distribution ; ils ne créent pas automatiquement une créance certaine à consolider ni un paiement. Les versements réels et leurs banques sont saisis séparément. Les indicateurs ne remplacent pas une couverture ou une méthode patrimoniale explicite, et une déclaration d’inventaire ne certifie pas des données externes absentes.

La gestion documentaire métier complète et les exports/réimports CSV/XLSX appartiennent à la phase 8 ; les tableaux de bord graphiques globaux à la phase 9 ; les performances, AppImage et qualification Linux à la phase 10. Aucune comptabilité générale, TVA, facturation, paie ou fiscalité des plus-values n’est ajoutée. Aucun changement d’environnement cloud ni nouvelle publication n’est effectué dans ce lot.
