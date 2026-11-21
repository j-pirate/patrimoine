# Immobilier et loyers — utilisation de la phase 5

Ce guide décrit le périmètre livré en phase 5. La phase 6 ajoute le [rapprochement bancaire, les financements, dépôts et créanciers explicites](comptes-et-financements.md), ainsi que leur bilan financier. Les limites indiquées ci-dessous comme « phase 6 » correspondent à cet état antérieur.

Ce module complète le registre commun. La valeur du bien reste celle de l’actif déjà créé dans **Patrimoine**. Les unités locatives, contrats et échéances ne créent pas de deuxième valeur immobilière. Les droits, estimations et taux continuent à utiliser la date économique D et les connaissances K décrites dans [le guide historique](devises-et-historique.md).

## Décrire le bien et ses unités

Dans **Immobilier et loyers → Biens et unités**, choisir **Ajouter une fiche immobilière** et rattacher la fiche à un actif physique existant. Si nécessaire, créer d’abord cet actif, ses propriétaires et ses estimations dans **Patrimoine**. Renseigner type, ville, adresse, surface, situation et période descriptive. La fin d’une période est exclue. Une surface inconnue reste vide ; elle ne devient pas nulle.

**Ajouter une unité** décrit un logement, local ou lot technique dans le bien : référence unique dans ce bien, nom, surface éventuelle, période de suivi et disponibilité déclarée. Un bien simple peut avoir une unité couvrant l’ensemble du bien. Pour des lots juridiquement valorisés et cessibles séparément, créer des actifs immobiliers distincts ; ne pas valoriser simultanément un immeuble entier et ses lots pour les mêmes surfaces.

Les unités de cette version ont une période de suivi et une disponibilité déclarée. Un bail doit rester dans cette période disponible. L’occupation calculée est **contractuelle**, à partir des baux ; elle ne prétend pas constater la présence physique du locataire.

## Séparer acquisition, estimation et cession

**Enregistrer une acquisition** permet d’indiquer date, prix, frais et devise. Le coût total est prix + frais, sans y ajouter implicitement des travaux ou intérêts. Des frais explicitement nuls se saisissent `0` ; une case vide signifie que les frais sont inconnus et laisse le coût complet non calculable. **Début du suivi sans acquisition supposée** conserve une date d’entrée dans le suivi sans inventer de coût d’achat.

Le contrôle de valeur distingue :

- Le coût historique, converti au cours de la date d’acquisition.
- La dernière estimation applicable à D, convertie au cours de D.
- Le gain latent en devise d’origine lorsque les deux sources utilisent la même devise.
- La différence entre estimation EUR et coût historique EUR. Cette différence inclut l’effet du change ; elle ne se calcule pas en convertissant simplement le gain natif au cours courant.

Exemple : coût `100 000 TND` à `0,30 EUR`, estimation `120 000 TND` à `0,40 EUR` : gain natif `20 000 TND`, différence EUR `48 000 − 30 000 = 18 000 EUR`. Sans estimation ou cours applicable, le résultat reste inconnu. Le coût ne sert pas automatiquement de valeur actuelle. Dette restante et valeur nette restent inconnues tant que les financements ne sont pas intégrés en phase 6.

**Enregistrer une cession** concerne une cession complète des droits suivis sur ce bien. La confirmation enregistre le fait de vente et clôture tous les droits concernés dans la même transaction, à la même date. Un droit commençant ce jour est annulé dans les connaissances actuelles, avec son histoire conservée. Les droits futurs incompatibles doivent être régularisés avant la vente. Un simple changement de libellé ou un archivage ne vend pas le bien.

Après cession, la fiche et les estimations restent consultables, mais l’ancienne estimation ne représente plus un stock détenu. Aucun produit de vente bancaire n’est créé par hypothèse. Pour une cession partielle, les droits datés restent gérés dans le registre ; le bouton de cession complète ne convient pas. Le cycle immobilier actuellement contrôlé comprend une acquisition ou ouverture de suivi et au plus une cession.

Une correction de cession conserve ses anciennes versions. Corriger sa date doit aussi réparer la clôture des droits dans la même opération ; si ces droits ont été modifiés depuis de manière incompatible, la correction est refusée. Les connaissances antérieures restent consultables à leur K.

## Enregistrer baux, conditions et échéances

Dans **Baux et échéances**, créer un bail pour une unité et un locataire identifié dans le registre. Les locataires peuvent être personnes ou sociétés. Les baux successifs sont conservés ; deux baux d’une même unité ne peuvent pas se chevaucher. Un changement d’unité réel demande un nouveau contrat.

Ajouter les **conditions de bail** : loyer hors charges, charges contractuelles, devise, fréquence, jour d’échéance, dépôt requis et période d’application. Le dépôt requis décrit le contrat ; il ne prouve pas qu’un dépôt a été reçu. Un bail conserve sa devise. Une révision de loyer se représente par des conditions successives avec des périodes sans chevauchement.

**Générer les échéances** sélectionne les dates d’exigibilité dans `[début, fin)`, jusqu’à dix années par demande. Les périodes sont calendaires : mois, trimestres civils ou années civiles. Elles sont bornées par les dates du bail et des conditions, pas tronquées arbitrairement par la fenêtre demandée. Le premier et le dernier segment utilisent le prorata exact de leurs jours civils dans la période calendaire, avec un arrondi à la moitié au pair à l’unité monétaire de chaque échéance. Un jour 31 est ramené au dernier jour du mois concerné ; l’exigibilité reste dans le segment contractuel.

Une deuxième génération retrouve les mêmes clés et identités ; elle ne crée pas de doublon ni de mutation inutile. Si des conditions corrigées ne correspondent plus à une échéance déjà générée, le désaccord est signalé. Les paiements et anciennes échéances ne sont pas réécrits silencieusement. Une régularisation se fait par correction motivée des sources concernées et réaffectation explicite des paiements ; une source encore utilisée ne peut pas être annulée sans traiter ses dépendances.

## Recevoir, affecter et rembourser un paiement

Dans **Paiements et dépenses**, **Enregistrer un paiement** indique le bail, le montant réellement reçu, la devise, la date, le bénéficiaire déclaré et la référence. Le motif de non-rapprochement est requis tant que le compte bancaire n’est pas intégré. Cette source locative ne fabrique pas un second encaissement bancaire et ne s’ajoute pas au stock patrimonial comme une nouvelle banque.

**Affecter un paiement** relie tout ou partie de cette réception à une échéance, sur la composante **loyer** ou **charges**. Un paiement peut couvrir plusieurs échéances ; une échéance peut recevoir plusieurs paiements. Le moteur refuse les dépassements du paiement disponible, du dû par composante et les devises incompatibles. Une avance non affectée reste visible ; elle ne solde pas automatiquement un loyer futur.

Exemple : échéance de `1 000 EUR`, réception de `400` puis de `300` : `700` affectés et `300` restant. Un complément reçu en avril ne réduit pas l’impayé observé en mars. Une fois l’échéance soldée, la date de règlement et le retard historique restent consultables.

Un **remboursement** est négatif et référence la réception d’origine. Il ne peut pas dépasser ce qui a été reçu. Pour une somme déjà affectée, préciser les lignes d’affectation reprises : paiement négatif et reprises négatives sont enregistrés atomiquement. Un remboursement sans reprise ne peut porter que sur la part disponible non affectée. Cette règle évite de présenter une créance comme réglée alors que son paiement a été rendu au locataire.

Les dépôts de garantie ne se saisissent pas comme loyers. Leur réception, restitution et obligation patrimoniale seront intégrées avec les comptes et dettes en phase 6 ; le contrat de calcul pur garantit déjà qu’un dépôt reçu n’est pas un revenu.

## Lire les indicateurs

Les filtres de bien, pays et société propriétaire complètent le contexte familial ou individuel. Un pays historique ne se déduit pas du profil courant : il doit être daté dans le registre. Les revenus courus suivent les droits économiques pendant leur période, scindée aux changements de droits ou d’appartenance. Les sommes reçues suivent le bénéficiaire déclaré à la date de réception. L’analyse économique d’un revenu conservé par une société ne constitue pas une distribution à ses associés.

| Indicateur | Base |
|---|---|
| Contractuel annualisé | Conditions actives à D, hors charges, ×12/4/1 selon fréquence |
| Exigible | Échéances arrivées à exigibilité dans la période demandée et au plus tard à D |
| Encaissé | Paiements affectés au loyer ou aux charges, à la date de réception, selon les connaissances consultées |
| Impayé | Dû résiduel à D, après allocations connues et reçues à cette date ; droits historiques du revenu conservés après une vente |
| Exploitation | Dépenses qualifiées d’exploitation, avec taxes/autres qualifications explicites ; capital et financement séparés |
| Revenu net d’exploitation (NOI) | Revenu locatif couru + charges reconnues − dépenses d’exploitation de la même période |
| Rendement brut | Contractuel annualisé / estimation de référence applicable |
| Rendement net avant financement | NOI annualisé sur une convention de 365 jours à partir des jours effectivement observés, puis divisé par la même base de valeur |
| Occupation contractuelle | Jours-unités sous bail / jours-unités déclarés disponibles dans le périmètre sélectionné |

L’annualisation sur 365 jours est une convention explicite : elle peut différer d’un total sur une année civile bissextile. La fin de la période observée est au plus tard le lendemain de D, exclu ; une borne future ne dilue pas le rendement et une durée observée nulle ne donne pas de rendement net. Une valeur de référence absente ou nulle laisse les rendements non calculables. Une période de bail sans conditions ou échéances suffisantes produit un résultat incomplet ; elle ne devient pas un loyer nul.

Pour l’occupation contractuelle, une unité est comptée une fois pour les jours où le périmètre possède une part économique connue strictement positive ; sa quote-part ne transforme pas une unité physique en fraction de jour. Les unités d’un tiers sont exclues. Des droits indéterminés qui empêchent de définir le périmètre laissent le pourcentage non calculable.

Les dépenses distinguent exploitation, travaux en capital, intérêts, principal et frais de financement. La date économique est distincte de la date de décaissement renseignée. Le cash-flow après financement reste absent tant que la couverture de trésorerie et les contrats de financement ne sont pas intégrés. Une valeur absente ne signifie jamais financement nul.

Les impayés du rapport sont répartis proportionnellement aux droits historiques des jours courus. Les affectations ne désignent pas encore un créancier juridique distinct du bénéficiaire de la réception : une répartition préférentielle ou une cession individuelle de créance demande le futur suivi explicite des droits sur créances. Le contrôle du registre n’ajoute pas ces impayés ; leur intégration avec banques, avances, dépôts et couvertures est livrée dans le [bilan financier](comptes-et-financements.md). Un rapport locatif ne certifie donc pas un bilan net exhaustif.

## Historique et récupération

Choisir une ancienne séquence K rend les formulaires immobiliers et locatifs non modifiables. Fiches, droits, conditions, échéances, paiements et corrections utilisent les connaissances de cette séquence, sans reprendre de source saisie ensuite. La bibliothèque **Documents** suit les connaissances K ; les associations sont également filtrées par la date D. À K fixe, les écritures documentaires sont bloquées. Les paramètres et les sauvegardes/exports intégraux concernent le coffre actuel et toute son histoire. Voir [Documents et portabilité](documents-et-portabilite.md).

La migration `0004` s’effectue sur une copie contrôlée et conserve la génération source. Les anciennes archives restent intactes ; la sauvegarde courante inclut les nouvelles tables, toutes leurs révisions et les clés d’échéance. Les commandes exécutées et leurs résultats figurent dans [la validation de phase 5](../historique/validation-phase-5.md).
