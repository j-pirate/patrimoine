import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { FormEvent, ReactNode } from 'react';
import { api, displayCurrencies, errorMessage } from '../services/api';
import { refreshAfterCommit } from '../services/mutations';
import type { DocumentSummary } from '../services/api';
import type { RegistrySnapshot } from '../services/registry-types';
import type { RentalSnapshot } from '../services/rental-types';
import type { PropertySnapshot } from '../services/property-types';
import type { DebtSnapshot } from '../services/debt-types';
import type { AccountReconciliation, CashLink, TreasuryAccount, TreasuryEntry, TreasuryReport, TreasurySnapshot, TreasuryTransaction } from '../services/treasury-types';
import { assetAt, EditorFrame, entityAt, Field, RegistryNotice } from './registry-ui';
import type { ViewContext } from './registry-ui';
import { FxTrace } from './exchange-rates';
import CashPostings from './treasury-postings';

type Tab = 'accounts' | 'journal' | 'reconciliations' | 'links' | 'postings';
type Kind = 'account' | 'transaction' | 'reconciliation' | 'link';
type Row = TreasuryAccount | TreasuryTransaction | AccountReconciliation | CashLink;
type Values = Record<string, string>;
interface Editor { kind: Kind; record: Row | null; values: Values; entries: TreasuryEntry[]; reason: string }
const transactionKinds: Record<string, string> = { opening: 'Ouverture du suivi', receipt: 'Entrée', payment: 'Sortie', transfer: 'Transfert interne', fx: 'Conversion exécutée', rent: 'Loyer', loan: 'Financement', asset_trade: 'Acquisition ou vente', dividend: 'Distribution', correction: 'Correction justifiée' };
const roles: Record<string, string> = { principal: 'Principal', fee: 'Frais', opening: 'Ouverture', adjustment: 'Ajustement' };
const sourceKinds: Record<string, string> = { rent_payment: 'Paiement locatif', rental_expense: 'Dépense immobilière', debt_movement: 'Mouvement de financement', deposit_movement: 'Dépôt locatif' };
export const treasuryLineId = () => globalThis.crypto.randomUUID();
const blankLine = (account_id = '', role = 'principal'): TreasuryEntry => ({ line_id: treasuryLineId(), account_id, amount: '', role });
const signed = (amount: string, negative: boolean) => `${negative ? '-' : ''}${amount.trim().replace(/^-/, '')}`;

export default function TreasuryPage({ mode, registry, context, onChanged, onBusyChange, onNavigateRegistry }: {
  mode: 'bank' | 'cash'; registry: RegistrySnapshot; context: ViewContext;
  onChanged: () => Promise<void>; onBusyChange: (busy: boolean) => void; onNavigateRegistry: () => void;
}) {
  const [tab, setTab] = useState<Tab>('accounts');
  const [snapshot, setSnapshot] = useState<TreasurySnapshot | null>(null);
  const [rentals, setRentals] = useState<RentalSnapshot | null>(null);
  const [properties, setProperties] = useState<PropertySnapshot | null>(null);
  const [debts, setDebts] = useState<DebtSnapshot | null>(null);
  const [documents, setDocuments] = useState<DocumentSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [pending, setPending] = useState(false);
  const [childBusy, setChildBusy] = useState(false);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [editor, setEditor] = useState<Editor | null>(null);
  const [removal, setRemoval] = useState<Row | null>(null);
  const [removalReason, setRemovalReason] = useState('');
  const [selectedAccount, setSelectedAccount] = useState('');
  const [report, setReport] = useState<TreasuryReport | null>(null);
  const [loadedKnowledge, setLoadedKnowledge] = useState<number | null | undefined>(undefined);
  const requestId = useRef(0);
  const reportId = useRef(0);
  const reportRegion = useRef<HTMLElement>(null);
  const journalHeading = useRef<HTMLHeadingElement>(null);
  const focusJournal = useRef(false);
  const readOnly = context.knowledgeSequence != null;
  useEffect(() => { onBusyChange(loading || pending || childBusy); return () => onBusyChange(false); }, [loading, pending, childBusy, onBusyChange]);
  const reload = useCallback(async (afterCommit = false) => {
    const request = ++requestId.current;
    setLoading(true); setError('');
    try {
      const k = context.knowledgeSequence ?? null;
      const [treasury, leases, homes, financing, docs] = await Promise.all([api.listTreasuryAt(k), api.listRentalsAt(k), api.listPropertiesAt(k), api.listDebtsAt(k), api.listDocuments()]);
      if (request === requestId.current) {
        setSnapshot(treasury); setRentals(leases); setProperties(homes); setDebts(financing); setDocuments(docs); setLoadedKnowledge(context.knowledgeSequence ?? null);
        setSelectedAccount(current => treasury.accounts.some(account => account.id === current && account.kind === mode) ? current : '');
      }
    } catch (failure) { if (request === requestId.current) setError(errorMessage(failure)); if (afterCommit) throw failure; }
    finally { if (request === requestId.current) setLoading(false); }
  }, [context.knowledgeSequence, mode]);
  useEffect(() => { void reload(); return () => { requestId.current += 1; }; }, [reload, registry.knowledge_sequence]);
  useLayoutEffect(() => {
    if (!focusJournal.current) return;
    focusJournal.current = false;
    journalHeading.current?.focus();
  }, [tab]);
  useEffect(() => { setEditor(null); setRemoval(null); setReport(null); reportId.current += 1; }, [context.date, context.knowledgeSequence, registry.knowledge_sequence, mode]);
  useEffect(() => { setMessage(''); }, [context.date, context.knowledgeSequence, mode]);
  const assetName = (id: string) => { const a = registry.assets.find(item => item.id === id); return a ? assetAt(registry, a, context.date).name : id; };
  const entityName = (id: string) => { const e = registry.entities.find(item => item.id === id); return e ? entityAt(registry, e, context.date).display_name : id; };
  const accountName = (id: string) => { const a = snapshot?.accounts.find(item => item.id === id); return a ? `${assetName(a.asset_id)} · ${a.currency}` : id; };
  const accounts = snapshot?.accounts.filter(a => a.kind === mode) ?? [];
  const openingAccounts = new Set(snapshot?.transactions
    .filter(transaction => transaction.kind === 'opening')
    .flatMap(transaction => transaction.entries.map(entry => entry.account_id)));
  const journal = snapshot?.transactions.filter(t => t.entries.some(e => accounts.some(a => a.id === e.account_id)) && (!selectedAccount || t.entries.some(e => e.account_id === selectedAccount))) ?? [];
  const allSources = {
    rent_payment: rentals?.payments.map(p => ({ id: p.id, revision_id: p.revision_id, label: `${p.reference || 'Réception locative'} · ${p.amount} ${p.currency} · ${p.effective_on}`, amount: p.amount })) ?? [],
    rental_expense: rentals?.expenses.map(p => ({ id: p.id, revision_id: p.revision_id, label: `${assetName(p.asset_id)} · ${p.amount} ${p.currency} · ${p.paid_on || 'Décaissement à préciser'}`, amount: signed(p.amount, true) })) ?? [],
    debt_movement: debts?.movements.filter(p => ['drawdown', 'principal_repayment'].includes(p.kind)).map(p => ({ id: p.id, revision_id: p.revision_id, label: `${debts.contracts.find(d => d.id === p.debt_id)?.name || p.debt_id} · ${p.effective_on}`, amount: p.principal_change })) ?? [],
    deposit_movement: snapshot?.deposits.filter(p => ['received', 'returned'].includes(p.kind)).map(p => ({ id: p.id, revision_id: p.revision_id, label: `${p.kind === 'received' ? 'Dépôt reçu' : 'Restitution'} · ${p.amount} ${p.currency} · ${p.effective_on}`, amount: p.amount })) ?? [],
  };
  async function changed() { await onChanged(); await reload(true); setReport(null); }
  async function mutate(action: () => Promise<unknown>, success: string) {
    if (readOnly) return;
    setPending(true); setError(''); setMessage('');
    try { await action(); setEditor(null); setRemoval(null); await refreshAfterCommit(changed, success, setMessage); }
    catch (failure) { setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  function edit(kind: Kind, record: Row | null = null, openingAccount?: TreasuryAccount, accountId = selectedAccount) {
    if (readOnly) return;
    const initial: Values = { asset_id: '', kind: mode, currency: 'EUR', tracking_start: context.date, legal_owner_entity_id: '', bank_name: '', reference: '', country: 'FR', storage_location: '', notes: '', archived: 'false', account_id: selectedAccount, effective_on: context.date, statement_balance: '', statement_reference: '', document_id: '', source_kind: 'rent_payment', source_id: '', transaction_revision_id: '', line_id: '', amount: '', component: 'full', side: '', label: '' };
    if (kind === 'transaction') initial.kind = openingAccount ? 'opening' : 'receipt';
    if (openingAccount) initial.effective_on = openingAccount.tracking_start;
    const values = record ? { ...initial, ...Object.fromEntries(Object.entries(record).filter(([key]) => !['id', 'revision_id', 'entries'].includes(key)).map(([key, value]) => [key, value === null ? '' : String(value)])) } : initial;
    const entries = record && 'entries' in record ? record.entries.map(e => ({ ...e })) : [blankLine(openingAccount?.id || accountId, openingAccount ? 'opening' : 'principal')];
    setError(''); setMessage(''); setRemoval(null); setEditor({ kind, record, values, entries, reason: '' });
  }
  function update(key: string, value: string) { setEditor(current => current && ({ ...current, values: { ...current.values, [key]: value } })); }
  async function save(event: FormEvent) {
    event.preventDefault(); if (!editor || readOnly) return;
    const v = editor.values; const meta = { id: editor.record?.id || null, expected_revision_id: editor.record?.revision_id || null, reason: editor.reason.trim() };
    if (editor.kind === 'account') await mutate(() => api.saveTreasuryAccount({ ...meta, asset_id: v.asset_id, kind: v.kind, currency: v.currency, tracking_start: v.tracking_start, legal_owner_entity_id: v.legal_owner_entity_id, bank_name: v.bank_name, reference: v.reference, country: v.country, storage_location: v.storage_location, notes: v.notes, archived: v.archived === 'true' }), 'Le compte a été enregistré sans créer de solde implicite.');
    if (editor.kind === 'transaction') await mutate(() => api.saveTreasuryTransaction({ ...meta, kind: v.kind, effective_on: v.effective_on, label: v.label, reference: v.reference, notes: v.notes, entries: editor.entries.map(e => ({ ...e, amount: e.amount.trim() })) }), 'Le journal a été enregistré avec toutes ses lignes dans une même opération.');
    if (editor.kind === 'reconciliation') await mutate(() => api.saveAccountReconciliation({ ...meta, account_id: v.account_id, effective_on: v.effective_on, statement_balance: v.statement_balance.trim(), statement_reference: v.statement_reference, document_id: v.document_id || null, notes: v.notes }), 'Le relevé a été enregistré. Aucun ajustement automatique de solde n’a été effectué.');
    if (editor.kind === 'link') {
      const source = allSources[v.source_kind as keyof typeof allSources]?.find(s => s.id === v.source_id);
      if (!source) { setError('Choisissez une source financière existante.'); return; }
      await mutate(() => api.saveCashLink({ ...meta, source_kind: v.source_kind, source_id: v.source_id, source_revision_id: source.revision_id, transaction_revision_id: v.transaction_revision_id, line_id: v.line_id, amount: v.amount.trim(), component: v.source_kind === 'debt_movement' ? v.component : 'full', side: v.source_kind === 'debt_movement' ? v.side : '', notes: v.notes }), 'La source a été reliée à la ligne existante, sans nouveau mouvement bancaire.');
    }
  }
  function chooseAccount(id: string) {
    setSelectedAccount(id); setReport(null); reportId.current += 1;
  }
  function openJournal(accountId: string, focusHeading = true) {
    chooseAccount(accountId);
    focusJournal.current = focusHeading;
    setTab('journal');
    setEditor(null);
    setRemoval(null);
  }
  async function inspect(accountId: string) {
    if (!accountId) return;
    const id = ++reportId.current; setPending(true); setError(''); setReport(null);
    try { const value = await api.getTreasuryReport({ account_id: accountId, date: context.date, knowledge_sequence: context.knowledgeSequence ?? null }); if (id === reportId.current) setReport(value); }
    catch (failure) { if (id === reportId.current) setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  const options = (values: { id: string; label: string }[]) => <><option value="">Choisir…</option>{values.map(v => <option key={v.id} value={v.id}>{v.label}</option>)}</>;
  const e = editor; const v = e?.values ?? {};
  const input = (key: string, label: string, required = false, type = 'text', hint?: string) => <Field id={`treasury-${key}`} label={label} hint={hint}><input id={`treasury-${key}`} aria-describedby={hint ? `treasury-${key}-help` : undefined} type={type} required={required} value={v[key] || ''} onChange={event => update(key, event.currentTarget.value)} /></Field>;
  const select = (key: string, label: string, choices: { id: string; label: string }[], required = true, hint?: string) => <Field id={`treasury-${key}`} label={label} hint={hint}><select id={`treasury-${key}`} aria-describedby={hint ? `treasury-${key}-help` : undefined} required={required} value={v[key] || ''} onChange={event => update(key, event.currentTarget.value)}>{options(choices)}</select></Field>;
  const accountOptions = snapshot?.accounts.map(a => ({ id: a.id, label: accountName(a.id) })) ?? [];
  const actions = (kind: Kind, row: Row) => <div className="row-actions"><button type="button" className="button secondary small-button" disabled={pending || readOnly} onClick={() => edit(kind, row)}>Modifier</button><button type="button" className="text-button" disabled={pending || readOnly} onClick={() => { setRemoval(row); setRemovalReason(''); setEditor(null); }}>Annuler l’enregistrement</button></div>;
  function table(title: string, headers: string[], rows: ReactNode[], empty: string) {
    return <section className="card">
      <h2>{title}</h2>
      {rows.length > 0 ? <div className="table-scroll"><table>
        <thead><tr>{headers.map(header => <th key={header} scope="col">{header}</th>)}</tr></thead>
        <tbody>{rows}</tbody>
      </table></div> : <p className="muted">{empty}</p>}
    </section>;
  }
  return <div className="registry-workspace">
    {readOnly && <RegistryNotice>Vous consultez une version enregistrée de vos comptes. Revenez à la version actuelle pour les modifier.</RegistryNotice>}
    <div className="module-tabs">{([['accounts', mode === 'cash' ? 'Caisses' : 'Comptes'], ['journal', 'Journal et transferts'], ['reconciliations', 'Relevés et soldes'], ['links', 'Rapprocher une source'], ['postings', 'Loyers et dépôts']] as const).map(([id, label]) => <button type="button" key={id} aria-pressed={tab === id} className={`button ${tab === id ? '' : 'secondary'}`} disabled={pending || childBusy} onClick={() => { setTab(id); setEditor(null); setRemoval(null); }}>{label}</button>)}</div>
    {loading && <p role="status">Chargement de la trésorerie…</p>}
    {error && <RegistryNotice error><strong>Opération impossible.</strong><p>{error}</p><p>En cas de conflit ou de source déjà liée, rechargez les données et vérifiez les références. La confirmation d’enregistrement apparaît séparément ci-dessous.</p><button type="button" className="button secondary" disabled={pending} onClick={() => { void reload(); }}>Recharger la trésorerie</button></RegistryNotice>}
    {message && <RegistryNotice>{message}</RegistryNotice>}
    {snapshot && rentals && properties && debts && loadedKnowledge === (context.knowledgeSequence ?? null) && <>
      {accounts.length > 0 && ['accounts', 'journal', 'reconciliations'].includes(tab) && <section className="card registry-filters"><Field id="treasury-filter" label={mode === 'cash' ? 'Caisse consultée' : 'Compte consulté'}><select id="treasury-filter" value={selectedAccount} disabled={pending} onChange={event => chooseAccount(event.currentTarget.value)}><option value="">{mode === 'cash' ? 'Toutes les caisses' : 'Tous les comptes'}</option>{accounts.map(a => <option key={a.id} value={a.id}>{accountName(a.id)}</option>)}</select></Field></section>}
      {tab === 'accounts' && <>
        {table(mode === 'cash' ? 'Caisses suivies' : 'Comptes suivis', ['Nom et devise', 'Titulaire et lieu', 'Début du suivi', 'Actions'], accounts.filter(a => !selectedAccount || a.id === selectedAccount).map(a => <tr key={a.id}>
          <td><strong>{assetName(a.asset_id)}</strong><small>{a.currency}{a.archived ? ' · archivé' : ''}</small></td>
          <td>{entityName(a.legal_owner_entity_id)}<small>{[a.kind === 'bank' ? a.bank_name : a.storage_location, a.reference, a.country].filter(Boolean).join(' · ')}</small></td>
          <td>{a.tracking_start}<small>{openingAccounts.has(a.id) ? 'Solde de départ renseigné' : 'Solde de départ à renseigner'}</small></td>
          <td>
            <div className="row-actions">
              <button type="button" className="button secondary small-button" disabled={pending} onClick={() => { chooseAccount(a.id); reportRegion.current?.focus(); void inspect(a.id); }}>Voir le solde</button>
              <button type="button" className="text-button" disabled={pending} onClick={() => openJournal(a.id)}>Voir les mouvements</button>
            </div>
            {actions('account', a)}
            {!openingAccounts.has(a.id) && <button type="button" className="text-button" disabled={pending || readOnly} onClick={() => { openJournal(a.id, false); edit('transaction', null, a); }}>Enregistrer l’ouverture</button>}
          </td>
        </tr>), mode === 'cash' ? 'Ajoutez une première caisse pour suivre vos espèces.' : 'Ajoutez un premier compte pour suivre son solde et ses mouvements.')}
        <details className="card advanced-disclosure" id="treasury-account-options">
          <summary>Options avancées</summary>
          <p>Vous pouvez rattacher un compte à un bien déjà créé et confirmer ensuite son solde de départ. Le titulaire du compte et la part du solde qui revient à chaque personne se renseignent séparément.</p>
          <div className="button-row">
            <button type="button" className="button secondary" disabled={pending || readOnly || !registry.assets.length || !registry.entities.length} onClick={() => edit('account')}>{mode === 'cash' ? 'Rattacher une caisse à un bien existant' : 'Rattacher un compte à un bien existant'}</button>
            <button type="button" className="text-button" disabled={pending} onClick={onNavigateRegistry}>Gérer les biens et leurs propriétaires</button>
          </div>
        </details>
      </>}
      {tab === 'journal' && <><section className="card"><div className="section-title"><h2 ref={journalHeading} tabIndex={-1}>Opérations du journal</h2><button type="button" className="button" disabled={pending || readOnly || !snapshot.accounts.length} onClick={() => edit('transaction')}>Ajouter une opération</button></div><p>Suivez les entrées, les sorties et les transferts de vos comptes.</p><details className="advanced-disclosure"><summary>Comment saisir un transfert ou un change</summary><p>Inscrivez une ligne pour chaque compte concerné, avec le montant négatif sur le compte débité et positif sur le compte crédité. Pour un change, indiquez chaque montant dans la devise de son compte et ajoutez les frais sur une ligne séparée.</p></details></section>{table('Journal des mouvements', ['Date et nature', 'Libellé', 'Montants par compte', 'Actions'], journal.map(t => <tr key={t.id}><td>{t.effective_on}<small>{transactionKinds[t.kind] || t.kind}</small></td><td>{t.label}<small>{t.reference}</small></td><td>{t.entries.map(line => <div key={line.line_id}>{accountName(line.account_id)} : <strong>{line.amount}</strong><small>{roles[line.role] || line.role}</small></div>)}</td><td>{actions('transaction', t)}</td></tr>), accounts.length ? 'Aucun mouvement dans la sélection. Ajoutez une opération pour suivre les entrées et les sorties.' : 'Créez un compte ou une caisse pour suivre ses mouvements.')}</>}
      {tab === 'reconciliations' && <><section className="card"><div className="section-title"><h2>Rapprocher un relevé</h2><button type="button" className="button" disabled={pending || readOnly || !accounts.length} onClick={() => edit('reconciliation')}>Ajouter un relevé</button></div><p>L’écart est calculé contre le journal à la date du relevé. Il ne crée jamais une écriture de correction.</p></section>{table('Relevés enregistrés', ['Compte', 'Date', 'Solde constaté', 'Référence', 'Actions'], snapshot.reconciliations.filter(r => accounts.some(a => a.id === r.account_id) && (!selectedAccount || r.account_id === selectedAccount)).map(r => <tr key={r.id}><td>{accountName(r.account_id)}</td><td>{r.effective_on}</td><td>{r.statement_balance}</td><td>{r.statement_reference}</td><td>{actions('reconciliation', r)}</td></tr>), 'Aucun relevé enregistré. Ajoutez le solde d’un relevé pour le comparer au journal.')}</>}
      {tab === 'links' && <><section className="card"><div className="section-title"><h2>Lier une source à une ligne déjà enregistrée</h2><button type="button" className="button" disabled={pending || readOnly || !snapshot.transactions.length} onClick={() => edit('link')}>Rapprocher une source financière</button></div><p>Choisissez cette action si le mouvement figure déjà dans le journal. La source reste unique ; aucun deuxième crédit ou débit n’est créé.</p></section>{table('Sources rapprochées', ['Nature', 'Montant lié', 'Composante', 'Références', 'Actions'], snapshot.cash_links.map(link => <tr key={link.id}><td>{sourceKinds[link.source_kind] || link.source_kind}</td><td>{link.amount}</td><td>{link.component === 'full' ? 'Montant complet' : link.component === 'interest' ? 'Intérêts' : roles[link.component] || 'Composante financière'}{link.side && <small>{link.side === 'debtor' ? 'Débiteur' : 'Créancier'}</small>}</td><td><details><summary>Sources exactes</summary><p className="hash">Source : {link.source_revision_id}<br />Journal : {link.transaction_revision_id}<br />Ligne : {link.line_id}</p></details></td><td>{actions('link', link)}</td></tr>), 'Aucun mouvement rapproché. Enregistrez un mouvement et sa recette ou sa dépense pour les relier.')}</>}
      {tab === 'postings' && <CashPostings registry={registry} context={context} treasury={snapshot} rentals={rentals} properties={properties} debts={debts} onChanged={changed} onBusyChange={setChildBusy} />}
      {editor && !readOnly && <EditorFrame title={editor.record ? 'Corriger la trésorerie' : 'Enregistrer la trésorerie'} pending={pending} onCancel={() => setEditor(null)}><form onSubmit={event => { void save(event); }}><fieldset disabled={pending}>
        {e?.kind === 'account' && <>{select('asset_id', 'Actif du compte', registry.assets.map(a => ({ id: a.id, label: assetName(a.id) })))}<div className="form-grid">{select('kind', 'Nature du compte', [{ id: 'bank', label: 'Compte bancaire' }, { id: 'cash', label: 'Caisse physique' }])}{select('currency', 'Devise du compte', displayCurrencies.map(code => ({ id: code, label: code })))}</div>{select('legal_owner_entity_id', 'Titulaire juridique', registry.entities.map(person => ({ id: person.id, label: entityName(person.id) })), true, 'La personne ou la société au nom de laquelle le compte est ouvert.')}<div className="form-grid">{select('country', 'Pays du compte', [{ id: 'FR', label: 'France' }, { id: 'TN', label: 'Tunisie' }, { id: 'CH', label: 'Suisse' }, { id: 'AE', label: 'Émirats arabes unis' }])}{input('tracking_start', 'Début du suivi', true, 'date', 'Date à partir de laquelle votre journal suit les mouvements. Le solde de départ est celui de ce jour.')}</div>{v.kind === 'bank' ? input('bank_name', 'Banque', true, 'text', 'Nom de l’établissement qui tient le compte.') : input('storage_location', 'Lieu de conservation', true)}<details className="advanced-disclosure" id="treasury-account-fields"><summary>Options avancées</summary>{v.kind === 'bank' && input('reference', 'Référence du compte', false, 'text', 'Facultatif. Par exemple, les quatre derniers chiffres du compte.')}<label className="checkbox-label"><input type="checkbox" checked={v.archived === 'true'} onChange={event => update('archived', String(event.currentTarget.checked))} />Compte archivé</label></details></>}
        {e?.kind === 'transaction' && <>{select('kind', 'Nature de l’opération', Object.entries(transactionKinds).map(([id, label]) => ({ id, label })))}<div className="form-grid">{input('effective_on', 'Date de l’opération', true, 'date')}{input('label', 'Libellé de l’opération', true)}</div><p className="field-hint">Saisissez le montant dans la devise du compte. Par exemple, 100 pour une entrée et -100 pour une sortie.</p>{editor.entries.map((entry, index) => <div className="treasury-line" key={entry.line_id}><div className="form-grid"><Field id={`treasury-line-account-${index}`} label={`Compte de la ligne ${index + 1}`}><select id={`treasury-line-account-${index}`} required value={entry.account_id} onChange={event => { const changedValue = event.currentTarget.value; setEditor(current => current && ({ ...current, entries: current.entries.map((line, i) => i === index ? { ...line, account_id: changedValue } : line) })); }}>{options(accountOptions)}</select></Field><Field id={`treasury-line-amount-${index}`} label={`Montant signé de la ligne ${index + 1}`}><input id={`treasury-line-amount-${index}`} required inputMode="decimal" value={entry.amount} onChange={event => { const changedValue = event.currentTarget.value; setEditor(current => current && ({ ...current, entries: current.entries.map((line, i) => i === index ? { ...line, amount: changedValue } : line) })); }} /></Field></div><Field id={`treasury-line-role-${index}`} label={`Rôle de la ligne ${index + 1}`}><select id={`treasury-line-role-${index}`} value={entry.role} onChange={event => { const changedValue = event.currentTarget.value; setEditor(current => current && ({ ...current, entries: current.entries.map((line, i) => i === index ? { ...line, role: changedValue } : line) })); }}>{Object.entries(roles).map(([id, label]) => <option value={id} key={id}>{label}</option>)}</select></Field><button type="button" className="text-button" disabled={editor.entries.length === 1} onClick={() => setEditor(current => current && ({ ...current, entries: current.entries.filter((_, i) => i !== index) }))}>Retirer cette ligne</button></div>)}<button type="button" className="button secondary" onClick={() => setEditor(current => current && ({ ...current, entries: [...current.entries, blankLine()] }))}>Ajouter une ligne au journal</button></>}
        {e?.kind === 'reconciliation' && <>{select('account_id', 'Compte du relevé', accountOptions)}<div className="form-grid">{input('effective_on', 'Date du relevé', true, 'date')}{input('statement_balance', 'Solde figurant au relevé', true)}</div>{input('statement_reference', 'Référence du relevé', true)}{select('document_id', 'Document du relevé (facultatif)', documents.map(d => ({ id: d.id, label: d.original_name })), false)}</>}
        {e?.kind === 'link' && <><Field id="treasury-source_kind" label="Nature de la source"><select id="treasury-source_kind" value={v.source_kind} onChange={event => { const changedValue = event.currentTarget.value; setEditor(current => current && ({ ...current, values: { ...current.values, source_kind: changedValue, source_id: '', amount: '', component: changedValue === 'debt_movement' ? 'principal' : 'full', side: changedValue === 'debt_movement' ? 'debtor' : '' } })); }}>{Object.entries(sourceKinds).map(([id, label]) => <option key={id} value={id}>{label}</option>)}</select></Field><Field id="treasury-source_id" label="Source à rapprocher"><select id="treasury-source_id" required value={v.source_id} onChange={event => { const changedValue = event.currentTarget.value;  const source = allSources[v.source_kind as keyof typeof allSources]?.find(s => s.id === changedValue); setEditor(current => current && ({ ...current, values: { ...current.values, source_id: changedValue, amount: source?.amount || '' } })); }}>{options(allSources[v.source_kind as keyof typeof allSources] ?? [])}</select></Field><Field id="treasury-transaction_revision_id" label="Opération déjà enregistrée"><select id="treasury-transaction_revision_id" required value={v.transaction_revision_id} onChange={event => { const changedValue = event.currentTarget.value; setEditor(current => current && ({ ...current, values: { ...current.values, transaction_revision_id: changedValue, line_id: '' } })); }}>{options(snapshot.transactions.filter(t => t.kind !== 'opening').map(t => ({ id: t.revision_id, label: `${t.effective_on} · ${t.label}` })))}</select></Field>{select('line_id', 'Ligne de trésorerie', snapshot.transactions.find(t => t.revision_id === v.transaction_revision_id)?.entries.map(line => ({ id: line.line_id, label: `${accountName(line.account_id)} · ${line.amount} · ${roles[line.role] || line.role}` })) ?? [])}{v.source_kind === 'debt_movement' && <div className="form-grid">{select('component', 'Composante du financement', [{ id: 'principal', label: 'Principal' }, { id: 'interest', label: 'Intérêts' }, { id: 'fee', label: 'Frais' }])}{select('side', 'Face du contrat', [{ id: 'debtor', label: 'Débiteur' }, { id: 'creditor', label: 'Créancier' }])}</div>}{input('amount', 'Montant signé rapproché', true)}<p className="field-hint">Le moteur contrôle le signe, la date, la devise, le bénéficiaire et les montants déjà rapprochés. La ligne existante ne sera pas dupliquée.</p></>}
        <details className="advanced-disclosure"><summary>Références et notes</summary>{e?.kind === 'transaction' && input('reference', 'Référence externe', false, 'text', 'Facultatif. Par exemple, le numéro de l’opération sur votre relevé.')}<Field id="treasury-notes" label="Notes (facultatif)"><textarea id="treasury-notes" value={v.notes || ''} onChange={event => update('notes', event.currentTarget.value)} /></Field></details><Field id="treasury-reason" label="Motif de l’enregistrement de trésorerie"><textarea id="treasury-reason" required value={editor.reason} onChange={event => { const changedValue = event.currentTarget.value; setEditor(current => current && ({ ...current, reason: changedValue })); }} /></Field><button type="submit" className="button" disabled={!editor.reason.trim()}>{editor.record ? 'Enregistrer la correction de trésorerie' : 'Enregistrer la trésorerie'}</button>
      </fieldset></form></EditorFrame>}
      {removal && !readOnly && <section className="card restore-review"><h2>Annuler un enregistrement de trésorerie</h2><p>Les anciennes versions restent conservées. Une source encore utilisée ou une ouverture nécessaire ne sera pas remplacée silencieusement.</p><form onSubmit={event => { event.preventDefault(); void mutate(() => api.voidRegistryRecord(removal.id, removal.revision_id, removalReason.trim()), 'L’enregistrement a été annulé avec conservation de son historique.'); }}><fieldset disabled={pending}><Field id="treasury-void-reason" label="Motif de l’annulation de trésorerie"><textarea id="treasury-void-reason" required value={removalReason} onChange={event => { const changedValue = event.currentTarget.value; setRemovalReason(changedValue); }} /></Field><button type="submit" className="button danger" disabled={!removalReason.trim()}>Confirmer l’annulation de trésorerie</button><button type="button" className="button secondary" onClick={() => setRemoval(null)}>Conserver l’enregistrement</button></fieldset></form></section>}
      {tab !== 'postings' && accounts.length > 0 && <section ref={reportRegion} tabIndex={-1} aria-labelledby="treasury-report-title" aria-busy={pending} className="card treasury-report">
        <h2 id="treasury-report-title">Consulter un solde</h2>
        <p>Le solde au {context.date} correspond au solde de départ et aux mouvements enregistrés jusqu’à cette date.</p>
        <form onSubmit={event => { event.preventDefault(); void inspect(selectedAccount); }}>
          <fieldset disabled={pending}>
            <Field id="treasury-report-account" label="Compte du contrôle"><select id="treasury-report-account" required value={selectedAccount} onChange={event => chooseAccount(event.currentTarget.value)}>{options(accounts.map(a => ({ id: a.id, label: accountName(a.id) })))}</select></Field>
            <button type="submit" className="button" disabled={!selectedAccount}>Calculer le solde</button>
          </fieldset>
        </form>
        {report && <TreasuryReportView report={report} />}
      </section>}
    </>}
  </div>;
}

export function TreasuryReportView({ report }: { report: TreasuryReport }) {
  return <div className="treasury-result"><div className="control-result"><span>Solde au {report.date} · séquence {report.knowledge_sequence}</span><strong>{report.balance_original === null ? 'Solde non calculable' : `${report.balance_original} ${report.currency}`}</strong><small>{report.balance_eur === null ? 'Conversion EUR non calculable' : `${report.balance_eur} EUR`}{report.overdraft ? ' · découvert à présenter en passif' : ''}</small></div>{report.reconciliations.length > 0 && <div className="table-scroll"><table><caption>Écarts avec les relevés</caption><thead><tr><th>Date</th><th>Relevé</th><th>Journal</th><th>Écart relevé − journal</th></tr></thead><tbody>{report.reconciliations.map(r => <tr key={r.revision_id}><td>{r.effective_on}</td><td>{r.statement_balance}</td><td>{r.calculated_balance ?? 'Non calculable'}</td><td>{r.difference ?? 'Non calculable'}</td></tr>)}</tbody></table></div>}{report.executed_fx.length > 0 && <section><h3>Conversions réellement exécutées</h3>{report.executed_fx.map(fx => <div key={fx.revision_id}><p>{fx.effective_on} : {fx.debited_amount} {fx.from_currency} débités ; {fx.credited_amount} {fx.to_currency} crédités.</p><p>Taux exécuté exact : {fx.exact_rate.num}/{fx.exact_rate.den} {fx.to_currency} pour 1 {fx.from_currency}.</p>{fx.fees.map((fee, index) => <p key={index}>Frais séparés : {fee.amount.num}/{fee.amount.den} {fee.currency}.</p>)}</div>)}</section>}{report.warnings.length > 0 && <RegistryNotice><ul>{report.warnings.map((w, i) => <li key={i}>{w.message}</li>)}</ul></RegistryNotice>}<details><summary>Sources et taux de valorisation</summary><p className="hash">{report.source_ids.join(', ')}</p>{report.conversions.map((conversion, i) => <FxTrace key={i} result={conversion} />)}</details></div>;
}
