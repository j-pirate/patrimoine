# Règles métier et moteur de consolidation — proposition V1

Statut : conception de phase 1, sans implémentation ni validation automatisée. Les nombres ci-dessous sont des cas de test synthétiques, jamais des données utilisateur. Le modèle relationnel associé est décrit dans [02-modele-relationnel.md](02-modele-relationnel.md).

## 1. Contrat du moteur

Une requête contient `contexte` (famille ou personne), `date_economique D`, `connu_au K`, devise de consolidation EUR et version des règles. Par défaut, `K` désigne l'état des connaissances actuel. La borne K est résolue en séquence monotone de `change_sets`, conservée dans le rapport avec l'horodatage UTC : deux écritures ayant la même précision temporelle ne rendent pas la relecture ambiguë. Un filtre de pays, de devise ou de classe s'applique aux contributions finales, après calcul du graphe et des éliminations : filtrer préalablement les propriétaires ferait disparaître des chemins légitimes.

Le résultat contient :

- Les montants connus, le statut `complet`, `partiel` ou `non_calculable` et les motifs structurés.
- Les actifs, dettes et éliminations retenus, leurs identifiants, propriétaires juridiques, poids économiques et chemins de participation.
- Pour chaque conversion : montant et devise d'origine, valeur EUR, identifiants des cours, dates, méthode directe/inverse/croisée et ancienneté.
- La date de valorisation de chaque actif, les alertes, les objets exclus et la raison de chaque exclusion.
- Les versions du schéma, du moteur et des données permettant de reproduire le résultat.

Une valeur manquante est distincte de zéro. Un sous-total connu ne devient jamais un « patrimoine total » par simple absence d'erreur technique. Une estimation incomplète de l'actif ou du passif interdit d'annoncer un net exact, même si le sous-total est calculable. Le sous-total net connu n'est pas une borne basse garantie.

## 2. Représentations numériques et arrondis

### 2.1 Stockage et frontières

Les montants, taux et poids sont transmis à Rust et stockés en SQLite comme chaînes décimales canoniques : signe facultatif, point décimal, aucun séparateur de milliers, ni exposant, ni `NaN`, ni infini. La saisie française `1 234,56` est normalisée et validée avant persistance. Les droits sont des fractions exactes stockées en deux colonnes `<share>_num` et `<share>_den`, entiers textuels non négatifs, dénominateur strictement positif, fraction réduite ; ils peuvent ainsi représenter `1/3` sans inventer `33.3333 %`. Un droit inconnu a ses deux colonnes nulles. Un pourcentage décimal saisi est converti exactement en fraction. React ne calcule pas les soldes à partir de `number` JavaScript.

Bornes V1 proposées : montants à 24 chiffres significatifs et 8 décimales maximum ; pourcentages décimaux saisis à 12 décimales maximum, ou fraction exacte dont numérateur et dénominateur sources ont chacun au plus 36 chiffres ; taux et quantités à 18 décimales et 36 chiffres significatifs maximum. Ces limites sont contrôlées à l'import comme à la saisie et n'imposent aucune troncature aux fractions intermédiaires de calcul. Un dépassement est rejeté avec un message explicite, jamais tronqué. Elles sont indépendantes des décimales d'affichage des devises : EUR/CHF/AED/USD = 2, TND = 3.

Pour garantir les inversions et les chemins multiples, le calcul interne utilise des rationnels exacts à entiers de précision arbitraire : une chaîne décimale devient une fraction entière, et les produits/divisions ne sont pas arrondis intermédiairement. `num-bigint`/`num-rational` est le choix proposé. Les quotients non terminants restent rationnels ; les exports de calcul peuvent joindre numérateur et dénominateur à la présentation décimale. Les données sources exportées restent les chaînes décimales originales.

L'absence de limite artificielle du nombre de membres n'exclut pas les protections techniques : taille maximale d'une entrée, budget de calcul, annulation et diagnostic explicite d'une opération trop volumineuse. Un dépassement n'est pas un résultat partiel silencieux.

### 2.2 Convention d'arrondi

Additionner les contributions exactes puis arrondir une seule fois au centime EUR avec l'arrondi au plus proche, égalité vers le pair (`half-even`). Exemple : `1.005 EUR → 1.00`, `1.015 EUR → 1.02`. Une ligne ou un total dans sa devise d'origine utilise l'unité mineure correspondante ; les taux affichés peuvent avoir davantage de décimales sans devenir la source du calcul.

Les répartitions d'écran doivent se réconcilier au total : une allocation déterministe des centimes résiduels, par plus grands restes puis identifiant stable, ajuste uniquement les valeurs affichées. Conserver les contributions exactes et indiquer cet ajustement. Additionner les nets individuels déjà arrondis n'est pas la méthode de calcul du net familial.

### 2.3 Signes et unités

- Une valorisation d'actif ordinaire est positive ou nulle ; une dette certaine est présentée positive et soustraite du brut.
- Une écriture de trésorerie est signée : crédit positif, débit négatif. Les frais sont des sorties explicites.
- Les quantités en stock sont positives ou nulles ; les mouvements sont signés. Un stock négatif est refusé.
- Les droits sont stockés en fractions `[0,1]`, l'interface affiche des pourcentages.
- Ne jamais additionner des devises avant conversion. Un solde bancaire négatif devient une exposition de découvert dans les rapports ; il n'est pas compté simultanément en liquidité négative et en dette séparée.

## 3. Propriété : trois notions distinctes

Une entité est une personne physique, une société ou une autre entité juridique explicitement prise en charge. Le statut « membre de la famille » est une appartenance datée, pas un type exclusif de propriétaire. Un tiers peut être copropriétaire, associé, créancier ou locataire.

`company_participations` relie le détenteur à la société détenue ; `asset_ownerships` relie le propriétaire à l'actif. Chaque relation distingue :

1. La quote-part juridique : titre de propriété ou part du capital.
2. La quote-part économique utilisée pour la valeur attribuable.
3. Les droits de vote, renseignés séparément lorsqu'ils sont pertinents.

Les droits économiques ordinaires V1 sont proportionnels. Le démembrement, les préférences de liquidation, cascades de distribution et instruments convertibles demandent une règle spécifique : les enregistrer descriptivement est possible, mais ils empêchent une consolidation présentée comme complète tant qu'ils ne sont pas modélisés. Ni les votes ni le pourcentage juridique ne remplacent silencieusement des droits économiques inconnus. La reprise d'un même pourcentage pour les trois notions nécessite une confirmation de saisie.

Une fraction connue n'implique pas que le solde soit extérieur à la famille. Une fraction dont le titulaire est inconnu reste un reliquat signalé dans `ownership_declarations`, sans relation de propriété vers une entité fictive. Les droits des propriétaires connus sont conservés ; le solde non attribué rend le périmètre incomplet. Les sommes inférieures à 100 % sont autorisées uniquement avec ce statut incomplet documenté, ou une tranche résiduelle réellement attribuée à un tiers identifié. Les sommes supérieures à 100 %, fractions négatives et périodes incompatibles sont refusées.

À une date donnée, chaque ensemble de détenteurs représente une partition du même droit, sans doublons ; une copropriété directe et une participation indirecte sont deux chemins possibles vers une même exposition, pas deux actifs matériels. Un portefeuille de parts n'est pas ajouté comme actif indépendant lorsqu'il duplique une relation de participation existante.

## 4. Graphe et propagation économique

### 4.1 Invariants temporels

Le graphe orienté va du détenteur vers la société détenue, puis vers les actifs. Les personnes physiques sont des racines ; les sociétés peuvent être des nœuds intermédiaires. Le graphe des participations doit être acyclique pour chaque intervalle effectif, et pas seulement aujourd'hui.

Avant une insertion ou correction rétroactive, vérifier toutes les dates frontières de la période concernée. Un cycle actif sur une seule journée suffit à rejeter la transaction. Deux relations réciproques sur des intervalles disjoints ne constituent pas un cycle simultané. Vérifier également les sommes de fractions à chaque frontière. Tous les changements corrélés sont atomiques : on ne sauvegarde pas d'abord la vente puis l'achat dans deux transactions laissant un état intermédiaire exploitable.

### 4.2 Algorithme sur un DAG

À `D,K`, charger un instantané cohérent des révisions applicables et déterminer les personnes du périmètre. Pour une personne `m`, son poids propre vaut 1 ; le poids d'une autre personne vaut 0. Pour la famille, le poids initial de chaque membre actif vaut 1, celui d'un tiers vaut 0. Le **net** familial équivaut à la somme des nets individuels exacts des membres présents à `D`, avant arrondi, sous une même couverture. Le brut et les dettes familiaux ne sont pas la somme des bruts et dettes individuels après élimination des prêts réciproques.

Pour une société `c`, en ordre topologique :

`W_P(c) = Σ[détenteur h de c] W_P(h) × droit_économique(h,c)`.

Pour un actif matériel ou financier `a` :

`Q_P(a) = Σ[propriétaire h de a] W_P(h) × droit_économique(h,a)`.

Les chemins sont additionnés, leurs fractions multipliées. Le calcul ne marque pas un nœud « déjà vu » pour ignorer les autres chemins : cela perdrait de vraies participations. Le parcours dynamique calcule chaque nœud une fois tout en cumulant toutes ses contributions entrantes. Complexité linéaire en nœuds et arêtes, hors coût des rationnels ; les explications de chemins peuvent être paginées plutôt qu'énumérer un nombre exponentiel de chemins.

Un résultat `Q > 1` est une incohérence à diagnostiquer, jamais une valeur à plafonner automatiquement. Un chemin comportant une fraction inconnue génère une contribution non calculable et un motif d'incomplétude ; les contributions des autres chemins restent identifiables.

### 4.3 Exemples attendus

| Structure à la date choisie | Résultat |
|---|---|
| Appartement de 400 000 EUR : A 50 %, B 30 %, tiers 20 % | A 200 000 ; B 120 000 ; famille 320 000 ; valeur juridique entière 400 000 EUR |
| A → X 60 %, X → Y 50 %, Y → appartement 100 % | A détient économiquement 30 % ; appartement 400 000 → 120 000 EUR |
| A 40 %, B 35 %, tiers 25 % d'une société | Poids familial 75 % |
| A → X 60 %, A → Y 20 %, X → Y 50 %, Y → actif 100 % | A détient 20 % + 60 % × 50 % = 50 % ; actif 100 000 → 50 000 EUR |
| A → X 100 %, X → Y 100 %, Y → Z 100 %, Z → actif 100 % | Exposition finale 100 %, sans limite métier de trois niveaux |
| X → Y 40 % et Y → X 10 % pendant un même intervalle | Enregistrement refusé, diagnostic du cycle et des dates |

## 5. Frontière de consolidation des sociétés

`company_policies` définit une méthode datée ; elle n'est jamais choisie automatiquement selon le résultat le plus élevé.

### 5.1 Mode détaillé

Traverser les participations et retenir les actifs sous-jacents, trésoreries et passifs détaillés. La société elle-même n'ajoute aucune valeur globale supplémentaire. Une estimation indicative du prix des parts ou des indicateurs de société peut être affichée en comparaison, mais reste hors total.

Exemple : société familiale à 75 %, immeuble 400 000, banque 100 000, dette de l'entité 200 000 EUR. Attribution familiale : brut 375 000, dette 150 000, net 225 000 EUR. Une estimation des parts de 320 000 n'est pas ajoutée au résultat.

Les attestations de complétude portent sur les actifs, dettes, détentions et dates couvertes. « Aucune dette » est une déclaration explicite, pas la conséquence d'une table vide. Un élément important connu mais non chiffré rend le total partiel. Un actif net négatif est affichable comme exposition analytique en transparence ; il interdit un total patrimonial présenté comme complet tant que responsabilité et engagements des associés ne sont pas qualifiés, ou qu'une valeur explicite des parts n'est pas retenue en mode global. Ni plafonnement tacite à zéro, ni conversion de la perte en dette personnelle juridiquement exigible.

### 5.2 Mode valeur globale nette

Utiliser exclusivement une valeur datée des capitaux propres, accompagnée de sa méthode et de son périmètre inclus. Une valeur d'entreprise avant dette (`enterprise value`) n'est pas acceptée comme valeur des parts sans pont documenté vers les capitaux propres : trésorerie, dette, date et éventuels autres ajustements.

La valeur nette globale est un atome de consolidation au niveau de la société. Arrêter la traversée pour les chemins qui la traversent. Ne pas ajouter ses actifs ni soustraire de nouveau ses dettes incluses. Une détention directe indépendante dans une filiale reste cependant comptée : la frontière porte sur un chemin et non sur l'identité de toutes les filiales.

Le moteur distingue donc les poids du graphe complet, utiles pour expliquer les droits, des **poids visibles de consolidation**. Ces derniers démarrent aux personnes sélectionnées et s'accumulent topologiquement. À un nœud détaillé, propager son poids visible vers participations, actifs et faces de prêts ; à un nœud global, émettre uniquement `poids_visible × valeur_capitaux_propres`, puis ne rien propager vers l'aval. Un autre chemin entrant directement dans une filiale peut lui donner un poids visible indépendant. Cela s'applique aussi lorsque cette filiale est elle-même globale. Utiliser le poids complet d'une filiale à l'intérieur de la somme de consolidation compterait à nouveau la portion cachée derrière une frontière opaque.

Exemple : A possède 60 % de X, X possède 50 % de Y ; A possède aussi directement 20 % de Y. X est global net à 500 000 et Y détaillée, actif net 200 000 EUR. A retient `0,60 × 500 000 + 0,20 × 200 000 = 340 000 EUR`. Ajouter encore `0,60 × 0,50 × 200 000` serait un double compte.

Un même montant ne peut être à la fois une valeur globale et une somme d'actifs détaillés dans le même périmètre. Passer de méthode à une date produit une variation de méthode identifiée ; ce changement n'est pas une performance.

### 5.3 Présentation et limites

Une valeur nette de parts ne révèle pas les actifs et dettes bruts de la société. Dans un périmètre mixte, présenter séparément « actifs détaillés attribuables », « participations à valeur nette globale » et « dettes détaillées ». Leur différence donne un net économique sous les réserves de couverture. Ne pas baptiser la somme « patrimoine brut sous-jacent complet », ni faire croire que l'endettement sous-jacent total est connu.

Une créance ou une dette envers une société opaque nécessite un contrat de couverture : quels contrats, soldes et dates sont intégrés dans la valeur nette ? Une créance personnelle contre cette société peut s'ajouter à la valeur de ses parts si le passif correspondant est déjà inclus ; ce n'est pas un double compte. En revanche, supprimer cette créance comme « interne » sans connaître le passif intégré fausserait le net. Aucun ajustement réciproque implicite n'est permis. L'absence de couverture vérifiable rend le périmètre concerné incomplet.

Les garanties, engagements de soutien et pertes au-delà de l'investissement ne sont pas déduits automatiquement d'une valeur globale : leur traitement exige une obligation certaine distinctement enregistrée. Une situation de capitaux propres négatifs ne devient pas silencieusement une dette personnelle de chaque associé.

## 6. Dettes, créances et éliminations

### 6.1 Un contrat, deux faces

`debt_contracts` est la source unique du prêt, de son encours et de sa devise. Les faces créancière et débitrice sont des relations `debt_parties`, pas deux prêts indépendants. Chaque face porte ses quotes-parts économiques explicites ; le droit juridique et les éventuelles obligations solidaires sont conservés séparément. Une créance enregistrée comme actif générique ne peut pas dupliquer un contrat de prêt.

Pour plusieurs coemprunteurs, la solidarité juridique n'est pas répartie mécaniquement à parts égales : l'allocation économique doit être renseignée et justifiée. Si elle manque, les dettes individuelles sont inconnues. Les garanties et cautions sont des expositions conditionnelles à part ; une réalisation crée une dette certaine liée, sans laisser deux déductions.

Un lien entre prêt et immeuble indique objet du financement, sûreté ou affectation analytique. Il ne décide jamais qui supporte la dette. Si A possède 50 % d'un bien de 400 000 mais doit personnellement 150 000, son brut est 200 000 et son net après cette dette est 50 000 EUR ; diviser la dette par deux serait faux. Une dette de société se rattache à cette société et se transmet par ses droits économiques dans le calcul détaillé ; elle n'est pas ventilée suivant les quotes-parts d'un immeuble qu'elle finance.

Encours nominal à `D` :

`capital_initial + tirages + ajustements_capital − remboursements_principal`.

Les intérêts et frais ne diminuent pas le principal. Les intérêts courus ne sont pas capitalisés sans clause et mouvement explicites. Un solde bancaire associé au remboursement doit correspondre à principal + intérêts + frais, avec liaison vers l'opération de trésorerie, sans second décaissement. Si un solde d'ouverture à une date est le point de départ du suivi, les mouvements antérieurs ne sont pas réajoutés.

### 6.2 Consolidation proportionnelle des prêts détaillés

Pour un prêt d'encours EUR `L`, `c` est le poids économique du périmètre sur la face créancière, `d` celui sur la face débitrice. Les calculer indépendamment, en tenant compte des fractions économiques des parties. Avant élimination : créance attribuable `cL`, dette attribuable `dL`.

La convention de présentation V1 élimine la fraction réciproque `E = min(c,d) × L` à la fois du brut et des dettes. Ainsi :

`créance_résiduelle = max(c−d,0) × L` ;

`dette_résiduelle = max(d−c,0) × L` ;

`impact_net = (c−d) × L`, inchangé par l'élimination.

Cette convention est une consolidation patrimoniale proportionnelle, pas une consolidation comptable par contrôle. Les poids des deux faces doivent être connus, le contrat doit être le même et les soldes et dates doivent se réconcilier. Si les deux faces n'ont pas la même mesure (dépréciation de créance, litige, intérêts non reconnus), éliminer uniquement la valeur appariée justifiée et isoler le résidu ; ne pas fabriquer une symétrie.

| Cas, encours 100 000 EUR | Brut/dette avant élimination | Élimination | Solde après élimination |
|---|---|---|---|
| A et B membres, A prête à B | 100 000 / 100 000 | 100 000 des deux côtés | 0 / 0 ; impact net 0 |
| A membre prête à société détenue à 60 % par la famille | 100 000 / 60 000 | 60 000 des deux côtés | Créance 40 000 ; impact net +40 000 |
| Société familiale à 75 % prête à une autre familiale à 50 % | 75 000 / 50 000 | 50 000 des deux côtés | Créance 25 000 ; impact net +25 000 |
| A membre prête à un tiers extérieur | 100 000 / 0 | 0 | Créance 100 000 |

Les vues individuelles gardent le prêt A→B : A a une créance et B une dette. Au niveau familial, neutraliser le prêt ne supprime ni son contrat, ni ses paiements, ni les historiques individuels.

### 6.3 Filtres et explications

Les éliminations ont leurs propres lignes de rapport liées au contrat, aux deux contreparties et à leur poids. Affecter chaque ligne de réduction aux attributs de la face qu'elle annule : réduction d'actif dans la classe, la devise et le pays de la créance ; réduction de passif dans ceux de la dette. Les positions résiduelles conservent les attributs de leur face. La somme des classes et pays est calculée sur ces positions résiduelles après élimination ; filtrer une seule face peut donc montrer une réduction différente de celle de l'autre face, avec accès au rapprochement complet. Les vues juridiques exposent toujours les montants entiers et les garanties, distincts des montants économiques consolidés.

## 7. Formules patrimoniales et valorisation

Pour les actifs détaillés ordinaires valorisables : `B_P = Σ Q_P(a) × V_EUR(a)` ; ajouter les créances détaillées et les valeurs nettes globales selon les règles précédentes. Pour les dettes détaillées certaines : `D_P = Σ poids_débiteur × encours_EUR`. Retirer les mêmes éliminations appariées à B et D. Le net est `N_P = B_P − D_P`. Dans le cas mixte, les libellés de la section 5.3 remplacent « brut sous-jacent ».

Une valeur d'acquisition n'est pas une estimation actuelle par défaut. Le dernier montant connu antérieur ou égal à `D` est retenu pour les estimations datées, avec sa date et une alerte d'ancienneté. Une valeur future n'est jamais rétropropagée. Une dérogation « coût d'acquisition utilisé comme approximation » doit être explicite et signalée, sans remplacer la série des estimations. Après cession, la détention est close, et la trésorerie ou créance de cession prend le relais ; l'actif vendu ne reste pas dans le total courant.

Un actif sans valorisation ou sans propriétaire n'est pas effacé du rapport : il apparaît dans les exclusions non chiffrées. Une société ne devient pas détaillée complète parce que ses quelques actifs connus sont valorisés.

## 8. Historique : deux axes de temps

Les périodes de validité des états utilisent `[valid_from, valid_to)`, fin exclusive, `NULL` pour une fin non déterminée. Une cession au 1er juillet ferme les droits du vendeur au 1er juillet et ouvre ceux de l'acquéreur à cette même date. Les flux ont une date d'effet économique ; leur date de saisie UTC ne remplace pas cette date.

Chaque correction crée une nouvelle `revision` rattachée à l'identité stable de l'objet, mentionne son prédécesseur et un motif. Une chaîne de révisions est totale pour cet objet, sans branche concurrente ; les ensembles liés partagent un `change_set`. Sélectionner à la séquence `K` la dernière révision connue de chaque objet/segment, puis appliquer sa date ou sa période économique à `D`. Lorsqu'une correction scinde un intervalle, elle retire la version antérieure et crée les nouveaux segments dans un seul changement atomique. La sémantique exacte de résolution des révisions doit être identique pour les états, flux, annulations et imports.

Deux lectures sont proposées :

- « Situation au 31 décembre, corrigée avec les connaissances actuelles » : `D` passée, `K` actuelle ; lecture par défaut.
- « Situation au 31 décembre telle que connue à l'époque » : `D` et `K` figées ; indispensable pour reproduire un rapport ancien.

Les historiques d'acquisition, cession, propriété, périmètre familial, comptes, lots, prêts, taux, cours, baux et estimations sont tous interrogés sur ces mêmes axes. Une correction n'efface pas l'ancien fait de l'audit. Un archivage masque un objet des nouvelles saisies, sans retirer ses contributions historiques.

Un solde est reconstruit à partir du point d'ouverture du suivi et des flux applicables jusqu'à `D`. Avant cette date, l'état est « non suivi » plutôt que zéro. Les caches de calcul et points mensuels sont dérivés, invalidables par correction rétroactive et recréables ; ils ne sont pas les seules données historiques.

### 8.1 Attribution des flux et changement de périmètre

Un stock est attribué avec les droits à sa date d'observation. Un revenu économique locatif est attribué avec les droits économiques de la période courue, scindée aux changements de propriétaire ; l'encaissement est attribué au titulaire de la trésorerie à sa date d'effet. Cela évite d'attribuer au propriétaire actuel tous les loyers de l'année avant son acquisition.

L'entrée d'un membre dans la famille ne crée pas un rendement : elle élargit le périmètre. Sa sortie et les transferts vers des tiers sont également identifiés. Une donation entre membres peut modifier les vues individuelles sans variation familiale à valeurs et changes constants.

### 8.2 Pont de variation reproductible

Pour expliquer `N(D1) − N(D0)`, retenir un ordre conventionnel fixe : (1) changements de périmètre familial et de méthode, (2) flux et transferts externes, (3) variation des prix/estimations et intérêts ou revenus courus selon les droits applicables, (4) variation de change. Mesurer chaque étape comme différence de deux recalculs en ne modifiant qu'une catégorie ; l'effet croisé prix/change est ainsi inclus dans l'étape change. Documenter les événements et taux figés de chaque contrefactuel. Si un taux ou une estimation de référence manque, afficher un résidu non expliqué, jamais le rebaptiser performance.

Un rendement d'investissement sophistiqué (TWR/IRR) est hors V1 tant que flux, dates et frais ne permettent pas un calcul fiable. La simple variation de net n'est pas un taux de rendement.

## 9. Taux de change manuels

### 9.1 Convention

Un taux `R(A→B,D)` signifie le nombre d'unités B pour une unité A. La saisie privilégie `devise étrangère → EUR`. Exemple : `1 TND = 0.300000 EUR`. EUR→EUR vaut exactement 1 et ne requiert pas de cours enregistré.

Chaque observation stocke paire, taux strictement positif, date d'application, date de saisie, source facultative et notes. Une correction à date identique crée une révision ; pas deux valeurs concurrentes sans priorité. Les dates d'application sont des dates civiles, sans horaire de marché inventé.

### 9.2 Résolution déterministe à D,K

1. Retenir pour chaque paire la dernière observation de date d'application `≤ D`, connue à `K`.
2. Chercher la paire directe ; à défaut, l'inverse de la paire opposée ; à défaut, un chemin à un seul pivot autorisé, EUR en priorité pour les conversions d'affichage étrangères. Pour une consolidation vers EUR sans paire directe/inverse, choisir parmi USD, CHF, TND, AED dans cet ordre explicite, sans chercher arbitrairement un chemin de longueur non bornée.
3. Calculer exactement `R(B→A)=1/R(A→B)` et `R(A→C)=R(A→B)×R(B→C)`. Pour deux devises cotées en EUR : `R(A→B)=R(A→EUR)/R(B→EUR)`.
4. Conserver tous les identifiants et dates du chemin ; l'ancienneté affichée d'un croisement est celle de son cours le plus ancien. Si plusieurs sources possibles divergent, appliquer cette priorité et montrer une alerte d'incohérence plutôt que moyenner des cours.

L'absence d'un chemin autorisé rend la conversion impossible. Aucune parité supposée pour AED/USD, aucun taux égal à un, aucun taux futur. Les seuils d'alerte d'ancienneté sont paramétrables, avec une proposition initiale de 30 jours pour le change et 365 jours pour une estimation immobilière ; ils ne changent pas le montant et restent visibles dans le rapport. Pour de très vieux cours, l'utilisateur peut choisir un mode strict rendant la conversion non exploitable au lieu d'une estimation signalée.

### 9.3 Dates selon la nature du calcul

- Un stock à `D` utilise le cours applicable à `D`, même si l'estimation immobilière est plus ancienne ; les deux dates sont affichées.
- Un flux historique utilise le cours applicable à sa date économique pour les rapports de flux.
- Une plus-value en devise d'origine vaut valeur de cession/estimation moins coût historique dans cette même devise. Sa composante EUR distingue le coût converti à la date d'acquisition et la valeur convertie à la date d'observation ; elle ne se confond pas avec une simple conversion du gain au taux courant.
- Une conversion bancaire réellement exécutée utilise ses montants débités/crédités et son taux effectif, pas un cours manuel de valorisation remplaçant le prix réellement payé.

Exemple : 1 000 TND au 30 juin, cours saisi du 1er juin `0.30 EUR/TND`, cours du 1er juillet `0.31`. Valorisation du 30 juin = 300 EUR, cours du 1er juin identifié. Sans cours antérieur ni chemin disponible, résultat inconnu ; 310 EUR serait un usage interdit du futur. Avec `CHF→EUR=1.04` et `TND→EUR=0.30`, `TND→CHF=15/52` exactement, soit environ `0.288461538…`.

## 10. Trésorerie et opérations internes

Chaque compte ou caisse est un actif spécialisé avec une devise de tenue et un point d'ouverture. Le solde vaut ouverture + somme des `treasury_entries` signées applicables. Le pays, la banque et la devise ne déterminent jamais le propriétaire : une société tunisienne peut détenir un compte EUR en Suisse.

Un transfert est une seule `ledger_transaction` avec plusieurs écritures liées ; même devise : débit et crédit se compensent hors frais. Aucune « entrée de revenu » supplémentaire n'est créée. Une conversion multidevise conserve les deux montants exacts, taux effectif, frais et dates. Pour une conversion de 1 000 EUR en 1 040 CHF avec 5 EUR de frais, les écritures sont `−1 000 EUR`, `+1 040 CHF` et `−5 EUR` ; le taux brut effectif vaut 1.04 CHF/EUR et les frais ne sont pas débités une deuxième fois.

Une contribution de 1 000 EUR entre deux comptes entièrement familiaux ne modifie pas le net familial avant frais. Entre une personne familiale et une société détenue à 60 %, un apport de capital ne peut pas être traité comme un simple transfert neutre : il modifie également les droits ou la valeur des parts et bénéficie potentiellement aux tiers. Enregistrer sa nature juridique, les contreparties et droits, puis calculer son effet réel.

Une correction de caisse est un mouvement justifié et audité, pas l'écrasement du solde. Le rapprochement bancaire conserve date, solde de relevé, solde calculé et écart ; il ne crée pas une écriture d'ajustement sans action explicite. Une distribution de dividende est distincte d'un indicateur de résultat de société, avec contrepartie de trésorerie et date réelle ; un résultat conservé n'est pas une liquidité personnelle reçue.

## 11. Immobilier et locations

Le bien et ses unités locatives sont distincts : les unités V1 sont techniques et ne sont pas des actifs consolidables indépendants. Seule la valeur du bien entre dans le patrimoine. Pour des lots acquis et cessibles séparément, créer des biens distincts sans ajouter un second actif « immeuble » couvrant les mêmes lots. Le statut vendu accompagne la clôture des droits par l'opération de cession ; modifier le seul libellé de statut ne suffit pas à clôturer les droits. Le statut vacant n'annule pas sa valeur.

Le coût total d'acquisition est prix + frais d'acquisition ; travaux immobilisés et frais de financement restent des catégories explicites, pas des coûts ajoutés arbitrairement. La plus-value latente est valeur estimée moins coût de revient retenu, avec méthode documentée. La valeur nette avant cession est estimation moins dette effectivement affectée économiquement, sans frais de vente ni fiscalité présumés.

`rent_dues` représente les échéances contractuelles, avec distinction loyer, charges récupérables et autres postes. Une révision de bail crée des termes datés, ne réécrit pas les paiements historiques. `rent_allocations` permet N paiements ↔ N échéances, avec montants, devise et date ; les sommes allouées ne peuvent dépasser le disponible du paiement ni le dû résiduel, sauf avoir/avance explicite.

La V1 distingue :

- Loyer contractuel annualisé : somme des loyers courants hors charges × nombre de périodes annuelles, avec affichage de l'hypothèse pour fréquences irrégulières.
- Loyer exigible sur `[d0,d1)` : somme des échéances arrivées à échéance dans la période, nettes d'avoirs valides.
- Loyer encaissé : fraction des paiements affectée au poste loyer, à la date de réception ; une avance non allouée n'est pas une réduction tacite d'une échéance future.
- Impayé à D : exigible jusqu'à D moins allocations reçues jusqu'à D et avoirs ; retards calculés à partir des dates d'exigibilité, sans effacer un retard parce que le paiement est arrivé plus tard.
- Charges d'exploitation : dépenses qualifiées, avec séparation des travaux en capital, intérêts, principal et dépôts.
- Revenu net d'exploitation : loyers économiques hors charges + charges refacturées reconnues sur la période − dépenses brutes d'exploitation de la même période ; équivalent à loyers hors charges moins charges nettes non récupérées. Dépôts, travaux en capital et financement sont exclus. Afficher séparément la version de trésorerie si elle est calculée sur encaissements/décaissements.
- Rendement brut : loyer contractuel annualisé / valeur estimée de référence ; rendement net avant financement : revenu net d'exploitation annualisé / même base. Préciser période, base et caractère annualisé ; dénominateur nul/inconnu → indicateur non calculable.
- Cash-flow après financement : encaissements locatifs − charges réellement décaissées − intérêts − principal − frais de financement − travaux décaissés ; dépôts exclus des revenus et suivis séparément.
- Taux d'occupation physique : jours-unités effectivement occupés / jours-unités disponibles dans la période ; exclure et afficher les unités hors suivi/en travaux déclarées indisponibles. L'indicateur n'est pas un taux de recouvrement.

Exemple : échéance de loyer 1 000 EUR due le 5 mars, réception 600 le 5 mars puis 400 le 20 avril. Au 31 mars : exigible 1 000, encaissé 600, impayé 400. Au 30 avril : impayé 0 mais retard historique conservé. Si la banque contient déjà les 600 EUR, le paiement n'ajoute pas 600 EUR au patrimoine en plus de la banque.

Les dépôts de garantie entrent en trésorerie mais créent une obligation de restitution distincte ; leur réception n'est pas un revenu. Une éventuelle compensation avec un impayé doit être une décision justifiée, pas une allocation automatique.

Pour le bilan patrimonial V1, les loyers exigibles non encaissés sont une créance détaillée dérivée des échéances, avec dépréciation explicite si nécessaire. Son titulaire est le bénéficiaire du revenu lors de la période courue, avec sources de droits historisées dans `rent_receivable_rights` ; une cession de l'immeuble ne transmet pas tacitement les arriérés au nouveau propriétaire. Un transfert de créance exige un événement explicite. Le paiement éteint cette créance au moment où la trésorerie entre, sans créer de second revenu patrimonial. Une avance locataire ou un dépôt crée un passif, puis son affectation l'éteint. La valeur immobilière saisie doit être hors ces créances et dettes locatives ; sinon déclarer la couverture pour éviter de les ajouter deux fois. Une créance déjà représentée par un autre contrat est exclue en tant que doublon identifiable.

## 12. Métaux précieux

Les lots restent des actifs physiques identifiables, avec quantité de pièces, poids unitaire, unité et pureté. Les conversions sont définies exactement : `1 kg = 1 000 g` ; `1 once troy = 31.1034768 g`. Une once ordinaire n'est pas admise comme synonyme. La pureté est une fraction `[0,1]`, pas un pourcentage interprété au hasard.

`poids_fin_g = quantité_restante × poids_unitaire_g × pureté`.

`valeur_intrinsèque = poids_fin_g / poids_unité_cotation_g × cours_unitaire`.

La prime est soit un montant explicite par unité physique du lot, multiplié par la quantité restante, soit un ratio de valeur intrinsèque, jamais les deux sans explication. La valeur estimée de revente vaut intrinsèque + prime − décote/frais explicitement saisis ; afficher les composantes. Les cours sont datés avec la même règle de dernier cours antérieur, conversion de devise et ancienneté que les autres valeurs.

Exemple : 10 pièces de 1 once troy, pureté 0.900 → `279.9312912 g` de métal fin. À 100 EUR/g, valeur intrinsèque `27 993.12912 EUR`, affichée `27 993.13 EUR`. Vendre 2 pièces réduit quantité à 8 et poids fin à `223.94503296 g`, sans perdre l'historique.

Méthode V1 de coût des lots : lot identifié ; vente partielle d'un lot homogène alloue son coût exact au prorata de la quantité vendue. Pas de FIFO global caché. Un transfert de lieu ne crée ni revenu ni cession ; un transfert de propriétaire ferme/ouvre les droits à la même date sans multiplier le stock. Une vente enregistre sortie du métal et entrée de trésorerie/une créance, avec frais ; le produit de vente ne se rajoute pas à la valeur de métal conservée.

## 13. Indicateurs des sociétés et contrôles de cohérence

Les indicateurs périodiques (chiffre d'affaires, résultat, trésorerie indicative, créances et dettes de pilotage) sont des observations analytiques. Ils ne créent ni banque, ni créance, ni dette supplémentaire dans le bilan. Une donnée utilisée comme remplacement d'un détail manquant nécessite une méthode de valorisation explicite, une couverture et une date ; un bouton de « synchronisation » implicite est proscrit.

Chaque clôture de calcul vérifie : sommes de droits, dates, absence de cycle, couverture des sociétés, propriétaires connus, cours et estimations disponibles, signes et quantités, identité des contreparties, absence d'actifs/contrats dupliqués, encours non négatifs sauf trop-payé classé séparément. Le rapport expose les anomalies ; il ne corrige pas silencieusement les données sources.

Les hypothèses sensibles de cette proposition — droits proportionnels ordinaires, consolidation proportionnelle, valeur globale des capitaux propres et non d'entreprise, créances locatives courues, unités/périodes et arrondis — devront être figées dans les décisions d'architecture et couvertes par des tests avant les tableaux de bord financiers. Les règles fiscales et les garanties de confidentialité entre membres restent hors périmètre V1.
