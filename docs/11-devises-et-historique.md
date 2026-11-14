# Devises, dates et historique — phase 4

Le registre utilise désormais des taux manuels en EUR, TND, CHF, USD et AED. Tout reste dans le coffre local. Aucune cotation n’est téléchargée et aucune parité n’est supposée, y compris pour AED/USD. La consolidation reste un contrôle du sous-total des actifs enregistrés ; les flux de trésorerie et les passifs saisissables viendront dans les phases suivantes.

## Enregistrer et vérifier un cours

Dans **Espèces et devises**, choisir **Ajouter un taux**, les deux devises, le taux et sa date d’application. La convention est toujours **1 unité de départ = taux unités d’arrivée** : `1 TND = 0,30 EUR` convertit `1 000 TND` en `300,00 EUR`. Préciser la source lorsqu’elle est disponible et le motif de l’enregistrement. Les saisies décimales acceptent la virgule ou le point et suppriment les zéros finaux inutiles ; les séparateurs de milliers et la notation scientifique ne sont pas acceptés.

Une nouvelle date correspond à une nouvelle observation. Une erreur dans une observation existante se corrige avec **Modifier**, en donnant un motif. Le registre conserve la même identité et une nouvelle révision ; la version précédente reste dans l’historique. Annuler un taux l’écarte des connaissances actuelles en conservant ses anciennes révisions. Deux observations concurrentes de la même paire à la même date sont refusées.

**Vérifier une conversion** utilise la date et les connaissances de la barre de contexte. Pour chaque paire orientée, le moteur retient le dernier cours applicable à cette date. Il applique dans cet ordre :

1. Une cotation directe.
2. L’inverse exact de la cotation inverse.
3. Deux cotations par un seul pivot, avec priorité EUR, USD, CHF, TND puis AED ; pour chaque branche, direct avant inverse.

Cette priorité est fixe : une cotation directe disponible reste prioritaire même si une autre route est plus récente. Des routes donnant des résultats différents produisent une alerte ; le moteur ne fait aucune moyenne. Sans route autorisée, le résultat est **Conversion impossible**, même pour un montant nul. Un cours postérieur à la date du calcul n’est jamais repris.

Le détail montre les devises, le sens, chaque date, source et révision, le taux exact, ainsi que la date de la branche la plus ancienne. L’ancienneté est comptée en jours civils depuis cette date. Au-delà de 30 jours, une alerte apparaît ; le montant reste calculé avec les références affichées. Il ne s’agit pas d’une certification de la qualité du cours.

Les taux sources sont strictement positifs, limités à 36 chiffres significatifs et 18 décimales. Les montants sources ont leur propre limite de 24 chiffres significatifs et 8 décimales. Les calculs et inversions restent rationnels exacts. Seul l’affichage final est arrondi à la moitié au pair : deux décimales EUR/USD/CHF/AED, trois décimales TND. Une devise cible sans précision prise en charge est refusée. Les montants et devises d’origine restent conservés.

## Distinguer date économique et connaissances

La barre globale comporte deux références indépendantes :

- **Date de référence D** : date du patrimoine observé. Elle sélectionne appartenances, droits et méthodes actifs, profils datés, dernière estimation et derniers cours applicables.
- **Connaissances K** : état enregistré au plus tard à une séquence du journal. **Connaissances actuelles** utilise toutes les corrections disponibles. Une séquence ancienne reproduit les versions connues à ce moment, sans reprendre les corrections saisies ensuite.

Exemple : un cours du 1er janvier est enregistré à `0,30`, puis corrigé en mars à `0,31`. À D = 31 janvier, les connaissances actuelles donnent `310,00 EUR` pour `1 000 TND`. En choisissant la séquence précédant la correction, la même date donne `300,00 EUR`. La date de saisie dans le journal n’est pas une nouvelle date économique du cours.

Dans **Historique**, chaque ligne présente la séquence, la date de saisie, l’objet et le motif. **Consulter la séquence** ouvre cet état en lecture seule ; **Revenir aux connaissances actuelles** rétablit la saisie. Les paramètres, documents et sauvegardes portent toujours sur le coffre actuel, comme l’indique l’interface. Choisir une séquence ne restaure pas le coffre. Les séquences comprennent aussi les changements documentaires et de paramètres ; elles ne sont pas des numéros de cours.

Une estimation de stock en devise est convertie au cours de **D**, même si son estimation est plus ancienne. La date de cette estimation et les dates des cotations restent distinctes dans le détail de contribution. Les conversions de flux à leur date de paiement appartiennent aux modules futurs.

## Dater les profils descriptifs

Les pages **Famille**, **Sociétés** et **Patrimoine** proposent **Définir une période de profil**. Choisir l’objet, la révision exacte du profil à appliquer et son intervalle `[début, fin)`. Une fin au 1er juillet exclut cette version dès le 1er juillet. Les périodes d’un même objet ne peuvent pas se chevaucher. Une révision d’une autre identité ou d’un autre type d’objet est refusée.

Pour dater un déménagement ou un changement de nom, créer/corriger d’abord le profil descriptif, puis associer les révisions souhaitées à leurs périodes. Modifier un profil de base ne modifie pas implicitement les périodes déjà enregistrées : celles-ci continuent à référencer leur version exacte. Le formulaire de modification présente le profil de base des connaissances actuelles, même si la consultation à D affiche une ancienne version datée. Les droits et appartenances conservent leurs propres dates.

Une période absente ne permet pas de déduire un ancien pays. Le libellé reste celui connu à K, avec une alerte de profil non daté ; le pays historique de la contribution reste inconnu. Les migrations ne créent pas de périodes rétroactives supposées. Une annulation sans dépendances conserve les anciennes révisions, mais une période active exige une cible courante.

## Expliquer une variation

Dans **Historique**, comparer deux dates avec **Expliquer la variation**. Le pont mesure la différence entre deux sous-totaux d’actifs enregistrés au même K. Il effectue cinq calculs exacts, en remplaçant successivement les dates des éléments suivants :

| Étape | Éléments passant de la date initiale à la date finale |
|---|---|
| Périmètre familial et méthodes | Appartenances et méthode détaillée/globale des sociétés |
| Droits économiques et actifs suivis | Participations et propriétés |
| Prix et estimations | Dernières estimations applicables |
| Change | Derniers cours applicables |

Les interactions sont attribuées à la dernière étape concernée. Par exemple, `1 000 TND × 0,30` devient `1 200 TND × 0,40` : la variation de `180 EUR` se décompose en `60 EUR` d’estimation puis `120 EUR` de change. Ce choix d’ordre est une convention de décomposition et non une mesure causale du rendement.

Une entrée dans le périmètre, une acquisition ou un changement de droits ne devient pas un gain de marché. Les flux et coûts d’acquisition ne sont pas encore inventoriés : aucun rendement ni apport de trésorerie n’est déduit. Si une référence manque, la composante apparaît **Non attribuable**. Le résidu est la différence non expliquée des sous-totaux connus, sans estimation du patrimoine absent. Même un pont réconcilié ne certifie pas un bilan net exhaustif. Les détails donnent les dates figées, sources et alertes de chaque calcul.

## Sauvegarder et retrouver ces données

La migration `0003` ajoute les taux et périodes descriptives. Les coffres et archives des schémas 1 et 2 sont contrôlés dans leur schéma d’origine, copiés, puis migrés avant activation. La génération précédente et l’archive source sont conservées. Le format ZIP reste en version 1 ; le schéma SQLite courant est 3.

Une sauvegarde contient les taux, toutes les révisions, périodes, séquences et documents originaux, avec les autres données du registre. Après restauration, vérifier D et K puis relancer le contrôle : la restauration ne sauvegarde pas un calcul affiché en tant que rapport immuable. Les preuves exécutées et limites sont dans [la validation de phase 4](10-validation-phase4.md).
