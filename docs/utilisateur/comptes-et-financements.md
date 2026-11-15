# Comptes, financements et bilan — utilisation de la phase 6

La phase 7 complète ce guide avec les [métaux, observations des sociétés et dividendes](metaux-et-societes.md). Les limites signalées ici comme futures pour ces modules décrivent l’état de la phase 6.

Guide du **11 novembre 2026, Europe/Paris**. Cette phase ajoute les comptes et caisses, les mouvements réellement rapprochés, les financements et le bilan financier du périmètre. Les exemples ci-dessous illustrent des saisies ; ils ne constituent pas des données ajoutées au coffre.

La date économique **D**, les connaissances **K** et le contexte familial ou individuel restent communs à l’application. Un mouvement postérieur à D n’affecte pas le solde observé. Une correction connue après K n’est pas utilisée dans cette ancienne vue. Toute sélection d’une séquence K fixe, y compris `0`, rend les formulaires historiques non modifiables. Les conventions de dates et de change sont décrites dans [Devises et historique](devises-et-historique.md).

## Créer un compte ou une caisse et confirmer son ouverture

Dans **Comptes bancaires → Comptes**, choisir **Ajouter un compte bancaire**. Dans **Espèces et devises**, choisir **Caisses physiques**, puis **Ajouter une caisse**. L’onglet **Taux et conversions** reste consacré aux cours de valorisation.

Rattacher la fiche à un actif existant dans **Patrimoine**. Au besoin, utiliser **Gérer les actifs et leurs propriétaires** pour créer cet actif et renseigner ses droits. La fiche de trésorerie décrit notamment la devise, la date de début du suivi, le titulaire juridique, le pays et les références bancaires ou le lieu de conservation des espèces.

Après création, l’actif rattaché, la devise et le début du suivi sont fixes. Leur changement dans le formulaire de correction est refusé ; une correction descriptive ne réinterprète pas les anciennes lignes dans une autre devise ou un autre actif.

Le titulaire juridique du compte est distinct des droits économiques de l’actif. Nommer une personne titulaire ne lui attribue pas implicitement `100 %` des fonds. Les droits économiques, directs ou au travers d’une société, se définissent dans le registre commun. Le bilan utilise le solde du journal pour cet actif ; une estimation générique ne constitue pas une deuxième valeur à additionner au même compte.

Créer la fiche ne crée aucun solde. Choisir ensuite **Enregistrer l’ouverture**, saisir le stock confirmé à la date de début du suivi et son motif, puis enregistrer. Une ouverture explicitement nulle se saisit `0`. Une case vide signifie absence de confirmation. L’ouverture est unique pour le compte et se corrige par une nouvelle révision si nécessaire.

Sans ouverture, même un journal contenant des mouvements ne donne pas un solde complet. Par exemple, connaître un crédit de `500 EUR` ne prouve pas que le compte contient `500 EUR` : le stock antérieur manque. Avant la date de début du suivi, le solde reste également inconnu. Une ouverture est un stock ; elle ne peut pas être rapprochée comme encaissement d’un loyer, d’un dépôt ou d’un prêt.

## Enregistrer les opérations, transferts et conversions exécutées

Dans **Journal et transferts**, choisir **Ajouter une opération** et renseigner sa nature, sa date, son libellé et ses lignes. Chaque ligne désigne un compte ou une caisse, un montant signé et un rôle : principal, frais, ouverture ou ajustement. Le crédit d’un compte est positif, son débit négatif. Les montants sont transmis comme textes décimaux et calculés exactement ; les fractions conservées évitent un arrondi intermédiaire binaire.

Un transfert entre deux comptes de même devise utilise deux lignes principales opposées dans une seule opération. Il diminue un compte et augmente l’autre sans créer de revenu. Les frais éventuels occupent des lignes distinctes, négatives. **Ajouter une ligne au journal** permet de conserver toutes les jambes de l’opération ensemble.

Une conversion exécutée utilise les deux montants réellement débités et crédités, dans leurs comptes et devises respectifs. Exemple :

| Ligne | Montant | Rôle |
|---|---:|---|
| Compte EUR, montant échangé | `−100 EUR` | Principal |
| Compte TND, montant reçu | `+330 TND` | Principal |
| Compte EUR, commission | `−2 EUR` | Frais |

Le compte EUR diminue de `102 EUR`, le compte TND augmente de `330 TND`. Le taux exécuté exact est `330 / 100 = 33/10 TND` pour `1 EUR`. Les `2 EUR` de frais restent séparés et ne sont pas comptés une deuxième fois. Ce taux exécuté ne remplace pas automatiquement un cours manuel de valorisation.

**Calculer le solde** présente les **Conversions réellement exécutées** et, séparément, les **Sources et taux de valorisation** employés pour exprimer le solde en EUR à D. Un cours absent laisse la conversion EUR non calculable même si le solde dans sa devise est connu. Une conversion bancaire ne résulte jamais d’une parité supposée.

Une caisse physique ne peut pas présenter de solde connu négatif aux dates contrôlées. Un compte bancaire négatif constitue un découvert présenté en passif. Ne pas créer en plus un contrat de prêt représentant ce même découvert : cela compterait deux fois l’obligation.

## Comparer un relevé au journal

Dans **Relevés et soldes**, choisir **Ajouter un relevé**. Indiquer le compte, la date du relevé, son solde et sa référence, puis ajouter éventuellement un document justificatif. Le formulaire actuel exige la référence même lorsqu’un document est choisi ; l’API accepte aussi un document identifié sans référence textuelle.

Sélectionner le compte dans **Contrôler un solde**, puis **Calculer le solde**. Le calcul tient compte du stock d’ouverture et des mouvements retenus ; le contrôle expose le solde et ses éventuels avertissements. Le tableau **Écarts avec les relevés** compare chaque observation au journal applicable à cette date :

`écart = solde du relevé − solde calculé`

Pour un relevé de `100 EUR` et un journal de `110 EUR`, l’écart est `−10 EUR`. Enregistrer le relevé ne crée aucune écriture de régularisation et ne remplace pas le solde calculé. Rechercher le mouvement manquant ou la source erronée, puis saisir une correction motivée. Si l’ouverture manque, le solde calculé et l’écart restent inconnus.

## Relier les sources aux mouvements existants

Un paiement locatif, une dépense, un mouvement de financement ou un dépôt décrit une source économique. Sa présence ne suffit pas à prouver son mouvement dans un compte. Un lien de trésorerie identifie la révision exacte de la source, la révision exacte de l’opération du journal et la ligne concernée.

Si la source et la ligne bancaire existent déjà, utiliser **Rapprocher une source → Rapprocher une source financière**. Choisir la source, l’opération et la ligne précises, puis le montant signé à rapprocher. Pour un financement, préciser aussi la face du contrat et la composante : principal, intérêts ou frais. Ce rapprochement ne crée aucun crédit ni débit supplémentaire.

Le moteur vérifie notamment date, devise, signe, bénéficiaire et capacités encore disponibles. Une ligne peut couvrir plusieurs sources compatibles dans la limite de son montant. Ainsi, rapprocher un loyer de `400 EUR` d’un crédit bancaire de `1 000 EUR` conserve le crédit bancaire de `1 000 EUR` ; le lien de `400 EUR` ne réduit pas le journal à la seule source rapprochée.

Les ouvertures, dépréciations et autres ajustements sans trésorerie ne servent pas de flux à rapprocher. Une source déjà liée ne doit pas être enregistrée une deuxième fois. En cas de refus pour conflit ou dépendance, recharger les sources et vérifier leurs références avant de reprendre la correction.

Les paiements et dépenses saisis avant cette phase conservent leurs identités et leur histoire. La migration ne leur invente ni compte, ni ouverture, ni ligne bancaire. Les rapprocher explicitement avec une ligne existante ou documenter leur absence de rapprochement ; une ancienne liste de paiements ne constitue pas à elle seule un solde bancaire complet.

## Enregistrer ensemble la source et sa trésorerie

Dans **Loyers et dépôts**, les boutons **Enregistrer un loyer en trésorerie**, **Enregistrer une dépense payée**, **Enregistrer un mouvement de dépôt** et **Enregistrer un financement en trésorerie** proposent une saisie liée. Le sélecteur **Source existante à rapprocher ou corriger (facultatif)** permet de reprendre une source déjà enregistrée au lieu d’en créer une autre.

Le **Traitement du journal** propose trois parcours :

| Choix | Utilisation |
|---|---|
| Créer une opération avec la source | Le mouvement bancaire n’est pas encore enregistré. |
| Rapprocher une ligne déjà enregistrée | Le journal contient déjà le mouvement réel. Aucun doublon bancaire n’est créé. |
| Corriger la ligne liée avec la source | La source et sa ligne doivent être corrigées ensemble, avec conservation de leurs versions antérieures. |

Pour une opération comportant plusieurs jambes ou des frais, préparer le journal complet puis choisir sa ligne existante. La saisie simplifiée d’une nouvelle opération ne déduit pas une conversion multidevise. Lors d’une correction, un montant bancaire laissé vide conserve le montant de la ligne existante ; il ne le remplace pas par le montant partiel de la source.

**Enregistrer la source et la trésorerie ensemble** écrit la source, ses affectations éventuelles et ses liens dans une seule transaction contrôlée. La demande utilise la séquence globale observée et les révisions attendues. Un conflit ou une validation refusée annule l’ensemble de cette demande ; il ne laisse pas une source confirmée avec une écriture bancaire manquante. Les corrections et leurs motifs restent consultables dans l’historique.

### Loyers, affectations et avances

Renseigner le bail, la date, le montant réellement reçu, la devise et le bénéficiaire déclaré. **Ajouter une affectation au loyer** relie une part du paiement à une échéance, sur le loyer ou les charges. Plusieurs paiements peuvent régler une échéance ; un paiement peut couvrir plusieurs échéances. Les limites par paiement, échéance et composante restent contrôlées.

Une réception de `1 000 EUR` dont `700 EUR` sont affectés laisse une avance de `300 EUR`. Cette avance reste une obligation visible du bénéficiaire envers le locataire ; elle ne devient pas automatiquement un revenu acquis ou un règlement du prochain loyer. Le paiement réduit la créance et augmente la trésorerie pour les parts effectivement affectées et rapprochées, sans ajouter ces deux représentations comme deux enrichissements.

Un remboursement est négatif et référence la réception initiale. Si la somme a déjà été affectée, reprendre explicitement les affectations concernées par des montants négatifs. Paiement, reprises et mouvement bancaire sont enregistrés ensemble. Un remboursement sans reprise ne peut porter que sur le disponible non affecté.

### Dépenses payées

La dépense indique un montant positif et sa qualification économique. Le débit du compte est négatif. La date économique et la date de paiement sont distinctes : une charge courue et son décaissement ne relèvent pas nécessairement de la même période. Choisir une ligne déjà enregistrée lorsque le débit figure déjà au journal.

### Dépôts de garantie

Le dépôt requis dans les conditions du bail décrit le contrat. Seuls les mouvements de dépôt décrivent les sommes effectivement reçues, rendues ou utilisées. Choisir le bail, la nature, le bénéficiaire, la date et la devise, puis renseigner les notes justificatives et le motif.

| Mouvement | Montant signé | Trésorerie et effet |
|---|---:|---|
| Réception | Positif | Crédit lié ; augmentation de l’obligation de restitution. Revenu : `0`. |
| Restitution | Négatif | Débit lié ; diminution de l’obligation. Aucune charge d’exploitation créée. |
| Application à une échéance | Négatif | Échéance et composante loyer/charges explicites ; diminution du dû et du dépôt disponible, sans flux bancaire. |
| Ajustement justifié | Signé selon la correction | Correction documentée du suivi du dépôt, sans mouvement bancaire créé. |

Exemple : recevoir `2 000 EUR` ajoute `2 000 EUR` à la banque et `2 000 EUR` à l’obligation de restitution. Le revenu et l’effet net de cette réception sont nuls. Appliquer ensuite `300 EUR` à une échéance réduit cette obligation et l’impayé de `300 EUR`, sans nouvel encaissement. Restituer `500 EUR` diminue la banque et l’obligation du même montant.

Pour l’application, choisir **Appliquer le dépôt sans mouvement bancaire**. Pour un ajustement, choisir **Ajuster le dépôt sans mouvement bancaire**. Une compensation ne doit pas être ressaisie comme paiement reçu. **Dépôts effectivement enregistrés → Corriger le dépôt** permet de reprendre une source existante et ses liens lorsque le mouvement comporte de la trésorerie.

## Décrire un financement et ses deux faces

Dans **Dettes et financements → Contrats et parties**, choisir **Créer un contrat de financement**. Indiquer son nom, sa devise et le début du suivi. Un seul contrat représente la créance du prêteur et la dette de l’emprunteur. Éviter deux contrats indépendants pour ces deux faces.

La devise et le début du suivi du contrat sont fixes après création. Une correction de nom ou de notes ne permet pas de les modifier ni de réinterpréter les mouvements existants.

Choisir **Définir les parties du financement** pour renseigner les conditions datées et, de chaque côté, les entités, droits juridiques, droits économiques et portée juridique de l’obligation. Les fractions sont explicites : `1/1`, `1/2`, etc. Laisser les deux cases d’une fraction vides conserve un droit inconnu. La solidarité juridique ne signifie pas que chaque débiteur supporte économiquement `100 %` du prêt.

La répartition du prêt est indépendante de celle du bien financé. Pour un bien de `400 000 EUR` détenu à parts égales par A et B et une dette de `100 000 EUR` économiquement supportée par A seul, les positions nettes sont `100 000 EUR` pour A et `200 000 EUR` pour B, soit `300 000 EUR` ensemble, si les autres sources et couvertures nécessaires sont établies. La dette de A n’est pas divisée entre A et B parce que le bien est indivis.

Déclarer la connaissance du principal dans les conditions, puis enregistrer son **Ouverture historique** explicitement, même à zéro. Cette ouverture utilise exactement la date de début du suivi, avec intérêts et frais nuls. Un contrat créé sans ouverture ou dont le suivi est incomplet ne prouve pas un encours nul. Le taux indicatif et sa convention décrivent le contrat ; ils ne créent pas automatiquement des intérêts ni un échéancier.

## Séparer principal, intérêts, frais et prévisions

Dans **Mouvements et échéances**, **Enregistrer un mouvement de financement** permet de conserver un fait du contrat avec un motif explicite d’absence de rapprochement bancaire. Une ouverture et un tirage augmentent le principal selon leur nature ; un remboursement le diminue. Les intérêts et les frais sont saisis séparément. Un ajustement du principal ou une remise juridique se documente comme tel ; ils ne fabriquent pas un mouvement bancaire.

Pour un mouvement réellement encaissé ou payé, utiliser **Rapprocher les mouvements dans les comptes**, puis la saisie liée du financement dans **Loyers et dépôts**. Sélectionner une source existante si le mouvement du prêt est déjà saisi. Renseigner le montant bancaire total signé et les composantes rapprochées, avec leur face créancier ou débiteur et leur ligne précise. Les fractions économiques du contrat ne sont pas déduites du compte choisi.

Exemple côté emprunteur : principal restant initial `100 000 EUR`, remboursement de principal `−1 000`, intérêts payés `200`, frais payés `10`, débit bancaire total `−1 210`. L’encours devient `99 000 EUR`. Les liens de la face débiteur portent respectivement `−1 000`, `−200` et `−10` ; les intérêts et frais ne diminuent pas le principal. Pour un prêt entre deux comptes suivis, préparer les deux jambes du journal et lier les composantes de chaque face aux lignes correspondantes.

**Ajouter une échéance de financement** enregistre une prévision de principal, intérêts et frais. Elle n’est ni remboursement ni débit. **Affecter un remboursement prévu** rapproche un remboursement réel d’une échéance sans créer un nouveau mouvement. Les retards et régularisations restent historiques.

**Déprécier ou reprendre une créance** ajuste sa valeur portée pour le créancier désigné, avec une variation signée et une source. Une dépréciation ne remet pas la dette certaine de l’emprunteur. Une extinction juridique utilise une remise du principal distincte ; ne pas remplacer cette distinction par une simple baisse d’estimation.

Dans **Contrôler un financement**, **Calculer l’encours du financement** analyse le contrat entier à D/K : principal, parties, valeurs portées, échéancier, liens aux biens, garanties, sources et conversion. Le bilan familial ou individuel applique ensuite ses propres poids et éliminations. Archiver le contrat conserve son histoire et n’éteint pas son encours.

## Biens financés, garanties et valeurs globales de sociétés

Dans **Biens et garanties**, **Lier un financement à un bien** distingue financement analytique et sûreté/collatéral. La part analytique du principal est facultative : une part inconnue reste vide. Ce lien ne désigne pas l’emprunteur et n’attribue jamais la dette aux copropriétaires du bien.

**Enregistrer une garantie conditionnelle** conserve le garant, sa période et sa limite éventuelle. Cette exposition figure hors des dettes certaines. Un appel de garantie exige une opération explicite et justifiée ; enregistrer la caution ne diminue pas automatiquement le patrimoine comme une dette appelée.

Une société valorisée par sa valeur globale nette peut déjà comprendre une créance ou une dette. **Déclarer une couverture de société** précise, pour un contrat, une société, une face et une période, si cette face est **déjà comprise** ou **explicitement exclue** de cette valeur globale, avec une source vérifiable. L’absence de déclaration reste inconnue ; elle ne signifie pas « exclue ». Ce contrôle évite d’ajouter une créance ou de retrancher une dette déjà comprise dans la participation.

Cette couverture de valeur globale est distincte de la déclaration d’inventaire complet du bilan. L’une n’établit pas l’autre. Une exposition passant par une valeur globale sans couverture précise peut laisser le bilan partiel et empêcher une élimination réciproque fiable.

## Définir les créanciers des loyers et lire le bilan

Dans **Patrimoine**, **Bilan du périmètre → Calculer le bilan financier** rassemble les positions financières à D/K dans le contexte familial ou individuel choisi. **Contrôle des attributions** reste le contrôle du registre des actifs ; ce contrôle historique ne constitue pas, à lui seul, le bilan intégrant banques, créances et passifs.

### Droits sur les créances locatives

Ouvrir **Déclarations de couverture et droits sur les loyers → Définir les créanciers d’une échéance**. Indiquer les créanciers économiques, leurs fractions connues et la période d’application. Aucun créancier à `100 %` n’est inventé à partir du propriétaire actuel du bien ou du bénéficiaire d’un encaissement.

Ces droits décrivent la créance à sa naissance ou après une cession explicite. Vendre le bien ne transfère pas automatiquement ses anciens impayés au nouvel acquéreur. Les anciennes échéances sans créanciers suffisamment définis peuvent donc rendre le bilan incomplet même si le rapport locatif affiche un impayé analytique.

Le champ **Même créance déjà représentée par un contrat (facultatif)** sert uniquement lorsque le contrat de financement désigné représente réellement cette même créance. Le lien écarte sa deuxième représentation locative dans le bilan ; il ne cède pas le droit à un autre créancier.

**Ajuster la valeur d’une créance locative** permet une dépréciation négative ou une reprise positive pour un créancier, une échéance et une date, avec justificatif ou méthode. Cet ajustement de valeur n’efface pas la dette juridique du locataire et ne crée aucun encaissement.

### Couverture du bilan et montants inconnus

**Déclarer la couverture du bilan** concerne tout l’espace familial, indépendamment du membre actuellement consulté. Sur sa période d’application, la déclaration distingue trois inventaires : actifs ; financements et engagements certains ; créances, avances et dépôts locatifs. Cocher uniquement les inventaires effectivement vérifiés. Une table vide ne prouve pas qu’il n’existe aucune dette ou créance.

Une déclaration ne remplace pas les comptes manquants, les ouvertures, les parties, les fractions, les cours ou les créanciers non saisis. Le rapport distingue la couverture déclarée et la cohérence des sources nécessaires au calcul. Il ne présente un net complet que lorsque ces deux conditions sont satisfaites pour le périmètre observé.

| Affichage | Lecture |
|---|---|
| Montant `0` | Résultat connu explicitement nul, avec sources et couverture suffisantes. |
| **Non calculable** | Source ou couverture insuffisante ; la valeur absente de l’API (`None`/`null`) n’est pas zéro. |
| **Sous-totaux connus** | Positions calculables et leur différence ; cette différence n’est pas un patrimoine net complet. |
| **Couverture déclarée et sources cohérentes** | Rapport complet selon les faits et déclarations applicables à D/K. |

Le tableau **Positions et sources du bilan** expose les actifs, les soldes bancaires et caisses, les découverts, les créances et dettes de financement ou de location, ainsi que les avances et obligations de dépôt. Chaque position conserve ses droits, références et conversions consultables. Un compte négatif est présenté au passif ; une créance et son règlement bancaire ne sont pas ajoutés comme deux stocks indépendants.

Les **Éliminations internes du périmètre** montrent la créance et la dette avant élimination, leur part réciproque et le reliquat. Un prêt entre deux personnes entièrement comprises dans la famille s’élimine réciproquement dans ce contexte, tout en restant visible pour chaque personne. Les poids partiels sont conservés : pour un nominal de `10 000 EUR`, `7 500 EUR` de créance attribuable et `4 000 EUR` de dette attribuable donnent, dans un cas sans autre ajustement, `4 000 EUR` éliminés de chaque côté et `3 500 EUR` de créance résiduelle. Un lien de collatéral ne justifie aucune élimination à lui seul.

Les avertissements et **Références du calcul financier** indiquent les sources retenues et les limites restantes. La déclaration de couverture doit être réexaminée lorsque l’inventaire ou les faits de la période changent.

## Retrouver les effets dans les contrôles immobiliers et locatifs

Le contrôle immobilier peut présenter la **Dette restante affectée** et la **Valeur nette avant frais et fiscalité de cession** lorsque la couverture et les affectations analytiques des financements sont suffisantes. Les conversions des financements restent consultables séparément de celles de l’acquisition et de l’estimation. Une affectation à un bien reste analytique ; le bilan personnel suit les parties économiques du contrat de prêt. Aucun frais de cession ni impôt n’est ajouté par hypothèse.

Le rapport locatif distingue le revenu net d’exploitation couru et le cash-flow après financement. Ce dernier nécessite des sources de trésorerie rapprochées et une couverture suffisante ; une donnée absente reste non calculable. Le principal remboursé relève du financement, les intérêts et frais sont séparés, et un dépôt compensé réduit un impayé sans devenir un loyer reçu. Les bases du rapport immobilier et locatif figurent dans [le guide de phase 5](immobilier-et-loyers.md) ; les restrictions de ce guide sur la future trésorerie sont complétées par le présent module.

## Historique, sauvegarde et contrats d’API

Les corrections motivées créent des révisions ; elles ne réécrivent pas les connaissances déjà consultées. Annuler une source utilisée peut être refusé tant que ses dépendances n’ont pas été traitées. Paramètres, documents et sauvegardes portent toujours sur le coffre actuel, même lorsqu’une vue de calcul consulte un ancien K.

La migration `0005` amène le coffre au schéma `5` sur une copie contrôlée, sans inventer les mouvements financiers des phases antérieures. Les sauvegardes conservent comptes, écritures, rapprochements, financements, garanties, dépôts, créanciers et déclarations de couverture, avec leurs révisions. L’archive contient une base SQLite autonome, les documents et le dictionnaire du schéma. Les modalités générales restent décrites dans [Documents et portabilité](../technique/conception-documents-portabilite.md).

Pour les développements ou contrôles externes, les contrats applicatifs sont regroupés dans [TreasuryService](../../crates/application/src/treasury.rs), [DebtService](../../crates/application/src/debts.rs) et [FinancialService](../../crates/application/src/finance.rs). Les commandes liées `post_rent_payment`, `post_rental_expense`, `post_deposit` et `post_debt_movement` écrivent atomiquement leurs sources et liens. Les lectures `list_*_at` et les rapports utilisent K ; les rapports reçoivent également D. Le bilan reçoit explicitement le contexte familial ou individuel. Les montants restent des chaînes exactes et les fractions des couples d’entiers ; les consommateurs doivent conserver la distinction entre montant absent et zéro.

Les contrôles exécutés et leurs résultats sont consignés dans [la validation de phase 6](../historique/validation-phase-6.md). Ce guide décrit le périmètre de la phase 6 ; il n’annonce pas les modules des phases suivantes comme réalisés.
