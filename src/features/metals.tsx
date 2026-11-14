import { useCallback, useEffect, useRef, useState } from 'react';
import type { FormEvent, ReactNode } from 'react';
import { api, displayCurrencies, errorMessage } from '../services/api';
import type { Fraction, RegistrySnapshot, SaveRecord } from '../services/registry-types';
import type { MetalCashLink, MetalLot, MetalMovement, MetalPremium, MetalQuote, MetalReport, MetalSnapshot, PostedMetalCashLink } from '../services/metals-types';
import type { TreasuryEntry, TreasurySnapshot, TreasuryTransaction } from '../services/treasury-types';
import { activeAt, assetAt, EditorFrame, entityAt, Field, RegistryNotice } from './registry-ui';
import type { ViewContext } from './registry-ui';
import { FxTrace } from './exchange-rates';

type Tab = 'lots' | 'movements' | 'quotes' | 'report';
type Kind = 'lot' | 'movement' | 'quote' | 'premium' | 'link';
type Row = MetalLot | MetalMovement | MetalQuote | MetalPremium | MetalCashLink;
type Values = Record<string, string>;
type CashMode = 'unlinked' | 'new' | 'existing' | 'correct';
interface Editor { kind: Kind; record: Row | null; values: Values; reason: string; cashMode: CashMode; newLine: string; cashLinks: PostedMetalCashLink[]; entries: TreasuryEntry[] }
interface OwnerRow { entity: string; legalNum: string; legalDen: string; economicNum: string; economicDen: string }
interface Transfer { date: string; end: string; reason: string; owners: OwnerRow[] }
const metals = { gold: 'Or', silver: 'Argent' };
const forms = { bar: 'Barre', ingot: 'Lingot ou lingotin', coin: 'Pièce', other: 'Autre forme' };
const units = { g: 'Gramme', kg: 'Kilogramme', troy_oz: 'Once troy (31,1034768 g)' };
const movementKinds: Record<string, string> = { opening: 'Ouverture du stock', purchase: 'Achat', sale: 'Vente, y compris partielle', inventory_adjustment: 'Correction d’inventaire', storage_transfer: 'Transfert du lieu de stockage' };
const emptyOwner = (): OwnerRow => ({ entity: '', legalNum: '', legalDen: '', economicNum: '', economicDen: '' });
const fraction = (num: string, den: string): Fraction | null => !num.trim() && !den.trim() ? null : { num: num.trim(), den: den.trim() };
const exact = (value: Fraction | null) => value ? `${value.num}/${value.den}` : 'Inconnu';
const isTrade = (kind: string) => kind === 'purchase' || kind === 'sale';

export default function MetalsPage({ registry, context, onChanged, onBusyChange, onNavigateRegistry }: {
  registry: RegistrySnapshot; context: ViewContext; onChanged: () => Promise<void>;
  onBusyChange: (busy: boolean) => void; onNavigateRegistry: () => void;
}) {
  const [snapshot, setSnapshot] = useState<MetalSnapshot | null>(null);
  const [treasury, setTreasury] = useState<TreasurySnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [tab, setTab] = useState<Tab>('lots');
  const [selected, setSelected] = useState('');
  const [editor, setEditor] = useState<Editor | null>(null);
  const [transfer, setTransfer] = useState<Transfer | null>(null);
  const [removal, setRemoval] = useState<Row | null>(null);
  const [removalReason, setRemovalReason] = useState('');
  const [report, setReport] = useState<MetalReport | null>(null);
  const requestId = useRef(0);
  const reportId = useRef(0);
  const readOnly = context.knowledgeSequence != null;
  const reload = useCallback(async () => {
    const request = ++requestId.current;
    setLoading(true); setSnapshot(null); setTreasury(null); setReport(null); reportId.current += 1; setError('');
    try {
      const k = context.knowledgeSequence ?? null;
      const [lots, accounts] = await Promise.all([api.listMetalsAt(k), api.listTreasuryAt(k)]);
      if (request === requestId.current) {
        setSnapshot(lots); setTreasury(accounts);
        setSelected(current => lots.lots.some(l => l.id === current) ? current : '');
      }
    } catch (failure) { if (request === requestId.current) setError(errorMessage(failure)); }
    finally { if (request === requestId.current) setLoading(false); }
  }, [context.knowledgeSequence]);
  useEffect(() => { void reload(); return () => { requestId.current += 1; }; }, [reload, registry.knowledge_sequence]);
  useEffect(() => { onBusyChange(loading || pending); return () => onBusyChange(false); }, [loading, pending, onBusyChange]);
  useEffect(() => { setEditor(null); setTransfer(null); setRemoval(null); setMessage(''); setReport(null); reportId.current += 1; }, [context.date, context.personId, context.knowledgeSequence, registry.knowledge_sequence]);
  const assetName = (id: string) => { const a = registry.assets.find(item => item.id === id); return a ? assetAt(registry, a, context.date).name : id; };
  const entityName = (id: string) => { const e = registry.entities.find(item => item.id === id); return e ? entityAt(registry, e, context.date).display_name : id; };
  const lotName = (id: string) => snapshot?.lots.find(l => l.id === id)?.designation || id;
  const accountName = (id: string) => { const a = treasury?.accounts.find(item => item.id === id); return a ? `${assetName(a.asset_id)} · ${a.currency}` : id; };
  const relevant = (id: string) => !selected || selected === id;
  function edit(kind: Kind, record: Row | null = null, opening?: MetalLot) {
    if (readOnly) return;
    const lot = opening || snapshot?.lots.find(l => l.id === selected);
    const values: Values = { asset_id: '', metal: 'gold', form: 'coin', designation: '', unit_weight: '', weight_unit: 'g', purity: '', tracking_start: context.date, cost_currency: 'EUR', legal_owner_entity_id: '', country: lot?.country || 'FR', storage_location: lot?.storage_location || '', valuation_basis: 'intrinsic', notes: '', archived: 'false', lot_id: opening?.id || selected, kind: kind === 'premium' ? 'amount_per_unit' : 'opening', effective_on: opening?.tracking_start || context.date, quantity_change: '', cost_amount: '', trade_amount: '', trade_fees: '0', trade_currency: 'EUR', cash_entity_id: '', counterparty_entity_id: '', unlinked_reason: '', price: '', currency: 'EUR', source: '', value: '', discount_per_unit: '0', movement_id: '', transaction_revision_id: '', line_id: '', amount: '', cash_account: '', cash_amount: '', cash_label: '' };
    if (record) for (const [key, value] of Object.entries(record)) if (!['id', 'revision_id'].includes(key)) values[key] = value === null ? '' : String(value);
    if (opening) values.quantity_change = '';
    const links = record && kind === 'movement' ? snapshot?.cash_links.filter(l => l.movement_id === record.id) || [] : [];
    const transaction = links[0] ? treasury?.transactions.find(t => t.revision_id === links[0].transaction_revision_id) : undefined;
    if (transaction) values.transaction_revision_id = transaction.revision_id;
    setEditor({ kind, record, values, reason: '', cashMode: links.length ? 'existing' : 'unlinked', newLine: crypto.randomUUID(), cashLinks: links.map(l => ({ line_id: l.line_id, amount: l.amount })), entries: transaction?.entries.map(e => ({ ...e })) || [] });
    setTransfer(null); setRemoval(null); setError(''); setMessage('');
  }
  function movementLocation(lotId: string, date: string) {
    const lot = snapshot?.lots.find(l => l.id === lotId);
    const latest = snapshot?.movements.filter(m => m.lot_id === lotId && m.kind === 'storage_transfer' && m.effective_on < date).sort((a, b) => b.effective_on.localeCompare(a.effective_on) || b.id.localeCompare(a.id))[0];
    return { country: latest?.country || lot?.country || 'FR', storage_location: latest?.storage_location || lot?.storage_location || '' };
  }
  function update(key: string, value: string) {
    setEditor(current => {
      if (!current) return current;
      const values = { ...current.values, [key]: value };
      if (current.kind === 'movement' && ['lot_id', 'effective_on', 'kind'].includes(key) && values.kind !== 'storage_transfer') Object.assign(values, movementLocation(values.lot_id, values.effective_on));
      return { ...current, values };
    });
  }
  async function mutate(action: () => Promise<unknown>) {
    if (readOnly) return;
    setPending(true); setError(''); setMessage('');
    try { await action(); setEditor(null); setRemoval(null); setTransfer(null); await onChanged(); await reload(); setMessage('Les sources du métal ont été enregistrées avec leurs révisions.'); }
    catch (failure) { setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  async function save(event: FormEvent) {
    event.preventDefault(); if (!editor || !snapshot || readOnly) return;
    const d = editor; const v = d.values;
    const meta = { id: d.record?.id || null, expected_revision_id: d.record?.revision_id || null, reason: d.reason.trim() };
    if (d.kind === 'lot') await mutate(() => api.saveMetalLot({ ...meta, asset_id: v.asset_id, metal: v.metal, form: v.form, designation: v.designation, unit_weight: v.unit_weight.trim(), weight_unit: v.weight_unit, purity: v.purity.trim(), tracking_start: v.tracking_start, cost_currency: v.cost_currency, legal_owner_entity_id: v.legal_owner_entity_id, country: v.country, storage_location: v.storage_location, valuation_basis: v.valuation_basis, notes: v.notes, archived: v.archived === 'true' }));
    if (d.kind === 'quote') await mutate(() => api.saveMetalQuote({ ...meta, metal: v.metal, effective_on: v.effective_on, price: v.price.trim(), currency: v.currency, weight_unit: v.weight_unit, source: v.source, notes: v.notes }));
    if (d.kind === 'premium') await mutate(() => api.saveMetalPremium({ ...meta, lot_id: v.lot_id, effective_on: v.effective_on, kind: v.kind, value: v.value.trim(), currency: v.currency, discount_per_unit: v.discount_per_unit.trim(), source: v.source, notes: v.notes }));
    if (d.kind === 'link') {
      const movement = snapshot.movements.find(m => m.id === v.movement_id);
      if (!movement) { setError('Choisissez le mouvement réel de métal à rapprocher.'); return; }
      await mutate(() => api.saveMetalCashLink({ ...meta, movement_id: movement.id, movement_revision_id: movement.revision_id, transaction_revision_id: v.transaction_revision_id, line_id: v.line_id, amount: v.amount.trim(), notes: v.notes }));
    }
    if (d.kind === 'movement') {
      const trade = isTrade(v.kind);
      const movement = { ...meta, lot_id: v.lot_id, kind: v.kind, effective_on: v.effective_on, quantity_change: v.quantity_change.trim(), cost_amount: ['sale', 'storage_transfer'].includes(v.kind) ? null : v.cost_amount.trim() || null, trade_amount: trade ? v.trade_amount.trim() || null : null, trade_fees: trade ? v.trade_fees.trim() : '0', trade_currency: trade ? v.trade_currency : null, cash_entity_id: trade ? v.cash_entity_id : '', counterparty_entity_id: trade ? v.counterparty_entity_id || null : null, country: v.country, storage_location: v.storage_location, unlinked_reason: v.unlinked_reason, notes: v.notes };
      if (!trade || d.cashMode === 'unlinked') await mutate(() => api.saveMetalMovement(movement));
      else {
        let transaction: SaveRecord<TreasuryTransaction> | null = null;
        let links = d.cashLinks;
        if (d.cashMode === 'new') {
          transaction = { id: null, expected_revision_id: null, reason: d.reason.trim(), kind: 'asset_trade', effective_on: v.effective_on, label: v.cash_label || `${movementKinds[v.kind]} · ${lotName(v.lot_id)}`, reference: '', notes: v.notes, entries: [{ line_id: d.newLine, account_id: v.cash_account, amount: v.cash_amount.trim(), role: 'principal' }] };
          links = [{ line_id: d.newLine, amount: v.cash_amount.trim() }];
        }
        if (d.cashMode === 'correct') {
          const previous = treasury?.transactions.find(t => t.revision_id === v.transaction_revision_id);
          if (!previous) { setError('Choisissez l’opération bancaire d’origine à corriger.'); return; }
          transaction = { id: previous.id, expected_revision_id: previous.revision_id, reason: d.reason.trim(), kind: previous.kind, effective_on: v.effective_on, label: v.cash_label || previous.label, reference: previous.reference, notes: previous.notes, entries: d.entries };
        }
        await mutate(() => api.postMetalMovement({ expected_sequence: snapshot.knowledge_sequence, movement, transaction, existing_transaction_revision_id: d.cashMode === 'existing' ? v.transaction_revision_id : null, links }));
      }
    }
  }
  async function calculate(event: FormEvent) {
    event.preventDefault(); if (!selected) return;
    const id = ++reportId.current; setPending(true); setReport(null); setError('');
    try { const result = await api.getMetalReport({ lot_id: selected, date: context.date, knowledge_sequence: context.knowledgeSequence ?? null }); if (id === reportId.current) setReport(result); }
    catch (failure) { if (id === reportId.current) setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  function openTransfer() {
    const lot = snapshot?.lots.find(l => l.id === selected); if (!lot || readOnly) return;
    const rows = registry.ownerships.filter(o => o.asset_id === lot.asset_id && activeAt(o.valid_from, o.valid_to, context.date));
    setTransfer({ date: context.date, end: '', reason: '', owners: rows.map(o => ({ entity: o.owner_entity_id, legalNum: o.legal_share?.num || '', legalDen: o.legal_share?.den || '', economicNum: o.economic_share?.num || '', economicDen: o.economic_share?.den || '' })) });
    setEditor(null); setRemoval(null); setError('');
  }
  async function saveTransfer(event: FormEvent) {
    event.preventDefault(); if (!transfer || !snapshot || readOnly) return;
    const lot = snapshot.lots.find(l => l.id === selected); if (!lot) return;
    const reason = transfer.reason.trim();
    const previous = registry.ownerships.filter(o => o.asset_id === lot.asset_id && activeAt(o.valid_from, o.valid_to, transfer.date));
    const closes = previous.map(o => ({ id: o.id, expected_revision_id: o.revision_id, reason, asset_id: o.asset_id, owner_entity_id: o.owner_entity_id, legal_share: o.legal_share, economic_share: o.economic_share, valid_from: o.valid_from, valid_to: transfer.date }));
    const opens = transfer.owners.map(o => ({ id: null, expected_revision_id: null, reason, asset_id: lot.asset_id, owner_entity_id: o.entity, legal_share: fraction(o.legalNum, o.legalDen), economic_share: fraction(o.economicNum, o.economicDen), valid_from: transfer.date, valid_to: transfer.end || null }));
    await mutate(() => api.transferMetalOwnership({ expected_sequence: snapshot.knowledge_sequence, lot_id: lot.id, effective_on: transfer.date, ownerships: [...closes, ...opens], reason }));
  }
  function actions(kind: Kind, row: Row) {
    return <div className="row-actions"><button type="button" className="button secondary small-button" disabled={pending || readOnly} onClick={() => edit(kind, row)}>Modifier</button><button type="button" className="text-button" disabled={pending || readOnly} onClick={() => { setRemoval(row); setRemovalReason(''); setEditor(null); setTransfer(null); }}>Annuler l’enregistrement</button></div>;
  }
  function card(title: string, add: string, kind: Kind, headers: string[], body: ReactNode[]) {
    return <section className="card"><div className="section-title"><h2>{title}</h2><button type="button" className="button" disabled={pending || readOnly || ((kind === 'movement' || kind === 'premium') && !snapshot?.lots.length)} onClick={() => edit(kind)}>{add}</button></div>{body.length ? <div className="table-scroll"><table><thead><tr>{headers.map(h => <th scope="col" key={h}>{h}</th>)}</tr></thead><tbody>{body}</tbody></table></div> : <p className="muted">Aucun enregistrement dans cette sélection.</p>}</section>;
  }
  const d = editor; const v = d?.values || {};
  const options = (rows: { id: string; label: string }[]) => <><option value="">Choisir…</option>{rows.map(o => <option value={o.id} key={o.id}>{o.label}</option>)}</>;
  const input = (key: string, label: string, required = false, type = 'text') => <Field id={`metal-${key}`} label={label}><input id={`metal-${key}`} required={required} type={type} value={v[key] || ''} onChange={event => update(key, event.currentTarget.value)} /></Field>;
  const select = (key: string, label: string, rows: { id: string; label: string }[], required = true) => <Field id={`metal-${key}`} label={label}><select id={`metal-${key}`} required={required} value={v[key] || ''} onChange={event => update(key, event.currentTarget.value)}>{options(rows)}</select></Field>;
  const objectOptions = (obj: Record<string, string>) => Object.entries(obj).map(([id, label]) => ({ id, label }));
  const entityOptions = registry.entities.map(e => ({ id: e.id, label: entityName(e.id) }));
  const lotOptions = snapshot?.lots.map(l => ({ id: l.id, label: l.designation })) || [];
  const transaction = treasury?.transactions.find(t => t.revision_id === v.transaction_revision_id);
  const transactionOptions = treasury?.transactions.filter(t => t.kind !== 'opening').map(t => ({ id: t.revision_id, label: `${t.effective_on} · ${t.label}` })) || [];
  function chooseTransaction(revision: string) {
    const tx = treasury?.transactions.find(t => t.revision_id === revision);
    setEditor(current => current && ({ ...current, values: { ...current.values, transaction_revision_id: revision, line_id: '', amount: '' }, cashLinks: [{ line_id: '', amount: '' }], entries: tx?.entries.map(e => ({ ...e })) || [] }));
  }
  return <div className="registry-workspace">
    <RegistryNotice>Un lot complète un actif existant. Quantités, poids fin, coûts et cours restent séparés ; une estimation générique n’est pas ajoutée à sa valeur calculée. Les droits économiques de l’actif déterminent l’attribution du lot.</RegistryNotice>
    {readOnly && <RegistryNotice><strong>Lecture seule historique.</strong> Métaux connus à la séquence {context.knowledgeSequence}. Revenez aux connaissances actuelles pour modifier les sources.</RegistryNotice>}
    <div className="module-tabs">{([['lots', 'Lots et droits'], ['movements', 'Mouvements de métal'], ['quotes', 'Cours et primes'], ['report', 'Contrôler un lot']] as const).map(([id, label]) => <button type="button" className={`button ${tab === id ? '' : 'secondary'}`} key={id} disabled={pending} onClick={() => { setTab(id); setEditor(null); setTransfer(null); setRemoval(null); }}>{label}</button>)}</div>
    {loading && <p role="status">Chargement des métaux…</p>}
    {error && <RegistryNotice error><strong>Opération impossible.</strong><p>{error}</p><p>Les champs sont conservés après un refus. Rechargez les sources en cas de conflit de révision.</p><button type="button" className="button secondary" disabled={pending || loading} onClick={() => { void reload(); }}>Recharger les métaux</button></RegistryNotice>}
    {message && <RegistryNotice>{message}</RegistryNotice>}
    {!loading && snapshot && treasury && <>
      <section className="card"><Field id="metal-filter" label="Lot à consulter"><select id="metal-filter" value={selected} disabled={pending} onChange={event => { setSelected(event.currentTarget.value); setReport(null); reportId.current += 1; }}>{options(lotOptions)}</select></Field><button type="button" className="text-button" disabled={pending} onClick={onNavigateRegistry}>Gérer les actifs et leurs propriétaires</button></section>
      {tab === 'lots' && <>
        {card('Lots physiques', 'Créer un lot de métal', 'lot', ['Lot', 'Poids unitaire et pureté', 'Suivi et titulaire initial', 'Lieu initial', 'Actions'], snapshot.lots.filter(l => relevant(l.id)).map(l => <tr key={l.id}><td><strong>{l.designation}</strong><small>{metals[l.metal as keyof typeof metals]} · {forms[l.form as keyof typeof forms]} · {assetName(l.asset_id)}{l.archived ? ' · archivé' : ''}</small></td><td>{l.unit_weight} {l.weight_unit}<small>Pureté {l.purity}</small></td><td>{l.tracking_start}<small>{entityName(l.legal_owner_entity_id)} · coût en {l.cost_currency}</small></td><td>{l.country} · {l.storage_location}</td><td>{actions('lot', l)}<button type="button" className="text-button" disabled={pending || readOnly} onClick={() => edit('movement', null, l)}>Confirmer le stock d’ouverture</button></td></tr>))}
        <section className="card"><h2>Transfert des droits économiques et juridiques</h2><p>La quantité physique reste identique. Les droits actuels sont clôturés et les nouveaux droits ouverts ensemble à la date choisie. Les quotes-parts sont explicites ; un transfert familial ne constitue pas un gain.</p><button type="button" className="button secondary" disabled={pending || readOnly || !selected} onClick={openTransfer}>Transférer les droits du lot</button></section>
      </>}
      {tab === 'movements' && <>
        <RegistryNotice>Ouverture et achats positifs ; ventes négatives. Le coût historique restant d’un lot homogène est réparti exactement au prorata des sorties. Un transfert de stockage porte sur le lot entier, avec quantité nulle et sans banque.</RegistryNotice>
        {card('Historique des mouvements', 'Enregistrer un mouvement de métal', 'movement', ['Lot et date', 'Mouvement et quantité', 'Coût / transaction', 'Lieu', 'Actions'], snapshot.movements.filter(m => relevant(m.lot_id)).map(m => <tr key={m.id}><td>{lotName(m.lot_id)}<small>{m.effective_on}</small></td><td>{movementKinds[m.kind]}<small>{m.quantity_change}</small></td><td>{m.cost_amount === null ? 'Coût non renseigné' : `Coût ${m.cost_amount}`}{isTrade(m.kind) && <small>Prix {m.trade_amount ?? 'inconnu'} ; frais {m.trade_fees} {m.trade_currency}</small>}</td><td>{m.country} · {m.storage_location}</td><td>{actions('movement', m)}</td></tr>))}
        {card('Rapprochements de trésorerie', 'Rapprocher un mouvement de métal', 'link', ['Mouvement', 'Montant signé', 'Références', 'Actions'], snapshot.cash_links.filter(l => { const m = snapshot.movements.find(x => x.id === l.movement_id); return m && relevant(m.lot_id); }).map(l => <tr key={l.id}><td>{snapshot.movements.find(m => m.id === l.movement_id)?.effective_on || l.movement_id}</td><td>{l.amount}</td><td><details><summary>Sources exactes du rapprochement</summary><p className="hash">{l.movement_revision_id}<br />{l.transaction_revision_id}<br />{l.line_id}</p></details></td><td>{actions('link', l)}</td></tr>))}
      </>}
      {tab === 'quotes' && <>
        {card('Cours du métal fin', 'Ajouter un cours métal', 'quote', ['Métal', 'Date et cours', 'Source', 'Actions'], snapshot.quotes.map(q => <tr key={q.id}><td>{metals[q.metal as keyof typeof metals]}</td><td>{q.effective_on}<small>{q.price} {q.currency}/{q.weight_unit} fin</small></td><td>{q.source}</td><td>{actions('quote', q)}</td></tr>))}
        {card('Primes et décotes de revente', 'Définir une prime de revente', 'premium', ['Lot et date', 'Prime', 'Décote par unité', 'Source', 'Actions'], snapshot.premiums.filter(p => relevant(p.lot_id)).map(p => <tr key={p.id}><td>{lotName(p.lot_id)}<small>{p.effective_on}</small></td><td>{p.value} {p.kind === 'fraction_of_intrinsic' ? '× intrinsèque' : `${p.currency}/unité physique`}</td><td>{p.discount_per_unit} {p.currency}</td><td>{p.source}</td><td>{actions('premium', p)}</td></tr>))}
      </>}
      {tab === 'report' && <section className="card"><h2>Quantité, coût et valeur du lot</h2><p>Ce contrôle porte sur le lot entier, au {context.date}. Le bilan applique les droits du périmètre sélectionné. Une ouverture, un coût ou un cours absent reste inconnu ; une prime de revente n’est jamais présumée nulle.</p><form onSubmit={event => { void calculate(event); }}><button type="submit" className="button" disabled={pending || !selected}>Calculer le lot de métal</button></form>{report && <MetalReportView report={report} />}</section>}
      {d && !readOnly && <EditorFrame title={d.record ? 'Corriger une source métal' : 'Enregistrer une source métal'} pending={pending} onCancel={() => setEditor(null)}><form onSubmit={event => { void save(event); }}><fieldset disabled={pending}>
        {d.kind === 'lot' && <>
          {select('asset_id', 'Actif du lot', registry.assets.map(a => ({ id: a.id, label: assetName(a.id) })))}<div className="form-grid">{select('metal', 'Métal du lot', objectOptions(metals))}{select('form', 'Forme physique', objectOptions(forms))}</div>{input('designation', 'Désignation du lot', true)}<div className="form-grid">{input('unit_weight', 'Poids brut unitaire', true)}{select('weight_unit', 'Unité du poids brut', objectOptions(units))}{input('purity', 'Pureté (fraction décimale entre 0 et 1)', true)}</div><div className="form-grid">{input('tracking_start', 'Début du suivi du lot', true, 'date')}{select('cost_currency', 'Devise des coûts historiques', displayCurrencies.map(id => ({ id, label: id })))}</div>{select('legal_owner_entity_id', 'Titulaire juridique initial du lot', entityOptions)}<p className="field-hint">Le titulaire décrit le lot ; renseignez aussi les droits économiques de l’actif. Actif, métal, poids, pureté, début et devise du coût sont fixes après création.</p><div className="form-grid">{select('country', 'Pays de stockage initial', ['FR', 'TN', 'CH', 'AE'].map(id => ({ id, label: id })))}{input('storage_location', 'Lieu de stockage initial', true)}</div>{select('valuation_basis', 'Valeur retenue dans le bilan', [{ id: 'intrinsic', label: 'Valeur intrinsèque' }, { id: 'resale', label: 'Estimation de revente avec prime et décote explicites' }])}<label className="checkbox-label"><input type="checkbox" checked={v.archived === 'true'} onChange={event => update('archived', String(event.currentTarget.checked))} />Lot archivé (historique conservé)</label>
        </>}
        {d.kind === 'movement' && <>
          {select('lot_id', 'Lot du mouvement', lotOptions)}{select('kind', 'Nature du mouvement de métal', objectOptions(movementKinds))}<div className="form-grid">{input('effective_on', 'Date du mouvement de métal', true, 'date')}{input('quantity_change', 'Variation signée de quantité', true)}</div>{!['sale', 'storage_transfer'].includes(v.kind) && input('cost_amount', 'Coût ajouté dans la devise du lot (vide : inconnu)')}<p className="field-hint">Une ouverture est à la date de début du suivi. Pour un transfert de lieu, saisir 0. Les sorties d’inventaire et ventes retirent leur part du coût historique.</p><div className="form-grid">{select('country', v.kind === 'storage_transfer' ? 'Nouveau pays de stockage' : 'Pays de provenance du mouvement', ['FR', 'TN', 'CH', 'AE'].map(id => ({ id, label: id })))}{input('storage_location', v.kind === 'storage_transfer' ? 'Nouveau lieu de stockage' : 'Lieu de provenance du mouvement', true)}</div>
          {isTrade(v.kind) && <><div className="form-grid">{input('trade_amount', 'Prix total de la transaction avant frais', true)}{input('trade_fees', 'Frais réels de la transaction', true)}{select('trade_currency', 'Devise de la transaction', displayCurrencies.map(id => ({ id, label: id })))}</div>{select('cash_entity_id', 'Bénéficiaire ou payeur du cash', entityOptions)}{select('counterparty_entity_id', 'Contrepartie de la transaction (facultatif)', entityOptions, false)}
            <Field id="metal-cash-mode" label="Traitement de la trésorerie métal"><select id="metal-cash-mode" value={d.cashMode} onChange={event => { const value = event.currentTarget.value as CashMode; setEditor(current => current && ({ ...current, cashMode: value, cashLinks: current.cashLinks.length ? current.cashLinks : [{ line_id: '', amount: '' }] })); }}><option value="unlinked">Source historique non rapprochée</option><option value="new">Créer l’opération bancaire avec le mouvement</option><option value="existing">Rapprocher une opération bancaire existante</option><option value="correct">Corriger l’opération bancaire d’origine avec la source</option></select></Field>
            {d.cashMode === 'unlinked' ? <>{input('unlinked_reason', 'Motif du cash non rapproché', true)}<p className="scope-note">Aucune banque n’est créée par hypothèse. Ce flux manquant peut empêcher un bilan complet.</p></> : d.cashMode === 'new' ? <>{select('cash_account', 'Compte du mouvement métal', treasury.accounts.map(a => ({ id: a.id, label: accountName(a.id) })))}{input('cash_amount', 'Montant bancaire total signé', true)}{input('cash_label', 'Libellé bancaire du métal')}<p className="field-hint">Saisir le débit ou crédit réellement exécuté, frais inclus. Le moteur contrôle sa concordance avec le prix et les frais de la source.</p></> : <>
              <Field id="metal-transaction_revision_id" label="Opération bancaire du métal"><select id="metal-transaction_revision_id" required value={v.transaction_revision_id} onChange={event => chooseTransaction(event.currentTarget.value)}>{options(transactionOptions)}</select></Field>
              {d.cashLinks.map((link, index) => <div className="form-grid" key={index}><Field id={`metal-cash-line-${index}`} label={`Ligne bancaire métal ${index + 1}`}><select id={`metal-cash-line-${index}`} required value={link.line_id} onChange={event => { const value = event.currentTarget.value; setEditor(current => current && ({ ...current, cashLinks: current.cashLinks.map((l, i) => i === index ? { ...l, line_id: value } : l) })); }}>{options(transaction?.entries.map(e => ({ id: e.line_id, label: `${accountName(e.account_id)} · ${e.amount}` })) || [])}</select></Field><Field id={`metal-cash-linked-${index}`} label={`Montant signé rapproché métal ${index + 1}`}><input id={`metal-cash-linked-${index}`} required value={link.amount} onChange={event => { const value = event.currentTarget.value; setEditor(current => current && ({ ...current, cashLinks: current.cashLinks.map((l, i) => i === index ? { ...l, amount: value } : l) })); }} /></Field><button type="button" className="text-button" disabled={d.cashLinks.length === 1} onClick={() => setEditor(current => current && ({ ...current, cashLinks: current.cashLinks.filter((_, i) => i !== index) }))}>Retirer le lien métal</button></div>)}
              <button type="button" className="button secondary" onClick={() => setEditor(current => current && ({ ...current, cashLinks: [...current.cashLinks, { line_id: '', amount: '' }] }))}>Ajouter une ligne rapprochée métal</button>
              {d.cashMode === 'correct' && <><p>Les autres lignes restent présentes. Toute source partageant une ligne doit rester cohérente après correction.</p>{d.entries.map((entry, index) => <Field key={entry.line_id} id={`metal-corrected-amount-${index}`} label={`Nouveau montant bancaire signé ${index + 1} (${accountName(entry.account_id)})`}><input id={`metal-corrected-amount-${index}`} required value={entry.amount} onChange={event => { const value = event.currentTarget.value; setEditor(current => current && ({ ...current, entries: current.entries.map((e, i) => i === index ? { ...e, amount: value } : e) })); }} /></Field>)}</>}
            </>}
          </>}
        </>}
        {d.kind === 'quote' && <>{select('metal', 'Métal coté', objectOptions(metals))}<div className="form-grid">{input('effective_on', 'Date du cours métal', true, 'date')}{input('price', 'Prix d’une unité de métal fin', true)}{select('currency', 'Devise du cours métal', displayCurrencies.map(id => ({ id, label: id })))}{select('weight_unit', 'Unité de cotation du métal fin', objectOptions(units))}</div>{input('source', 'Source du cours métal', true)}<p className="field-hint">Le cours porte sur le métal fin. Une once ordinaire ne correspond pas à une once troy.</p></>}
        {d.kind === 'premium' && <>{select('lot_id', 'Lot de la prime', lotOptions)}{input('effective_on', 'Date de la prime', true, 'date')}{select('kind', 'Convention de prime', [{ id: 'amount_per_unit', label: 'Montant par unité physique' }, { id: 'fraction_of_intrinsic', label: 'Fraction décimale de la valeur intrinsèque' }])}{input('value', 'Valeur de la prime (0 explicite possible)', true)}{select('currency', 'Devise de la prime et décote', displayCurrencies.map(id => ({ id, label: id })))}{input('discount_per_unit', 'Décote ou frais estimés par unité physique', true)}{input('source', 'Source de la prime ou décote', true)}</>}
        {d.kind === 'link' && <>{select('movement_id', 'Mouvement réel à rapprocher', snapshot.movements.filter(m => isTrade(m.kind)).map(m => ({ id: m.id, label: `${lotName(m.lot_id)} · ${m.effective_on} · ${movementKinds[m.kind]}` })))}<Field id="metal-transaction_revision_id" label="Opération bancaire du métal"><select id="metal-transaction_revision_id" required value={v.transaction_revision_id} onChange={event => chooseTransaction(event.currentTarget.value)}>{options(transactionOptions)}</select></Field>{select('line_id', 'Ligne bancaire existante du métal', transaction?.entries.map(e => ({ id: e.line_id, label: `${accountName(e.account_id)} · ${e.amount}` })) || [])}{input('amount', 'Montant signé du lien métal', true)}</>}
        <Field id="metal-notes" label="Notes de la source métal"><textarea id="metal-notes" value={v.notes || ''} onChange={event => update('notes', event.currentTarget.value)} /></Field><Field id="metal-reason" label="Motif de l’enregistrement métal"><textarea id="metal-reason" required value={d.reason} onChange={event => { const value = event.currentTarget.value; setEditor(current => current && ({ ...current, reason: value })); }} /></Field><button type="submit" className="button" disabled={!d.reason.trim()}>{d.record ? 'Enregistrer la correction métal' : 'Enregistrer la source métal'}</button>
      </fieldset></form></EditorFrame>}
      {transfer && !readOnly && <EditorFrame title="Transférer les droits du lot" pending={pending} onCancel={() => setTransfer(null)}><form onSubmit={event => { void saveTransfer(event); }}><fieldset disabled={pending}><p>Clôturez les droits en vigueur et décrivez leur nouvelle répartition complète. Ces changements sont enregistrés ensemble ; la quantité ne change pas. Les périodes futures déjà saisies sont contrôlées.</p><div className="form-grid"><Field id="metal-transfer-date" label="Date du transfert des droits"><input id="metal-transfer-date" type="date" required value={transfer.date} onChange={event => { const value = event.currentTarget.value; setTransfer(current => current && ({ ...current, date: value })); }} /></Field><Field id="metal-transfer-end" label="Fin des nouveaux droits (exclue, facultative)"><input id="metal-transfer-end" type="date" value={transfer.end} onChange={event => { const value = event.currentTarget.value; setTransfer(current => current && ({ ...current, end: value })); }} /></Field></div>
        {transfer.owners.map((owner, index) => { const change = (key: keyof OwnerRow, value: string) => setTransfer(current => current && ({ ...current, owners: current.owners.map((o, i) => i === index ? { ...o, [key]: value } : o) })); return <div className="treasury-line" key={index}><Field id={`metal-transfer-owner-${index}`} label={`Nouveau propriétaire ${index + 1}`}><select id={`metal-transfer-owner-${index}`} required value={owner.entity} onChange={event => change('entity', event.currentTarget.value)}>{options(entityOptions)}</select></Field><div className="form-grid">{([['legalNum', 'Numérateur juridique'], ['legalDen', 'Dénominateur juridique'], ['economicNum', 'Numérateur économique'], ['economicDen', 'Dénominateur économique']] as const).map(([key, label]) => <Field key={key} id={`metal-transfer-${key}-${index}`} label={`${label} ${index + 1}`}><input id={`metal-transfer-${key}-${index}`} value={owner[key]} onChange={event => change(key, event.currentTarget.value)} /></Field>)}</div><button type="button" className="text-button" onClick={() => setTransfer(current => current && ({ ...current, owners: current.owners.filter((_, i) => i !== index) }))}>Retirer ce propriétaire</button></div>; })}
        <button type="button" className="button secondary" onClick={() => setTransfer(current => current && ({ ...current, owners: [...current.owners, emptyOwner()] }))}>Ajouter un nouveau propriétaire</button><Field id="metal-transfer-reason" label="Motif du transfert des droits"><textarea id="metal-transfer-reason" required value={transfer.reason} onChange={event => { const value = event.currentTarget.value; setTransfer(current => current && ({ ...current, reason: value })); }} /></Field><button type="submit" className="button" disabled={!transfer.reason.trim() || !transfer.owners.length}>Enregistrer le transfert des droits</button></fieldset></form></EditorFrame>}
      {removal && !readOnly && <section className="card restore-review"><h2>Annuler une source métal</h2><p>Les anciennes connaissances conserveront cette révision. Les dépendances et inventaires historiques sont contrôlés avant toute annulation.</p><form onSubmit={event => { event.preventDefault(); void mutate(() => api.voidRegistryRecord(removal.id, removal.revision_id, removalReason.trim())); }}><fieldset disabled={pending}><Field id="metal-void-reason" label="Motif de l’annulation métal"><textarea id="metal-void-reason" required value={removalReason} onChange={event => setRemovalReason(event.currentTarget.value)} /></Field><button type="submit" className="button danger" disabled={!removalReason.trim()}>Confirmer l’annulation métal</button><button type="button" className="button secondary" onClick={() => setRemoval(null)}>Conserver la source métal</button></fieldset></form></section>}
    </>}
  </div>;
}

export function MetalReportView({ report }: { report: MetalReport }) {
  const money = (value: string | null, currency = 'EUR') => value === null ? 'Non calculable' : `${value} ${currency}`;
  return <div className="metal-report-result"><div className="section-title"><h3>Lot au {report.date}</h3><span className="badge">{report.status === 'complete' ? 'Sources du lot cohérentes' : 'Sources du lot incomplètes'}</span></div><div className="rental-indicators"><div><span>Quantité restante</span><strong>{report.quantity_remaining ?? 'Inconnue'}</strong></div><div><span>Poids de métal fin</span><strong>{report.fine_weight_g === null ? 'Inconnu' : `${report.fine_weight_g} g`}</strong></div><div><span>Prix de revient restant</span><strong>{money(report.cost_remaining_original, report.cost_currency)}</strong></div><div><span>Prix de revient restant en EUR</span><strong>{money(report.cost_remaining_eur)}</strong></div><div><span>Valeur intrinsèque</span><strong>{money(report.intrinsic_eur)}</strong></div><div><span>Prime explicite</span><strong>{money(report.premium_eur)}</strong></div><div><span>Décote et frais estimés</span><strong>{money(report.discount_eur)}</strong></div><div><span>Estimation de revente</span><strong>{money(report.resale_eur)}</strong></div><div><span>Valeur retenue au bilan</span><strong>{money(report.valuation_eur)}</strong></div><div><span>Gain réalisé</span><strong>{money(report.realized_gain_eur)}</strong></div><div><span>Gain latent</span><strong>{money(report.unrealized_gain_eur)}</strong></div></div><p>Lieu suivi : {report.country} · {report.storage_location}. Base de valorisation : {report.valuation_basis === 'resale' ? 'revente explicitement documentée' : 'intrinsèque'}.</p>
    {report.quote && <details><summary>Cours retenu du métal fin</summary><p>{report.quote.price} {report.quote.currency}/{report.quote.weight_unit}, au {report.quote.effective_on} · {report.quote.source}</p><p className="hash">{report.quote.id} · {report.quote.revision_id}</p></details>}
    {report.premium && <details><summary>Prime et décote retenues</summary><p>{report.premium.value} {report.premium.kind === 'fraction_of_intrinsic' ? '× intrinsèque' : `${report.premium.currency}/unité physique`} ; décote {report.premium.discount_per_unit} {report.premium.currency}/unité physique, au {report.premium.effective_on} · {report.premium.source}</p><p className="hash">{report.premium.id} · {report.premium.revision_id}</p></details>}
    {report.cost_allocations.length > 0 && <div className="table-scroll"><table><caption>Coût alloué aux sorties et produits de vente</caption><thead><tr><th>Mouvement</th><th>Quantité</th><th>Coût historique exact</th><th>Produit net EUR exact</th><th>Gain réalisé EUR exact</th></tr></thead><tbody>{report.cost_allocations.map(a => <tr key={a.movement_id}><td className="hash">{a.movement_id}</td><td>{exact(a.quantity)}</td><td>{exact(a.exact_cost_original)} {report.cost_currency}</td><td>{exact(a.net_proceeds_eur)}</td><td>{exact(a.realized_gain_eur)}</td></tr>)}</tbody></table></div>}
    {report.warnings.length > 0 && <RegistryNotice><ul>{report.warnings.map((w, i) => <li key={`${w.code}-${i}`}>{w.message}</li>)}</ul></RegistryNotice>}
    <details><summary>Poids et coûts exacts, sources du calcul</summary><p>Quantité {exact(report.exact_quantity)} ; poids fin {exact(report.exact_fine_weight_g)} g ; coût historique {exact(report.exact_cost_remaining_original)} {report.cost_currency}. Connaissances à la séquence {report.knowledge_sequence}.</p><p className="hash">{report.source_ids.join(', ')}</p></details>
    {report.conversions.map((conversion, index) => <FxTrace key={index} result={conversion} />)}
  </div>;
}
