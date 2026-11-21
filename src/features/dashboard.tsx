import { useEffect, useMemo, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { api, displayCurrencies, errorMessage } from '../services/api';
import type { VaultStatus } from '../services/api';
import type { DashboardFilters, DashboardReport, DashboardRequest, DashboardTotals } from '../services/dashboard-types';
import type { CompanyReport } from '../services/companies-types';
import type { RegistrySnapshot } from '../services/registry-types';
import { DashboardAllocationChart, DashboardCompanyChart, DashboardHistoryChart, DashboardMembersChart } from './dashboard-charts';
import { entityAt, RegistryNotice } from './registry-ui';
import type { ViewContext } from './registry-ui';
import './dashboard.css';

const categories = [
  ['immobilier', 'Immobilier'], ['banque', 'Banques'], ['especes', 'Espèces'], ['or', 'Or'], ['argent', 'Argent métal'],
  ['participations', 'Participations'], ['creances', 'Créances'], ['financements', 'Financements'],
  ['depots', 'Dépôts'], ['avances', 'Avances locatives'], ['decouverts', 'Découverts'], ['autres', 'Autres actifs'],
] as const;
const categoryLabel = (key: string) => categories.find(([value]) => value === key)?.[1] ?? key;
const emptyFilters = (): DashboardFilters => ({ country: null, currency: null, category: null, owner_entity_id: null, query: '' });
const frequencyLabel = (value: string) => ({ monthly: 'Mensuelle', quarterly: 'Trimestrielle', annual: 'Annuelle' })[value] ?? value;

// Amounts remain decimal strings. Number conversion belongs only to chart geometry.
function amount(value: string | null, currency = 'EUR'): string {
  if (value === null) return 'Non calculable';
  const [integer, decimals] = value.split('.');
  return `${integer.replace(/\B(?=(\d{3})+(?!\d))/g, '\u202f')}${decimals === undefined ? '' : `,${decimals}`} ${currency}`;
}

export default function DashboardPage({ registry, context, status, onNavigate }: {
  registry: RegistrySnapshot; context: ViewContext; status: VaultStatus; onNavigate: (page: string) => void;
}) {
  const [fromDate, setFromDate] = useState(`${context.date.slice(0, 4)}-01-01`);
  const [interval, setInterval] = useState<'monthly' | 'annual'>('monthly');
  const [companySelection, setCompanySelection] = useState({ id: '', personId: context.personId });
  const [filters, setFilters] = useState<DashboardFilters>(emptyFilters);
  const [loaded, setLoaded] = useState<{ key: string; report: DashboardReport } | null>(null);
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const requestId = useRef(0);
  const companyId = companySelection.personId === context.personId ? companySelection.id : '';
  const knowledge = context.knowledgeSequence ?? registry.knowledge_sequence;
  const request: DashboardRequest = useMemo(() => ({
    context: companyId ? 'company' : context.personId ? 'person' : 'family',
    entity_id: companyId || context.personId,
    date: context.date, from_date: fromDate, interval, knowledge_sequence: knowledge,
    filters: companyId ? emptyFilters() : filters,
  }), [companyId, context.personId, context.date, fromDate, interval, knowledge, filters]);
  const key = JSON.stringify([status.generation_id, request]);
  const currentKey = useRef(key);
  currentKey.current = key;
  const report = loaded?.key === key ? loaded.report : null;
  const scopeKey = JSON.stringify([status.generation_id, context.date, context.personId, knowledge, companyId, fromDate, interval]);
  const name = (id: string) => {
    const entity = registry.entities.find(item => item.id === id);
    return entity ? entityAt(registry, entity, context.date).display_name : id;
  };
  const scopeName = companyId ? name(companyId) : context.personId ? name(context.personId) : 'Patrimoine familial';
  const countries = [...new Set(['FR', 'CH', 'TN', 'AE', ...registry.assets.map(asset => asset.country), ...registry.entities.map(entity => entity.residence_country).filter((country): country is string => country !== null)])].sort();

  // Hide results synchronously through their key, before any pending effect.
  useEffect(() => {
    requestId.current += 1;
    setLoading(false); setError(''); setMessage('');
  }, [key]);
  useEffect(() => {
    setCompanySelection({ id: '', personId: context.personId });
  }, [context.personId]);
  useEffect(() => {
    if (fromDate > context.date) setFromDate(`${context.date.slice(0, 4)}-01-01`);
  }, [context.date]);
  async function calculate(event?: FormEvent) {
    event?.preventDefault();
    if (!request.from_date || request.from_date > request.date) return;
    const id = ++requestId.current;
    const requestedKey = key;
    setLoading(true); setError(''); setMessage(''); setLoaded(null);
    try {
      const result = await api.getDashboard(request);
      if (id !== requestId.current || currentKey.current !== requestedKey) return;
      if (!result || !result.request) throw new Error('Le tableau de bord n’a pas pu être chargé. Actualisez le calcul.');
      setLoaded({ key: requestedKey, report: result });
    } catch (failure) {
      if (id === requestId.current && currentKey.current === requestedKey) setError(errorMessage(failure));
    } finally {
      if (id === requestId.current && currentKey.current === requestedKey) setLoading(false);
    }
  }
  // Context and period changes recalculate automatically. Draft text filters
  // use the refresh button, avoiding an expensive history per keystroke.
  useEffect(() => { void calculate(); }, [scopeKey]);
  useEffect(() => () => { requestId.current += 1; }, []);

  function updateFilter(field: keyof DashboardFilters, value: string) {
    setFilters(current => ({ ...current, [field]: field === 'query' ? value : value || null }));
  }
  function chooseCompany(value: string) {
    setFilters(emptyFilters());
    setCompanySelection({ id: value, personId: context.personId });
  }
  async function exportReport() {
    if (!report || exporting) return;
    const source = report.request;
    const sourceKey = key;
    setExporting(true); setMessage(''); setError('');
    try {
      const destination = await api.chooseDashboardReportDestination();
      if (!destination || currentKey.current !== sourceKey) return;
      await api.exportDashboardReport(source, destination);
      if (currentKey.current === sourceKey) setMessage('Le rapport filtré a été exporté en JSON avec son contexte et ses sources.');
    } catch (failure) {
      if (currentKey.current === sourceKey) setError(errorMessage(failure));
    } finally { setExporting(false); }
  }

  return <div className="dashboard-workspace">
    <section className="dashboard-intro" aria-label="Synthèse patrimoniale">
      <div><h2>{scopeName}</h2><p>{context.personId && !companyId ? 'Part des biens et des dettes de cette personne.' : 'Votre situation au jour choisi, en euros.'}</p></div>
      <div className="dashboard-local-state"><span><span className="status-dot" aria-hidden="true" />Accessible</span></div>
    </section>
    {error && <RegistryNotice error>{error}</RegistryNotice>}
    {message && <RegistryNotice>{message}</RegistryNotice>}
    {loading && <p role="status" className="dashboard-loading">Calcul de votre situation patrimoniale…</p>}
    {!report && !loading && !error && <RegistryNotice>Les filtres ont changé. Ouvrez les options et actualisez le tableau de bord pour afficher les résultats.</RegistryNotice>}
    {!registry.entities.length && !registry.assets.length && <section className="card dashboard-empty"><h2>Construire une vue à partir de vos données</h2><p>Ajoutez un membre, un bien immobilier ou un compte. Le tableau de bord se remplit à mesure de vos saisies.</p><div className="button-row"><button type="button" className="button" onClick={() => onNavigate('family')}>Renseigner la famille</button><button type="button" className="button secondary" onClick={() => onNavigate('property')}>Ouvrir l’immobilier</button><button type="button" className="button secondary" onClick={() => onNavigate('bank')}>Ouvrir les comptes bancaires</button></div></section>}
    <div className={report ? 'dashboard-results' : 'dashboard-calculation'} role={report ? 'region' : undefined} aria-label={report ? 'Résultats du tableau de bord' : undefined} data-knowledge-sequence={report?.knowledge_sequence}>
      {report && <>
        <div className="dashboard-result-heading"><h2>{report.company ? 'Pilotage de la société' : report.filters_applied ? 'Résultats filtrés' : 'Votre situation patrimoniale'}</h2><span className="muted">Au <time dateTime={report.request.date}>{report.request.date.split('-').reverse().join('/')}</time></span></div>
        {report.filters_applied && <RegistryNotice>Les montants correspondent aux filtres choisis. Les alertes restent celles du périmètre complet.</RegistryNotice>}
        {report.totals && <Totals totals={report.totals} />}
        {!!report.warnings.length && <Alerts report={report} />}
      </>}
    <details className="card dashboard-filters dashboard-disclosure">
      <summary>Filtres, période et export du rapport</summary>
      <form onSubmit={event => { void calculate(event); }}>
        <fieldset disabled={exporting}>
          <div className="dashboard-filter-grid">
            <div><label htmlFor="dashboard-from-date">Début de la période</label><input id="dashboard-from-date" type="date" required max={context.date} value={fromDate} onChange={event => setFromDate(event.currentTarget.value)} /></div>
            <div><label htmlFor="dashboard-interval">Fréquence de l’historique</label><select id="dashboard-interval" value={interval} onChange={event => setInterval(event.currentTarget.value as 'monthly' | 'annual')}><option value="monthly">Mensuelle</option><option value="annual">Annuelle</option></select></div>
            <div><label htmlFor="dashboard-company">Vue de société</label><select id="dashboard-company" value={companyId} onChange={event => chooseCompany(event.currentTarget.value)}><option value="">Vue familiale ou individuelle</option>{registry.entities.filter(entity => entity.kind === 'company').map(entity => <option value={entity.id} key={entity.id}>{name(entity.id)}</option>)}</select></div>
          </div>
          <fieldset className="dashboard-position-filters" disabled={!!companyId}>
            <legend>Filtrer les biens et les dettes</legend>
            <div className="dashboard-filter-grid">
              <div><label htmlFor="dashboard-country">Pays</label><select id="dashboard-country" value={filters.country ?? ''} onChange={event => updateFilter('country', event.currentTarget.value)}><option value="">Tous les pays</option>{countries.map(country => <option key={country} value={country}>{country}</option>)}</select></div>
              <div><label htmlFor="dashboard-currency">Devise d’origine</label><select id="dashboard-currency" value={filters.currency ?? ''} onChange={event => updateFilter('currency', event.currentTarget.value)}><option value="">Toutes les devises</option>{displayCurrencies.map(currency => <option key={currency}>{currency}</option>)}</select></div>
              <div><label htmlFor="dashboard-category">Catégorie</label><select id="dashboard-category" value={filters.category ?? ''} onChange={event => updateFilter('category', event.currentTarget.value)}><option value="">Toutes les catégories</option>{categories.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></div>
              <div className="dashboard-owner-filter"><label htmlFor="dashboard-owner">Propriétaire associé</label><select id="dashboard-owner" aria-describedby="dashboard-owner-hint" value={filters.owner_entity_id ?? ''} onChange={event => updateFilter('owner_entity_id', event.currentTarget.value)}><option value="">Tous les propriétaires associés</option>{registry.entities.map(entity => <option key={entity.id} value={entity.id}>{name(entity.id)}</option>)}</select></div>
              <div><label htmlFor="dashboard-query">Rechercher dans le rapport</label><input id="dashboard-query" type="search" maxLength={200} value={filters.query} onChange={event => updateFilter('query', event.currentTarget.value)} placeholder="Nom, identifiant ou source" /></div>
            </div>
            <p id="dashboard-owner-hint" className="field-hint">Le filtre par propriétaire conserve le montant entier de chaque ligne du rapport. Pour calculer la part d’une personne, choisissez-la dans le périmètre de consultation.</p>
          </fieldset>
          {companyId && <p className="field-hint">La vue de société utilise ses propres données. Revenez à la vue familiale ou individuelle pour appliquer ces filtres.</p>}
          <div className="dashboard-filter-actions"><button type="submit" className="button" disabled={loading || !fromDate || fromDate > context.date}>{loading ? 'Calcul du tableau de bord…' : 'Actualiser le tableau de bord'}</button><button type="button" className="button secondary" disabled={!report || loading || exporting} onClick={() => { void exportReport(); }}>{exporting ? 'Export du rapport…' : 'Exporter le rapport filtré'}</button><span>Du {fromDate || '…'} au {context.date} · K={knowledge}</span></div>
        </fieldset>
      </form>
    </details>
    {report && <>
      {report.company ? <CompanyPanel report={report.company} fromDate={report.request.from_date} /> : <>
        <DashboardAllocationChart buckets={report.by_category} title="Répartition par catégorie" />
        <details className="card dashboard-disclosure dashboard-advanced">
          <summary>Évolution et autres répartitions</summary>
          <div className="dashboard-disclosure-content">
            <DashboardHistoryChart points={report.history} />
            <p>Variation du patrimoine net sur la période : <b>{amount(report.period_change?.net_change_eur ?? null)}</b>. {report.period_change && <>Écart des différences connues : <b>{amount(report.period_change.known_net_change_eur)}</b>. Cet écart ne mesure pas une performance.</>}</p>
            <div className="dashboard-charts-grid"><DashboardAllocationChart buckets={report.by_country} title="Répartition par pays" /><DashboardAllocationChart buckets={report.by_currency} title="Répartition par devise d’origine" /></div>
            {!!report.members.length && <DashboardMembersChart members={report.members} />}
          </div>
        </details>
        <details className="card dashboard-disclosure dashboard-ledger"><summary>Détail des biens, dettes et calculs</summary><div className="dashboard-disclosure-content">
        <Positions report={report} name={name} />
        {report.variation && <section className="card dashboard-variation"><h2>Pont des estimations du registre</h2><p>Ce pont partiel concerne les estimations historiques du registre, hors filtres des positions. Il n’explique pas la variation complète du bilan ni un rendement. Les métaux dérivés et les nouveaux stocks financiers gardent les limites indiquées par leur moteur.</p><div className="table-scroll"><table><caption>Décomposition du registre, du {report.variation.from_date} au {report.variation.to_date}</caption><thead><tr><th>Composante</th><th>Variation en EUR</th></tr></thead><tbody>{report.variation.steps.map(step => <tr key={step.code}><td>{step.label}</td><td>{amount(step.amount_eur)}</td></tr>)}<tr><td>Résidu de ce pont</td><td>{amount(report.variation.residual_eur)}</td></tr></tbody></table></div><p className="field-hint">Statut du pont : {report.variation.status}. L’export d’une vue filtrée conserve les positions sélectionnées et signale l’exclusion de ce pont sans filtre.</p></section>}
        </div></details>
      </>}
      {report.rental && <RentalSummary rental={report.rental} />}
      <details className="card dashboard-sources dashboard-disclosure"><summary>Contexte et sources du rapport</summary><div className="dashboard-disclosure-content"><p>{scopeName} · date économique {report.request.date} · connaissances K={report.knowledge_sequence} · moteur {report.engine_version}. Chaque date historique est recalculée avec ce même K.</p><details><summary>Sources du rapport ({report.source_ids.length})</summary><code className="dashboard-source-ids">{report.source_ids.join(', ') || 'Aucune source enregistrée dans ce périmètre.'}</code></details><p className="field-hint">L’export JSON conserve les filtres, positions, historique, valeurs exactes, références et avertissements de cette consultation. Avec des filtres actifs, il signale l’exclusion des sections calculées sur un périmètre sans filtre. Pour récupérer l’intégralité du coffre et ses originaux, utilisez Sauvegarde et export.</p></div></details>
    </>}
    </div>
  </div>;
}

function Totals({ totals }: { totals: DashboardTotals }) {
  return <div className="dashboard-total-grid">
    <article className="dashboard-metric" aria-label="Patrimoine brut"><span className="dashboard-metric-label">Patrimoine brut</span><strong data-value={totals.assets_eur ?? 'unknown'}>{amount(totals.assets_eur)}</strong><p>{totals.assets_eur === null ? <>Sous-total connu : <b>{amount(totals.known_assets_eur)}</b>.</> : 'Valeur des biens et droits suivis.'}</p></article>
    <article className="dashboard-metric" aria-label="Dettes et obligations"><span className="dashboard-metric-label">Dettes et obligations</span><strong data-value={totals.liabilities_eur ?? 'unknown'}>{amount(totals.liabilities_eur)}</strong><p>{totals.liabilities_eur === null ? <>Sous-total connu : <b>{amount(totals.known_liabilities_eur)}</b>.</> : 'Sommes restant dues dans cette vue.'}</p></article>
    <article className="dashboard-metric dashboard-metric-main" aria-label="Patrimoine net"><span className="dashboard-metric-label">Patrimoine net</span><strong data-value={totals.net_eur ?? 'unknown'}>{amount(totals.net_eur)}</strong><p>{totals.net_eur === null ? <>Différence connue : <b>{amount(totals.known_net_eur)}</b>. Ce sous-total n’est pas un net complet.</> : 'Patrimoine brut moins dettes et obligations.'}</p></article>
  </div>;
}

function Metric({ label, value }: { label: string; value: string | null }) {
  return <div><span>{label}</span><strong data-value={value ?? 'unknown'}>{amount(value)}</strong></div>;
}

function RentalSummary({ rental }: { rental: NonNullable<DashboardReport['rental']> }) {
  const late = rental.dues.filter(due => due.overdue_days > 0);
  return <details className="card dashboard-rentals dashboard-disclosure">
    <summary>Loyers et dépenses</summary>
    <p>Du {rental.from_date} au {rental.to_date} (fin exclue), situation au {rental.date}. Seul le filtre par pays s’applique à ces indicateurs.</p>
    <div className="dashboard-rental-grid">
      <Metric label="Loyers courus" value={rental.economic_rent_eur} />
      <Metric label="Loyers encaissés" value={rental.rent_received_eur} />
      <Metric label="Loyers restant dus" value={rental.outstanding_rent_eur} />
      <Metric label="Revenu locatif net couru (NOI)" value={rental.noi_eur} />
    </div>
    <p className="field-hint">Les loyers courus correspondent à la période d’occupation. Les loyers encaissés correspondent aux paiements reçus. Le restant dû inclut les échéances exigibles.</p>
    {late.length > 0 && <details><summary>Échéances en retard et sources</summary><ul>{late.map(due => <li key={due.due_id}>{due.due_on} · {due.overdue_days} jours · loyer restant {amount(due.outstanding_rent, due.currency)}<code className="dashboard-source-ids">{due.source_ids.join(', ')}</code></li>)}</ul></details>}
  </details>;
}

function Alerts({ report }: { report: DashboardReport }) {
  return <section className="dashboard-quality dashboard-quality-warning" aria-label="Qualité des données">
    <div><span className="dashboard-quality-icon" aria-hidden="true">!</span><div><h2>{report.warnings.length} alerte{report.warnings.length > 1 ? 's' : ''} à examiner</h2><p>Les montants connus restent visibles. Une information manquante reste inconnue.</p></div></div>
    <details><summary>Afficher toutes les alertes et leurs références</summary><ul>{report.warnings.map((warning, index) => <li key={`${warning.code}-${warning.object_id}-${index}`}><strong>{warning.message}</strong><code>{warning.code} · {warning.object_id}</code></li>)}</ul></details>
  </section>;
}

function Positions({ report, name }: { report: DashboardReport; name: (id: string) => string }) {
  return <section className="card dashboard-positions"><div className="section-title"><h2>Positions et rapprochement du bilan</h2><span className="badge">{report.positions.length} position{report.positions.length > 1 ? 's' : ''}</span></div><p>Montants attribuables au périmètre, après éliminations internes. Les engagements conditionnels sont présentés hors total.</p>{!report.positions.length ? <p>Aucune position ne correspond aux sources et filtres sélectionnés.</p> : <div className="table-scroll"><table><caption>Positions filtrées, à D={report.request.date}, connues à K={report.knowledge_sequence}</caption><thead><tr><th>Position</th><th>Type et pays</th><th>Valeur d’origine</th><th>Montant retenu EUR</th><th>Sources et éliminations</th></tr></thead><tbody>{report.positions.map((row, index) => <tr key={`${row.position.object_id}-${row.position.kind}-${row.position.entity_id}-${index}`}><td><strong>{row.position.label}</strong><small>{row.side === 'liability' ? 'Passif' : row.side === 'contingent' ? 'Engagement conditionnel hors total' : 'Actif'}</small>{!!row.owner_entity_ids.length && <small>{row.owner_entity_ids.map(name).join(', ')}</small>}</td><td>{categoryLabel(row.category)}<small>{row.country ?? 'Pays non renseigné'}</small></td><td>{amount(row.position.amount_original, row.position.currency)}</td><td><strong data-value={row.amount_eur ?? 'unknown'}>{amount(row.amount_eur)}</strong>{row.side === 'contingent' && <small>Non ajouté aux totaux</small>}</td><td><details><summary>Voir les références</summary><p>Avant élimination : {amount(row.position.amount_eur)}.<br />Réduction interne : {amount(row.eliminated_eur)}.</p><p>Quote-part : {row.position.weight ? `${row.position.weight.num}/${row.position.weight.den}` : 'Inconnue'}.</p><p>Valeur EUR exacte après élimination : {row.exact_eur ? `${row.exact_eur.num}/${row.exact_eur.den}` : 'Inconnue'}.</p><code className="dashboard-source-ids">{row.position.object_id}<br />{row.position.source_ids.join(', ')}</code></details></td></tr>)}</tbody></table></div>}</section>;
}

function CompanyPanel({ report, fromDate }: { report: CompanyReport; fromDate: string }) {
  return <div className="dashboard-company-panel">
    <section className="card"><div className="section-title"><h2>{report.company.display_name}</h2><span className="badge">{report.policy?.mode === 'detailed' ? 'Méthode détaillée' : report.policy?.mode === 'global_equity' ? 'Valeur globale nette' : 'Méthode non renseignée'}</span></div><p>Le pilotage reprend les comptes, financements, participations et observations enregistrés pour cette société. Les indicateurs déclarés ne constituent pas des positions patrimoniales supplémentaires, et aucun patrimoine net de société n’est déduit de leur somme.</p><div className="dashboard-company-counts"><div><strong>{report.accounts.length}</strong><span>comptes suivis</span></div><div><strong>{report.assets.length}</strong><span>actifs associés</span></div><div><strong>{report.subsidiaries.length}</strong><span>participations suivies</span></div><div><strong>{report.financial_periods.length}</strong><span>observations datées</span></div></div></section>
    <DashboardCompanyChart periods={report.financial_periods} />
    <section className="card"><h2>Comptes de la société</h2>{!report.accounts.length ? <p>Aucun compte suivi pour cette société à cette date.</p> : <div className="table-scroll"><table><caption>Soldes réels des comptes, sans ajout des observations indicatives</caption><thead><tr><th>Compte</th><th>Solde d’origine</th><th>Solde EUR</th><th>Sources</th></tr></thead><tbody>{report.accounts.map(item => <tr key={item.account.id}><td>{item.account.bank_name || item.account.storage_location || item.account.reference || item.account.id}<small>{item.account.currency} · {item.account.kind === 'cash' ? 'Espèces' : 'Banque'}</small></td><td>{amount(item.report.balance_original, item.report.currency)}</td><td>{amount(item.report.balance_eur)}</td><td><details><summary>Sources du compte</summary><code className="dashboard-source-ids">{item.report.source_ids.join(', ')}</code></details></td></tr>)}</tbody></table></div>}</section>
    <section className="card"><h2>Financements de la société</h2>{!report.debt_positions.length ? <p>Aucun financement documenté dans ce contrôle. Cette absence ne confirme pas un inventaire complet des dettes.</p> : <div className="table-scroll"><table><caption>Faces des contrats de financement de la société</caption><thead><tr><th>Contrat</th><th>Face</th><th>Solde d’origine</th><th>Montant EUR</th><th>Sources</th></tr></thead><tbody>{report.debt_positions.map((position, index) => <tr key={`${position.debt_id}-${position.side}-${index}`}><td>{position.name}</td><td>{position.side === 'creditor' ? 'Créance' : 'Dette'}</td><td>{amount(position.amount_original, position.currency)}</td><td>{amount(position.amount_eur)}</td><td><details><summary>Sources du financement</summary><code className="dashboard-source-ids">{position.source_ids.join(', ')}</code></details></td></tr>)}</tbody></table></div>}</section>
    <section className="card"><h2>Observations de pilotage</h2><p>Les périodes entières dont la fin est postérieure au {fromDate} et au plus tard le {report.date} sont présentées, sans prorata. Les fréquences et devises restent distinctes. Les comptes, actifs et financements sont observés à la date {report.date}.</p>{!report.financial_periods.length ? <p>Aucune observation connue terminée dans cette fenêtre.</p> : <div className="table-scroll"><table><caption>Observations par période et fréquence, hors bilan patrimonial</caption><thead><tr><th>Période</th><th>Chiffre d’affaires</th><th>Résultat d’exploitation</th><th>Résultat net</th><th>Référence</th></tr></thead><tbody>{report.financial_periods.map(analysis => <tr key={analysis.period.revision_id}><td>{analysis.period.period_start} → {analysis.period.period_end} (fin exclue)<small>{frequencyLabel(analysis.period.frequency)}</small></td><td>{amount(analysis.period.revenue, analysis.period.currency)}</td><td>{amount(analysis.period.operating_result ?? analysis.calculated_operating_result, analysis.period.currency)}</td><td>{amount(analysis.period.net_result, analysis.period.currency)}</td><td><code className="dashboard-source-ids">{analysis.period.id}<br />{analysis.period.revision_id}</code></td></tr>)}</tbody></table></div>}</section>
    <section className="card"><h2>Distributions de la société</h2>{!report.distributions.length ? <p>Aucune distribution enregistrée.</p> : <div className="table-scroll"><table><caption>Déclarations et paiements, conservés séparément</caption><thead><tr><th>Date</th><th>Nature</th><th>Montant</th><th>Rapprochement</th><th>Source</th></tr></thead><tbody>{report.distributions.map(item => <tr key={item.distribution.revision_id}><td>{item.distribution.effective_on}</td><td>{item.distribution.kind === 'declared' ? 'Déclaration' : 'Paiement déclaré'}</td><td>{amount(item.distribution.amount, item.distribution.currency)}</td><td>{item.distribution.kind === 'declared' ? 'Sans paiement présumé' : item.payer_cash_complete && item.beneficiary_cash_complete ? 'Deux faces rapprochées' : 'Rapprochement incomplet'}</td><td><code className="dashboard-source-ids">{item.distribution.id}<br />{item.distribution.revision_id}</code></td></tr>)}</tbody></table></div>}</section>
  </div>;
}
