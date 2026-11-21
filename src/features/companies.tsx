import { useCallback, useEffect, useRef, useState } from 'react';
import type { FormEvent, ReactNode } from 'react';
import { api, displayCurrencies, errorMessage } from '../services/api';
import { refreshAfterCommit } from '../services/mutations';
import type { CompanyDistribution, CompanyFinancialPeriod, CompanyReport, CompanySnapshot, PostedCompanyDistributionCashLink } from '../services/companies-types';
import type { Fraction, RegistrySnapshot, SaveRecord } from '../services/registry-types';
import type { TreasurySnapshot, TreasuryTransaction } from '../services/treasury-types';
import { assetAt, EditorFrame, entityAt, Field, Period, RegistryNotice } from './registry-ui';
import type { ViewContext } from './registry-ui';

type Kind = 'period' | 'distribution';
type RecordRow = CompanyFinancialPeriod | CompanyDistribution;
type Values = Record<string, string>;
type CashMode = 'existing' | 'new' | 'correct' | 'unlinked';
interface Editor {
  kind: Kind; record: RecordRow | null; values: Values; reason: string;
  mode: CashMode; transactionRevision: string; payerAccount: string; beneficiaryAccount: string;
  links: PostedCompanyDistributionCashLink[]; journalAmounts: Record<string, string>;
}
const frequencies: Record<string, string> = { monthly: 'Mensuelle', quarterly: 'Trimestrielle', annual: 'Annuelle' };
const metricLabels = {
  revenue: 'Chiffre d’affaires', operating_expenses: 'Charges d’exploitation', operating_result: 'Résultat d’exploitation déclaré',
  net_result: 'Résultat net', indicative_cash: 'Trésorerie indicative', indicative_debt: 'Dettes indicatives',
  important_receivables: 'Créances importantes', dividends_declared: 'Dividendes déclarés indicatifs',
  indicative_global_value: 'Valeur globale indicative',
};
const metricKeys = Object.keys(metricLabels) as (keyof typeof metricLabels)[];
const amount = (value: string | null, currency: string) => value === null ? 'Non renseigné' : `${value} ${currency}`;
const fraction = (value: Fraction | null) => value === null ? 'Inconnus' : `${value.num}/${value.den}`;

export default function CompaniesPage({ registry, context, onChanged, onBusyChange, onNavigateTreasury }: {
  registry: RegistrySnapshot; context: ViewContext; onChanged: () => Promise<void>;
  onBusyChange: (busy: boolean) => void; onNavigateTreasury?: () => void;
}) {
  const [snapshot, setSnapshot] = useState<CompanySnapshot | null>(null);
  const [treasury, setTreasury] = useState<TreasurySnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [tab, setTab] = useState<'periods' | 'distributions' | 'report'>('periods');
  const [selected, setSelected] = useState('');
  const [frequency, setFrequency] = useState('');
  const [editor, setEditor] = useState<Editor | null>(null);
  const [removal, setRemoval] = useState<RecordRow | null>(null);
  const [removalReason, setRemovalReason] = useState('');
  const [report, setReport] = useState<CompanyReport | null>(null);
  const requestId = useRef(0);
  const reportId = useRef(0);
  const readOnly = context.knowledgeSequence != null;
  const companies = registry.entities.filter(e => e.kind === 'company');
  const entityName = (id: string) => { const e = registry.entities.find(e => e.id === id); return e ? entityAt(registry, e, context.date).display_name : id; };
  const assetName = (id: string) => { const a = registry.assets.find(a => a.id === id); return a ? assetAt(registry, a, context.date).name : id; };
  const accountName = (id: string) => { const a = treasury?.accounts.find(a => a.id === id); return a ? `${assetName(a.asset_id)} · ${a.currency}` : id; };
  const reload = useCallback(async (afterCommit = false) => {
    const request = ++requestId.current;
    setLoading(true); setError(''); setSnapshot(null); setTreasury(null);
    try {
      const k = context.knowledgeSequence ?? null;
      const [observations, journal] = await Promise.all([api.listCompaniesAt(k), api.listTreasuryAt(k)]);
      if (request === requestId.current) { setSnapshot(observations); setTreasury(journal); }
    } catch (failure) { if (request === requestId.current) setError(errorMessage(failure)); if (afterCommit) throw failure; }
    finally { if (request === requestId.current) setLoading(false); }
  }, [context.knowledgeSequence]);
  useEffect(() => { void reload(); return () => { requestId.current += 1; }; }, [reload, registry.knowledge_sequence]);
  useEffect(() => { onBusyChange(loading || pending); return () => onBusyChange(false); }, [loading, pending, onBusyChange]);
  useEffect(() => { reportId.current += 1; setReport(null); setEditor(null); setRemoval(null); }, [context.date, context.knowledgeSequence, registry.knowledge_sequence]);
  useEffect(() => { setMessage(''); }, [context.date, context.knowledgeSequence]);
  useEffect(() => { reportId.current += 1; setReport(null); }, [selected, frequency]);

  function open(kind: Kind, record: RecordRow | null = null) {
    if (readOnly) return;
    const values: Values = { company_id: selected, period_start: '', period_end: '', frequency: 'annual', currency: 'EUR',
      ...Object.fromEntries(metricKeys.map(key => [key, ''])), notes: '', kind: 'declared', effective_on: context.date,
      beneficiary_entity_id: '', amount: '', related_declaration_id: '', unlinked_reason: '' };
    if (record) for (const [key, value] of Object.entries(record)) if (typeof value === 'string' || value === null) values[key] = value ?? '';
    const links = record && 'cash_links' in record ? record.cash_links.map(l => ({ side: l.side, line_id: l.line_id, amount: l.amount })) : [];
    const transactionRevision = record && 'cash_links' in record ? record.cash_links[0]?.transaction_revision_id ?? '' : '';
    const journalAmounts = Object.fromEntries(treasury?.transactions.find(t => t.revision_id === transactionRevision)?.entries.map(line => [line.line_id, line.amount]) ?? []);
    setEditor({ kind, record, values, reason: '', mode: 'existing', transactionRevision, payerAccount: '', beneficiaryAccount: '', links, journalAmounts });
    setRemoval(null); setError(''); setMessage('');
  }
  function update(key: string, value: string) { setEditor(current => current && ({ ...current, values: { ...current.values, [key]: value } })); }
  async function save(event: FormEvent) {
    event.preventDefault(); if (!editor || !snapshot || !treasury || readOnly) return;
    const e = editor, v = e.values;
    const meta = { id: e.record?.id ?? null, expected_revision_id: e.record?.revision_id ?? null, reason: e.reason.trim() };
    setPending(true); setError(''); setMessage('');
    try {
      if (e.kind === 'period') {
        await api.saveCompanyFinancialPeriod({ ...meta, company_id: v.company_id, period_start: v.period_start, period_end: v.period_end,
          frequency: v.frequency, currency: v.currency, revenue: v.revenue.trim() || null, operating_expenses: v.operating_expenses.trim() || null,
          operating_result: v.operating_result.trim() || null, net_result: v.net_result.trim() || null, indicative_cash: v.indicative_cash.trim() || null,
          indicative_debt: v.indicative_debt.trim() || null, important_receivables: v.important_receivables.trim() || null,
          dividends_declared: v.dividends_declared.trim() || null, indicative_global_value: v.indicative_global_value.trim() || null, notes: v.notes });
      } else {
        const distribution: SaveRecord<CompanyDistribution> = { ...meta, company_id: v.company_id, beneficiary_entity_id: v.beneficiary_entity_id,
          kind: v.kind, effective_on: v.effective_on, amount: v.amount.trim(), currency: v.currency,
          related_declaration_id: v.related_declaration_id || null, unlinked_reason: v.unlinked_reason.trim(), notes: v.notes, cash_links: [] };
        if (v.kind === 'declared' || e.mode === 'unlinked') {
          await api.saveCompanyDistribution(distribution);
        } else {
          let transaction: SaveRecord<TreasuryTransaction> | null = null;
          let links = e.links;
          if (e.mode === 'new') {
            const entries = [
              ...(e.payerAccount ? [{ line_id: 'dividend-payer', account_id: e.payerAccount, amount: `-${v.amount.trim().replace(/^-/, '')}`, role: 'principal' }] : []),
              ...(e.beneficiaryAccount ? [{ line_id: 'dividend-beneficiary', account_id: e.beneficiaryAccount, amount: v.amount.trim(), role: 'principal' }] : []),
            ];
            if (!entries.length) throw new Error('Choisissez au moins un compte réellement suivi pour enregistrer le journal.');
            transaction = { id: null, expected_revision_id: null, reason: meta.reason, kind: 'dividend', effective_on: v.effective_on,
              label: `Dividende · ${entityName(v.company_id)} → ${entityName(v.beneficiary_entity_id)}`, reference: '', notes: v.notes, entries };
            links = entries.map(line => ({ side: line.line_id === 'dividend-payer' ? 'payer' : 'beneficiary', line_id: line.line_id, amount: line.amount }));
          } else if (e.mode === 'correct') {
            const previous = treasury.transactions.find(t => t.revision_id === e.transactionRevision);
            if (!previous) throw new Error('Choisissez l’opération d’origine à corriger avec le paiement.');
            transaction = { id: previous.id, expected_revision_id: previous.revision_id, reason: meta.reason,
              kind: previous.kind, effective_on: v.effective_on, label: previous.label, reference: previous.reference, notes: previous.notes,
              entries: previous.entries.map(line => ({ ...line, amount: e.journalAmounts[line.line_id]?.trim() || line.amount })) };
          }
          await api.postCompanyDistribution({ expected_sequence: snapshot.knowledge_sequence, distribution, transaction,
            existing_transaction_revision_id: e.mode === 'existing' ? e.transactionRevision : null,
            links: links.map(link => ({ ...link, amount: link.amount.trim() })) });
        }
      }
      setEditor(null); setReport(null);
      await refreshAfterCommit(async () => { await onChanged(); await reload(true); }, 'La source de société et ses révisions ont été conservées.', setMessage);
    } catch (failure) { setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  async function remove(event: FormEvent) {
    event.preventDefault(); if (!removal || readOnly) return;
    setPending(true); setError(''); setMessage('');
    try {
      await api.voidRegistryRecord(removal.id, removal.revision_id, removalReason.trim());
      setRemoval(null); setReport(null); await refreshAfterCommit(async () => { await onChanged(); await reload(true); }, 'L’annulation reste consultable dans les anciennes connaissances.', setMessage);
    } catch (failure) { setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  async function calculate(event: FormEvent) {
    event.preventDefault(); if (!selected) return;
    const request = ++reportId.current;
    setPending(true); setError(''); setReport(null);
    try {
      const result = await api.getCompanyReport({ company_id: selected, date: context.date, frequency: frequency || null, knowledge_sequence: context.knowledgeSequence ?? null });
      if (request === reportId.current) setReport(result);
    } catch (failure) { if (request === reportId.current) setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  const relevant = (id: string) => !selected || id === selected;
  const options = (rows: { id: string; label: string }[]) => <><option value="">Choisir…</option>{rows.map(r => <option key={r.id} value={r.id}>{r.label}</option>)}</>;
  function actions(kind: Kind, record: RecordRow) { return <div className="row-actions"><button type="button" className="button secondary small-button" disabled={pending || readOnly} onClick={() => open(kind, record)}>Modifier</button><button type="button" className="text-button" disabled={pending || readOnly} onClick={() => { setRemoval(record); setRemovalReason(''); setEditor(null); }}>Annuler l’enregistrement</button></div>; }
  function table(title: string, headers: string[], rows: ReactNode[]) { return <section className="card"><h2>{title}</h2>{rows.length ? <div className="table-scroll"><table><thead><tr>{headers.map(h => <th scope="col" key={h}>{h}</th>)}</tr></thead><tbody>{rows}</tbody></table></div> : <p className="muted">Aucune source dans cette sélection.</p>}</section>; }
  const e = editor, v = e?.values ?? {};
  const field = (key: string, label: string, required = false, type = 'text') => <Field id={`company-${key}`} label={label}><input id={`company-${key}`} type={type} required={required} value={v[key] || ''} onChange={event => update(key, event.currentTarget.value)} /></Field>;
  const select = (key: string, label: string, rows: { id: string; label: string }[], required = true, disabled = false) => <Field id={`company-${key}`} label={label}><select id={`company-${key}`} required={required} disabled={disabled} value={v[key] || ''} onChange={event => update(key, event.currentTarget.value)}>{options(rows)}</select></Field>;
  const availableTransactions = treasury?.transactions.filter(t => t.kind !== 'opening') ?? [];
  const chosenTransaction = treasury?.transactions.find(t => t.revision_id === e?.transactionRevision);
  const declarations = snapshot?.distributions.filter(d => d.kind === 'declared' && d.company_id === v.company_id && d.beneficiary_entity_id === v.beneficiary_entity_id && d.currency === v.currency) ?? [];
  return <div className="registry-workspace">
    <RegistryNotice>Les observations de gestion restent distinctes des positions du bilan. Un bénéfice, une trésorerie indicative ou un dividende déclaré ne crée aucun compte ni encaissement personnel. Les montants non renseignés restent inconnus.</RegistryNotice>
    {readOnly && <RegistryNotice><strong>Lecture seule historique.</strong> Sources de sociétés connues à la séquence {context.knowledgeSequence}.</RegistryNotice>}
    <div className="module-tabs">{([['periods', 'Observations périodiques'], ['distributions', 'Distributions réelles'], ['report', 'Consulter une société']] as const).map(([id, label]) => <button type="button" key={id} className={`button ${tab === id ? '' : 'secondary'}`} disabled={pending} onClick={() => { setTab(id); setEditor(null); setRemoval(null); }}>{label}</button>)}</div>
    {loading && <p role="status">Chargement des observations de sociétés…</p>}
    {error && <RegistryNotice error><strong>Opération impossible.</strong><p>{error}</p><button type="button" className="button secondary" disabled={loading || pending} onClick={() => { void reload(); }}>Recharger les sociétés</button></RegistryNotice>}
    {message && <RegistryNotice>{message}</RegistryNotice>}
    {!loading && snapshot && treasury && <>
      <section className="card registry-filters"><Field id="company-filter" label="Société consultée"><select id="company-filter" value={selected} disabled={pending} onChange={event => setSelected(event.currentTarget.value)}><option value="">Toutes les sociétés</option>{companies.map(c => <option key={c.id} value={c.id}>{entityName(c.id)}</option>)}</select></Field><p className="field-hint">Les contrôles portent sur la société choisie. Le périmètre familial ou individuel ne transforme pas ses sommes retenues en distributions reçues.</p></section>
      {tab === 'periods' && <>
        <section className="card"><div className="section-title"><h2>Indicateurs périodiques de gestion</h2><button type="button" className="button" disabled={readOnly || pending || !companies.length} onClick={() => open('period')}>Ajouter une observation de société</button></div><p>Choisissez une période mensuelle, trimestrielle ou annuelle, avec une fin exclue. Des fréquences différentes peuvent coexister ; elles ne sont pas additionnées. Une case vide n’est pas un zéro.</p></section>
        {table('Observations enregistrées', ['Société / période', 'Activité et résultat', 'Autres indicateurs', 'Actions'], snapshot.financial_periods.filter(p => relevant(p.company_id)).map(p => <tr key={p.id}><td>{entityName(p.company_id)}<small><Period from={p.period_start} to={p.period_end} /> · {frequencies[p.frequency] || p.frequency}</small></td><td>{metricLabels.revenue} : {amount(p.revenue, p.currency)}<small>{metricLabels.operating_result} : {amount(p.operating_result, p.currency)}<br />{metricLabels.net_result} : {amount(p.net_result, p.currency)}</small></td><td><details><summary>Voir les indicateurs</summary>{metricKeys.filter(k => !['revenue', 'operating_result', 'net_result'].includes(k)).map(k => <p key={k}>{metricLabels[k]} : {amount(p[k], p.currency)}</p>)}<p>{p.notes}</p></details></td><td>{actions('period', p)}</td></tr>))}
      </>}
      {tab === 'distributions' && <>
        <section className="card"><div className="section-title"><h2>Dividendes déclarés et payés</h2><button type="button" className="button" disabled={readOnly || pending || !companies.length} onClick={() => open('distribution')}>Enregistrer une distribution de société</button></div><p>La déclaration décrit une décision ; son paiement est une autre source, avec date et bénéficiaire réels. Rapprochez les côtés effectivement suivis du journal. Une distribution interne ne constitue pas un gain familial.</p>{onNavigateTreasury && <button type="button" className="button secondary" disabled={pending} onClick={onNavigateTreasury}>Préparer une opération dans les comptes</button>}</section>
        {table('Sources de distribution', ['Société / bénéficiaire', 'Nature / date', 'Montant et trésorerie', 'Actions'], snapshot.distributions.filter(d => relevant(d.company_id)).map(d => <tr key={d.id}><td>{entityName(d.company_id)}<small>{entityName(d.beneficiary_entity_id)}</small></td><td>{d.kind === 'paid' ? 'Paiement réel' : 'Déclaration, sans paiement'}<small>{d.effective_on}</small></td><td>{d.amount} {d.currency}<small>{d.cash_links.length ? `${d.cash_links.length} lien(s) exact(s)` : 'Aucun lien bancaire'}</small><details><summary>Sources du paiement</summary>{d.cash_links.map((link, index) => <p key={index}>{link.side === 'payer' ? 'Société' : 'Bénéficiaire'} : {link.amount}<br /><span className="hash">{link.transaction_revision_id} · {link.line_id}</span></p>)}<p>{d.unlinked_reason}</p></details></td><td>{actions('distribution', d)}</td></tr>))}
      </>}
      {tab === 'report' && <section className="card"><h2>Contrôler la société à la date choisie</h2><form onSubmit={event => { void calculate(event); }}><fieldset disabled={pending}><Field id="company-report-frequency" label="Fréquence des observations comparées"><select id="company-report-frequency" value={frequency} onChange={event => setFrequency(event.currentTarget.value)}><option value="">Toutes, présentées séparément</option>{Object.entries(frequencies).map(([id, label]) => <option key={id} value={id}>{label}</option>)}</select></Field><button type="submit" className="button" disabled={!selected}>Calculer le contrôle de société</button></fieldset></form>{report && <CompanyReportView report={report} entityName={entityName} assetName={assetName} />}</section>}
      {e && !readOnly && <EditorFrame title={e.kind === 'period' ? 'Observation de gestion' : 'Distribution de société'} pending={pending} onCancel={() => setEditor(null)}><form onSubmit={event => { void save(event); }}><fieldset disabled={pending}>
        {select('company_id', 'Société de la source', companies.map(c => ({ id: c.id, label: entityName(c.id) })), true, !!e.record)}
        {e.kind === 'period' ? <>
          <div className="form-grid">{select('frequency', 'Fréquence de la période', Object.entries(frequencies).map(([id, label]) => ({ id, label })))}{select('currency', 'Devise des indicateurs', displayCurrencies.map(id => ({ id, label: id })))}</div>
          <div className="form-grid">{field('period_start', 'Début de période', true, 'date')}{field('period_end', 'Fin de période exclue', true, 'date')}</div>
          <p className="field-hint">La période débute le premier jour d’un mois et dure un, trois ou douze mois. L’exercice annuel peut être décalé. Tous les montants ci-dessous sont facultatifs et restent hors du total patrimonial.</p>
          <div className="form-grid">{metricKeys.map(key => <div key={key}>{field(key, metricLabels[key])}</div>)}</div>
        </> : <>
          {select('beneficiary_entity_id', 'Bénéficiaire du dividende', registry.entities.map(entity => ({ id: entity.id, label: entityName(entity.id) })), true, !!e.record)}
          <div className="form-grid">{select('kind', 'Nature de la distribution', [{ id: 'declared', label: 'Dividende déclaré, sans banque' }, { id: 'paid', label: 'Dividende réellement payé' }], true, !!e.record)}{field('effective_on', 'Date de la distribution', true, 'date')}</div>
          <div className="form-grid">{field('amount', 'Montant positif de la distribution', true)}{select('currency', 'Devise de la distribution', displayCurrencies.map(id => ({ id, label: id })))}</div>
          {v.kind === 'paid' && <>
            {select('related_declaration_id', 'Déclaration réglée (facultative)', declarations.map(d => ({ id: d.id, label: `${d.effective_on} · ${d.amount} ${d.currency}` })), false)}
            <Field id="company-cash-mode" label="Traitement de la trésorerie du dividende"><select id="company-cash-mode" value={e.mode} onChange={event => { const mode = event.currentTarget.value as CashMode; setEditor(current => current && ({ ...current, mode })); }}><option value="existing">Rapprocher une opération déjà enregistrée</option><option value="correct">Corriger l’opération liée avec le paiement</option><option value="new" disabled={!!e.record && 'cash_links' in e.record && e.record.cash_links.length > 0}>Créer le journal avec le paiement</option><option value="unlinked" disabled={!!e.record && 'cash_links' in e.record && e.record.cash_links.length > 0}>Conserver une source historique non rapprochée</option></select></Field>
            {e.mode === 'new' && <>
              <p>Choisissez les comptes réellement suivis, dans la devise de la distribution. Le journal comporte un débit de la société et un crédit du bénéficiaire lorsque les deux comptes sont suivis. Aucune contrepartie extérieure n’est créée automatiquement.</p>
              <Field id="company-payer-account" label="Compte débité de la société (si suivi)"><select id="company-payer-account" value={e.payerAccount} onChange={event => { const payerAccount = event.currentTarget.value; setEditor(current => current && ({ ...current, payerAccount })); }}>{options(treasury.accounts.filter(a => a.legal_owner_entity_id === v.company_id && a.currency === v.currency).map(a => ({ id: a.id, label: accountName(a.id) })))}</select></Field>
              <Field id="company-beneficiary-account" label="Compte crédité du bénéficiaire (si suivi)"><select id="company-beneficiary-account" value={e.beneficiaryAccount} onChange={event => { const beneficiaryAccount = event.currentTarget.value; setEditor(current => current && ({ ...current, beneficiaryAccount })); }}>{options(treasury.accounts.filter(a => a.legal_owner_entity_id === v.beneficiary_entity_id && a.currency === v.currency).map(a => ({ id: a.id, label: accountName(a.id) })))}</select></Field>
            </>}
            {(e.mode === 'existing' || e.mode === 'correct') && <>
              <Field id="company-cash-transaction" label="Opération du dividende au journal"><select id="company-cash-transaction" required value={e.transactionRevision} onChange={event => { const transactionRevision = event.currentTarget.value; const journalAmounts = Object.fromEntries(treasury.transactions.find(t => t.revision_id === transactionRevision)?.entries.map(line => [line.line_id, line.amount]) ?? []); setEditor(current => current && ({ ...current, transactionRevision, links: [], journalAmounts })); }}>{options(availableTransactions.map(t => ({ id: t.revision_id, label: `${t.effective_on} · ${t.label}` })))}</select></Field>
              {e.mode === 'correct' && chosenTransaction && <section className="card"><h3>Journal corrigé dans la même opération</h3><p>Toutes les lignes sont conservées. Modifiez explicitement le montant bancaire total de chaque ligne concernée, puis les montants liés à cette source ci-dessous. Les autres sources utilisant le journal restent contrôlées.</p>{chosenTransaction.entries.map((line, index) => <Field id={`company-journal-amount-${index}`} key={line.line_id} label={`Montant total de la ligne ${index + 1} (${accountName(line.account_id)})`}><input id={`company-journal-amount-${index}`} required value={e.journalAmounts[line.line_id] ?? line.amount} onChange={event => { const value = event.currentTarget.value; setEditor(current => current && ({ ...current, journalAmounts: { ...current.journalAmounts, [line.line_id]: value } })); }} /></Field>)}</section>}
              {e.links.map((link, index) => <div className="treasury-line" key={index}><div className="form-grid"><Field id={`company-cash-side-${index}`} label={`Côté du lien ${index + 1}`}><select id={`company-cash-side-${index}`} value={link.side} onChange={event => { const side = event.currentTarget.value; setEditor(current => current && ({ ...current, links: current.links.map((l, i) => i === index ? { ...l, side } : l) })); }}><option value="payer">Débit de la société</option><option value="beneficiary">Crédit du bénéficiaire</option></select></Field><Field id={`company-cash-line-${index}`} label={`Ligne du lien ${index + 1}`}><select id={`company-cash-line-${index}`} required value={link.line_id} onChange={event => { const line_id = event.currentTarget.value; setEditor(current => current && ({ ...current, links: current.links.map((l, i) => i === index ? { ...l, line_id } : l) })); }}>{options(chosenTransaction?.entries.map(line => ({ id: line.line_id, label: `${accountName(line.account_id)} · ${line.amount}` })) ?? [])}</select></Field><Field id={`company-cash-amount-${index}`} label={`Montant bancaire signé du lien ${index + 1}`}><input id={`company-cash-amount-${index}`} required value={link.amount} onChange={event => { const amount = event.currentTarget.value; setEditor(current => current && ({ ...current, links: current.links.map((l, i) => i === index ? { ...l, amount } : l) })); }} /></Field></div><button type="button" className="text-button" onClick={() => setEditor(current => current && ({ ...current, links: current.links.filter((_, i) => i !== index) }))}>Retirer le lien</button></div>)}
              <button type="button" className="button secondary" onClick={() => setEditor(current => current && ({ ...current, links: [...current.links, { side: 'payer', line_id: '', amount: '' }] }))}>Ajouter un côté du paiement</button>
            </>}
            <Field id="company-unlinked_reason" label="Explication des côtés bancaires non rapprochés"><textarea id="company-unlinked_reason" required={e.mode === 'unlinked'} value={v.unlinked_reason} onChange={event => update('unlinked_reason', event.currentTarget.value)} /></Field>
            <p className="field-hint">Une source payée incomplètement rapprochée reste signalée. Une correction d’un paiement déjà lié conserve son opération d’origine ; le mode « Corriger » révise ses lignes et la source dans le même changement.</p>
          </>}
        </>}
        <Field id="company-notes" label="Notes de la source de société"><textarea id="company-notes" value={v.notes} onChange={event => update('notes', event.currentTarget.value)} /></Field>
        <Field id="company-reason" label="Motif de l’enregistrement de société"><textarea id="company-reason" required value={e.reason} onChange={event => { const reason = event.currentTarget.value; setEditor(current => current && ({ ...current, reason })); }} /></Field>
        <button type="submit" className="button" disabled={!e.reason.trim() || (e.kind === 'distribution' && v.kind === 'paid' && ['existing', 'correct'].includes(e.mode) && !e.links.length)}>{e.kind === 'period' ? 'Enregistrer l’observation de société' : 'Enregistrer la distribution de société'}</button>
      </fieldset></form></EditorFrame>}
      {removal && !readOnly && <section className="card restore-review"><h2>Annuler la source de société</h2><p>Les anciennes connaissances conservent la source. Les dépendances et liens bancaires sont contrôlés avant l’annulation.</p><form onSubmit={event => { void remove(event); }}><fieldset disabled={pending}><Field id="company-void-reason" label="Motif de l’annulation de société"><textarea id="company-void-reason" required value={removalReason} onChange={event => setRemovalReason(event.currentTarget.value)} /></Field><button type="submit" className="button danger" disabled={!removalReason.trim()}>Confirmer l’annulation de société</button><button type="button" className="button secondary" onClick={() => setRemoval(null)}>Conserver la source</button></fieldset></form></section>}
    </>}
  </div>;
}

export function CompanyReportView({ report, entityName, assetName }: { report: CompanyReport; entityName: (id: string) => string; assetName: (id: string) => string }) {
  const table = (title: string, headers: string[], rows: ReactNode[]) => <section className="card"><h3>{title}</h3>{rows.length ? <div className="table-scroll"><table><thead><tr>{headers.map(h => <th scope="col" key={h}>{h}</th>)}</tr></thead><tbody>{rows}</tbody></table></div> : <p className="muted">Aucune source suivie dans cette sélection ; l’absence de ligne ne confirme pas un montant nul.</p>}</section>;
  return <div className="company-result"><div className="section-title"><h3>{report.company.display_name} au {report.date}</h3><span className="badge">{report.status === 'complete' ? 'Sources disponibles contrôlées' : 'Sources incomplètes ou avertissements'}</span></div>
    <p>{report.company.residence_country || 'Pays non renseigné'} · {report.company.preferred_currency}. Méthode de valorisation : {report.policy?.mode === 'global_equity' ? 'Valeur globale des parts, sous réserve de couverture' : report.policy?.mode === 'detailed' ? 'Actifs et passifs détaillés' : 'Non renseignée'}.</p>
    <RegistryNotice>Les montants ci-dessous décrivent les sources de la société. Ils ne constituent pas un total patrimonial à additionner au bilan familial. Les comptes présentent leur solde entier lorsque la société est titulaire juridique ; leurs droits économiques restent ceux du registre. Les financements présentent déjà la quote-part contractuelle de la société. Les tendances comparent uniquement des périodes compatibles ; les fréquences restent séparées.</RegistryNotice>
    {table('Participations et détentions à la date du contrôle', ['Relation', 'Entité', 'Droits économiques', 'Période'], [...report.owners.map(p => <tr key={`owner-${p.id}`}><td>Propriétaire de la société</td><td>{entityName(p.owner_entity_id)}</td><td>{fraction(p.economic_share)}</td><td><Period from={p.valid_from} to={p.valid_to} /></td></tr>), ...report.subsidiaries.map(p => <tr key={`subsidiary-${p.id}`}><td>Participation détenue</td><td>{entityName(p.company_id)}</td><td>{fraction(p.economic_share)}</td><td><Period from={p.valid_from} to={p.valid_to} /></td></tr>)])}
    {table('Actifs directement détenus', ['Actif', 'Pays', 'Droits économiques'], report.assets.map(h => <tr key={h.ownership.id}><td>{h.asset.name}</td><td>{h.asset.country}</td><td>{fraction(h.ownership.economic_share)}</td></tr>))}
    {table('Comptes suivis et soldes du journal', ['Compte', 'Solde entier dans sa devise', 'Solde entier EUR', 'Sources'], report.accounts.map(a => <tr key={a.account.id}><td>{assetName(a.account.asset_id)}</td><td>{amount(a.report.balance_original, a.account.currency)}</td><td>{amount(a.report.balance_eur, 'EUR')}</td><td><details><summary>Journal et avertissements</summary><p className="hash">{a.report.source_ids.join(', ')}</p>{a.report.warnings.map((w, i) => <p key={i}>{w.message}</p>)}</details></td></tr>))}
    {table('Financements propres connus', ['Contrat', 'Face / droits', 'Montant propre', 'EUR'], report.debt_positions.map((d, index) => <tr key={`${d.debt_id}-${d.side}-${index}`}><td>{d.name}</td><td>{d.side === 'creditor' ? 'Créance' : 'Dette'} · {fraction(d.economic_share)}</td><td>{amount(d.amount_original, d.currency)}</td><td>{amount(d.amount_eur, 'EUR')}<details><summary>Références du financement</summary><p className="hash">{d.source_ids.join(', ')}</p></details></td></tr>))}
    {table('Observations terminées et tendances comparables', ['Période', 'Indicateurs', 'Contrôle du résultat d’exploitation', 'Évolution calculée'], report.financial_periods.map(row => <tr key={row.period.id}><td><Period from={row.period.period_start} to={row.period.period_end} /><small>{frequencies[row.period.frequency]} · {row.period.currency}</small></td><td>{metricKeys.map(key => <p key={key}>{metricLabels[key]} : {amount(row.period[key], row.period.currency)}</p>)}</td><td>Chiffre d’affaires − charges : {amount(row.calculated_operating_result, row.period.currency)}<small>Écart avec le résultat déclaré : {amount(row.operating_result_difference, row.period.currency)}</small></td><td>{row.trends.length ? row.trends.map(t => <p key={t.metric}>{metricLabels[t.metric as keyof typeof metricLabels] || t.metric} : {t.difference_original} {row.period.currency} ; {t.percentage === null ? 'variation en pourcentage non calculable' : `${t.percentage} %`}<small>Comparaison avec la source {t.previous_period_id}</small></p>) : 'Aucune comparaison compatible disponible'}</td></tr>))}
    {table('Déclarations et paiements contrôlés', ['Source', 'Montant', 'Rapprochement des deux côtés', 'Solde de déclaration'], report.distributions.map(d => <tr key={d.distribution.id}><td>{d.distribution.kind === 'paid' ? 'Paiement' : 'Déclaration'} du {d.distribution.effective_on}<small>{entityName(d.distribution.beneficiary_entity_id)}</small></td><td>{d.distribution.amount} {d.distribution.currency}</td><td>{d.distribution.kind === 'declared' ? 'Aucun paiement déduit de la déclaration' : <>Société : {d.payer_cash_complete ? 'rapprochée' : 'incomplète'}<br />Bénéficiaire : {d.beneficiary_cash_complete ? 'rapproché' : 'incomplet'}</>}</td><td>{amount(d.remaining_declared, d.distribution.currency)}{d.paid_against_declaration !== null && <small>Payé contre la déclaration : {d.paid_against_declaration} {d.distribution.currency}</small>}</td></tr>))}
    {report.warnings.length > 0 && <RegistryNotice><ul>{report.warnings.map((w, i) => <li key={i}>{w.message}</li>)}</ul></RegistryNotice>}
    <details><summary>Sources du contrôle de société</summary><p>Connaissances à la séquence {report.knowledge_sequence}.</p><p className="hash">{report.source_ids.join(', ')}</p></details>
  </div>;
}
