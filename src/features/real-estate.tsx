import { useCallback, useEffect, useRef, useState } from 'react';
import type { FormEvent, ReactNode } from 'react';
import { api, displayCurrencies, errorMessage } from '../services/api';
import { refreshAfterCommit } from '../services/mutations';
import type { PropertySnapshot } from '../services/property-types';
import type { Lease, LeaseTerms, RefundAllocationInput, RentAllocation, RentDue, RentalExpense, RentalReport, RentalSnapshot, RentPayment } from '../services/rental-types';
import type { RegistrySnapshot } from '../services/registry-types';
import { PropertyWorkspace } from './properties';
import { FxTrace } from './exchange-rates';
import { assetAt, EditorFrame, entityAt, Field, Period, RegistryNotice } from './registry-ui';
import type { ViewContext } from './registry-ui';

type Tab = 'properties' | 'leases' | 'payments' | 'reports';
type RecordKind = 'lease' | 'terms' | 'due' | 'payment' | 'allocation' | 'expense';
type RentalRecord = Lease | LeaseTerms | RentDue | RentPayment | RentAllocation | RentalExpense;
type Values = Record<string, string>;
interface Editor { kind: RecordKind; record: RentalRecord | null; values: Values; reason: string; refundLines: RefundAllocationInput[] }
const titles: Record<RecordKind, string> = {
  lease: 'Bail', terms: 'Conditions de bail', due: 'Échéance locative', payment: 'Paiement locatif',
  allocation: 'Affectation de paiement', expense: 'Dépense immobilière',
};
const frequencies: Record<string, string> = { monthly: 'Mensuelle', quarterly: 'Trimestrielle', yearly: 'Annuelle' };
const expenseKinds: Record<string, string> = { operating: 'Exploitation', capital_works: 'Travaux en capital', interest: 'Intérêts', principal: 'Capital remboursé', financing_fee: 'Frais de financement' };
const exactLabels: Record<string, string> = {
  contractual_annual_rent: 'Loyers contractuels annualisés', rent_due: 'Loyers exigibles', charges_due: 'Charges exigibles',
  rent_received: 'Loyers encaissés', charges_received: 'Charges encaissées', outstanding_rent: 'Loyers impayés',
  outstanding_charges: 'Charges impayées', unallocated_payments: 'Paiements non affectés', economic_rent: 'Loyers économiques courus',
  economic_charges: 'Charges économiques courues', operating_expenses: 'Dépenses d’exploitation', noi: 'Revenu net d’exploitation',
};

function initialValues(kind: RecordKind, date: string): Values {
  const common = { valid_from: date, valid_to: '', notes: '', currency: 'EUR', effective_on: date };
  switch (kind) {
    case 'lease': return { ...common, unit_id: '', tenant_entity_id: '' };
    case 'terms': return { ...common, lease_id: '', rent: '', charges: '0', frequency: 'monthly', due_day: '5', deposit_required: '0' };
    case 'due': return { ...common, lease_id: '', terms_revision_id: '', period_start: date, period_end: '', due_on: date, rent: '', charges: '0', generation_key: '' };
    case 'payment': return { ...common, lease_id: '', kind: 'receipt', amount: '', original_payment_id: '', recipient_entity_id: '', reference: '', unlinked_reason: '' };
    case 'allocation': return { ...common, payment_id: '', due_id: '', component: 'rent', amount: '' };
    case 'expense': return { ...common, asset_id: '', kind: 'operating', amount: '', paid_on: '', recoverable: 'false' };
  }
}

function valuesOf(record: RentalRecord): Values {
  return Object.fromEntries(Object.entries(record).filter(([key]) => key !== 'id' && key !== 'revision_id').map(([key, value]) => [key, value === null ? '' : String(value)]));
}

export default function RealEstatePage({ registry, context, onChanged, onBusyChange, onNavigateRegistry }: {
  registry: RegistrySnapshot; context: ViewContext; onChanged: () => Promise<void>;
  onBusyChange: (busy: boolean) => void; onNavigateRegistry: () => void;
}) {
  const [tab, setTab] = useState<Tab>('properties');
  const [properties, setProperties] = useState<PropertySnapshot | null>(null);
  const [rentals, setRentals] = useState<RentalSnapshot | null>(null);
  const [loadedKnowledge, setLoadedKnowledge] = useState<number | null | undefined>(undefined);
  const [loading, setLoading] = useState(true);
  const [pending, setPending] = useState(false);
  const [childBusy, setChildBusy] = useState(false);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [selectedAsset, setSelectedAsset] = useState('');
  const [editor, setEditor] = useState<Editor | null>(null);
  const [removal, setRemoval] = useState<RentalRecord | null>(null);
  const [removalReason, setRemovalReason] = useState('');
  const [generation, setGeneration] = useState(false);
  const [generationLease, setGenerationLease] = useState('');
  const [generationFrom, setGenerationFrom] = useState(`${context.date.slice(0, 4)}-01-01`);
  const [generationTo, setGenerationTo] = useState(`${Number(context.date.slice(0, 4)) + 1}-01-01`);
  const [generationReason, setGenerationReason] = useState('');
  const [fromDate, setFromDate] = useState(`${context.date.slice(0, 4)}-01-01`);
  const [toDate, setToDate] = useState(context.date);
  const [report, setReport] = useState<RentalReport | null>(null);
  const [reportCountry, setReportCountry] = useState('');
  const [reportCompany, setReportCompany] = useState('');
  const [wholeProperty, setWholeProperty] = useState(false);
  const requestId = useRef(0);
  const reportId = useRef(0);
  const readOnly = context.knowledgeSequence != null;

  useEffect(() => { onBusyChange(pending || loading || childBusy); return () => onBusyChange(false); }, [pending, loading, childBusy, onBusyChange]);
  const reload = useCallback(async (afterCommit = false) => {
    const request = ++requestId.current;
    setLoading(true); setError('');
    try {
      const [nextProperties, nextRentals] = await Promise.all([
        api.listPropertiesAt(context.knowledgeSequence ?? null), api.listRentalsAt(context.knowledgeSequence ?? null),
      ]);
      if (request === requestId.current) { setProperties(nextProperties); setRentals(nextRentals); setLoadedKnowledge(context.knowledgeSequence ?? null); }
    } catch (failure) { if (request === requestId.current) setError(errorMessage(failure)); if (afterCommit) throw failure; }
    finally { if (request === requestId.current) setLoading(false); }
  }, [context.knowledgeSequence]);
  useEffect(() => { void reload(); return () => { requestId.current += 1; }; }, [reload, registry.knowledge_sequence]);
  useEffect(() => {
    reportId.current += 1; setReport(null); setEditor(null); setRemoval(null); setGeneration(false);
  }, [context.date, context.personId, context.knowledgeSequence, selectedAsset, registry.knowledge_sequence]);
  useEffect(() => { setMessage(''); }, [context.date, context.personId, context.knowledgeSequence, selectedAsset]);
  useEffect(() => { setToDate(context.date); }, [context.date]);
  useEffect(() => { reportId.current += 1; setReport(null); }, [reportCountry, reportCompany, wholeProperty]);
  useEffect(() => { if (!selectedAsset || reportCompany) setWholeProperty(false); }, [selectedAsset, reportCompany]);
  useEffect(() => { if (reportCompany && !registry.entities.some(entity => entity.id === reportCompany && entity.kind === 'company')) setReportCompany(''); }, [reportCompany, registry.entities]);

  async function changed() {
    reportId.current += 1; setReport(null);
    await Promise.all([reload(true), onChanged()]);
  }
  async function mutate(action: () => Promise<unknown>, success: string | (() => string)) {
    if (readOnly) return;
    setPending(true); setError(''); setMessage('');
    try { await action(); setEditor(null); setRemoval(null); setGeneration(false); await refreshAfterCommit(changed, typeof success === 'function' ? success() : success, setMessage); }
    catch (failure) { setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  function edit(kind: RecordKind, record: RentalRecord | null = null) {
    if (readOnly) return;
    setRemoval(null); setGeneration(false); setMessage(''); setError('');
    const values = record ? valuesOf(record) : initialValues(kind, context.date);
    if (!record && kind === 'expense') values.asset_id = selectedAsset;
    const refundLines = kind === 'payment' && record && 'kind' in record && record.kind === 'refund'
      ? rentals?.allocations.filter(allocation => allocation.payment_id === record.id).map(({ due_id, component, amount }) => ({ due_id, component, amount })) ?? [] : [];
    setEditor({ kind, record, values, reason: '', refundLines });
  }
  function update(key: string, value: string) {
    setEditor(current => {
      if (!current) return current;
      const values = { ...current.values, [key]: value };
      if (key === 'lease_id') values.terms_revision_id = '';
      return { ...current, values, refundLines: ['lease_id', 'kind', 'original_payment_id'].includes(key) ? [] : current.refundLines };
    });
  }
  async function save(event: FormEvent) {
    event.preventDefault(); if (!editor || readOnly) return;
    const v = editor.values;
    const meta = { id: editor.record?.id ?? null, expected_revision_id: editor.record?.revision_id ?? null, reason: editor.reason.trim() };
    const interval = { valid_from: v.valid_from, valid_to: v.valid_to || null };
    const actions: Record<RecordKind, () => Promise<unknown>> = {
      lease: () => api.saveLease({ ...meta, unit_id: v.unit_id, tenant_entity_id: v.tenant_entity_id, ...interval, notes: v.notes }),
      terms: () => api.saveLeaseTerms({ ...meta, lease_id: v.lease_id, rent: v.rent.trim(), charges: v.charges.trim(), currency: v.currency, frequency: v.frequency, due_day: Number(v.due_day), deposit_required: v.deposit_required.trim(), ...interval, notes: v.notes }),
      due: () => api.saveRentDue({ ...meta, lease_id: v.lease_id, terms_revision_id: v.terms_revision_id, period_start: v.period_start, period_end: v.period_end, due_on: v.due_on, rent: v.rent.trim(), charges: v.charges.trim(), currency: v.currency, generation_key: v.generation_key.trim(), notes: v.notes }),
      payment: () => {
        const payment = { ...meta, lease_id: v.lease_id, kind: v.kind, amount: v.amount.trim(), currency: v.currency, effective_on: v.effective_on, original_payment_id: v.kind === 'refund' ? v.original_payment_id || null : null, recipient_entity_id: v.recipient_entity_id, reference: v.reference.trim(), unlinked_reason: v.unlinked_reason.trim() };
        return v.kind === 'refund' ? api.refundRentPayment({ payment, allocations: editor.refundLines.map(line => ({ ...line, amount: line.amount.trim() })), expected_sequence: rentals!.knowledge_sequence }) : api.saveRentPayment(payment);
      },
      allocation: () => api.saveRentAllocation({ ...meta, payment_id: v.payment_id, due_id: v.due_id, component: v.component, amount: v.amount.trim(), currency: v.currency, effective_on: v.effective_on }),
      expense: () => api.saveRentalExpense({ ...meta, asset_id: v.asset_id, kind: v.kind, amount: v.amount.trim(), currency: v.currency, effective_on: v.effective_on, paid_on: v.paid_on || null, recoverable: v.recoverable === 'true', notes: v.notes }),
    };
    await mutate(actions[editor.kind], 'L’enregistrement locatif a été conservé avec son historique.');
  }
  async function generate(event: FormEvent) {
    event.preventDefault(); if (!rentals) return;
    let count = 0; let existing = 0;
    await mutate(async () => {
      const result = await api.generateRentDues({ lease_id: generationLease, from_date: generationFrom, to_date: generationTo, expected_sequence: rentals.knowledge_sequence, reason: generationReason.trim() });
      count = result.created.length; existing = result.existing_count;
    }, () => `${count} échéance(s) créée(s), ${existing} déjà présente(s).`);
  }
  async function calculate(event: FormEvent) {
    event.preventDefault(); setPending(true); setError(''); setReport(null);
    const request = ++reportId.current;
    try {
      const whole = wholeProperty && !!selectedAsset && !reportCompany;
      const result = await api.getRentalReport({ context: reportCompany ? 'company' : whole ? 'property' : context.personId ? 'person' : 'family', person_id: reportCompany || (whole ? null : context.personId), asset_id: selectedAsset || null, country: reportCountry || null, from_date: fromDate, to_date: toDate, date: context.date, knowledge_sequence: context.knowledgeSequence ?? null });
      if (request === reportId.current) setReport(result);
    } catch (failure) { if (request === reportId.current) setError(errorMessage(failure)); }
    finally { setPending(false); }
  }

  const assetName = (id: string) => { const value = registry.assets.find(asset => asset.id === id); return value ? assetAt(registry, value, context.date).name : id; };
  const entityName = (id: string) => { const value = registry.entities.find(entity => entity.id === id); return value ? entityAt(registry, value, context.date).display_name : id; };
  const propertyLabel = (id: string) => `${assetName(id)} · ${id}`;
  const unitName = (id: string) => { const unit = properties?.units.find(item => item.id === id); return unit ? `${propertyLabel(unit.property_asset_id)} · ${unit.reference} · ${unit.name}` : id; };
  const leaseName = (id: string) => { const lease = rentals?.leases.find(item => item.id === id); return lease ? `${unitName(lease.unit_id)} · ${entityName(lease.tenant_entity_id)} · ${lease.valid_from} → ${lease.valid_to || 'sans fin'} · ${lease.id}` : id; };
  const leaseAsset = (id: string) => properties?.units.find(unit => unit.id === rentals?.leases.find(lease => lease.id === id)?.unit_id)?.property_asset_id;
  const matchesLease = (id: string) => !selectedAsset || leaseAsset(id) === selectedAsset;
  const propertyAssets = registry.assets.filter(asset => properties?.properties.some(property => property.asset_id === asset.id));
  const rows = <T,>(items: T[], columns: string[], render: (item: T) => ReactNode) => items.length ? <div className="table-scroll"><table><thead><tr>{columns.map(label => <th key={label} scope="col">{label}</th>)}</tr></thead><tbody>{items.map(render)}</tbody></table></div> : <p className="muted">Aucun enregistrement dans cette sélection.</p>;
  const actions = (kind: RecordKind, record: RentalRecord) => <div className="row-actions"><button type="button" className="button secondary small-button" disabled={pending || readOnly} onClick={() => edit(kind, record)}>Modifier</button><button type="button" className="text-button" disabled={pending || readOnly} onClick={() => { setEditor(null); setGeneration(false); setRemoval(record); setRemovalReason(''); }}>Annuler l’enregistrement</button></div>;
  const card = (title: string, addLabel: string, kind: RecordKind, content: ReactNode) => <section className="card"><div className="section-title"><h2>{title}</h2><button type="button" className="button secondary" disabled={pending || readOnly} onClick={() => edit(kind)}>{addLabel}</button></div>{content}</section>;

  return <div className="registry-workspace real-estate-workspace">
    <RegistryNotice>Le bien immobilier correspond à un actif unique du registre. Les unités servent à gérer les locations et n’ajoutent aucune valeur patrimoniale. Les paiements locatifs sont rapprochés des lignes exactes du journal dans les comptes ; une échéance ne crée aucun encaissement supposé.</RegistryNotice>
    <div className="module-tabs" role="group" aria-label="Sections immobilières">{([['properties', 'Biens et unités'], ['leases', 'Baux et échéances'], ['payments', 'Paiements et dépenses'], ['reports', 'Indicateurs locatifs']] as const).map(([id, label]) => <button key={id} type="button" className={`button ${tab === id ? '' : 'secondary'}`} aria-pressed={tab === id} disabled={pending || loading || childBusy} onClick={() => { setTab(id); setEditor(null); setRemoval(null); setGeneration(false); }}>{label}</button>)}</div>
    {readOnly && <RegistryNotice><strong>Lecture seule historique.</strong> Les fiches et calculs utilisent la séquence {context.knowledgeSequence}. Revenez aux connaissances actuelles pour enregistrer une modification.</RegistryNotice>}
    {loading && <p role="status">Chargement de l’immobilier et des locations…</p>}
    {error && <RegistryNotice error><strong>Opération impossible.</strong><p>{error}</p><p>En cas de conflit, rechargez les connaissances actuelles et rouvrez le formulaire. La confirmation d’enregistrement apparaît séparément ci-dessous.</p><button type="button" className="button secondary" disabled={pending || loading} onClick={() => { void reload(); }}>Recharger l’immobilier</button></RegistryNotice>}
    {message && <RegistryNotice>{message}</RegistryNotice>}
    {properties && rentals && loadedKnowledge === (context.knowledgeSequence ?? null) && <>
      {tab === 'properties' ? <PropertyWorkspace registry={registry} snapshot={properties} context={context} onChanged={changed} onBusyChange={setChildBusy} onNavigateRegistry={onNavigateRegistry} /> : <>
        <section className="card"><Field id="rental-property-filter" label="Bien à consulter"><select id="rental-property-filter" value={selectedAsset} disabled={pending} onChange={event => setSelectedAsset(event.currentTarget.value)}><option value="">Tous les biens suivis</option>{propertyAssets.map(asset => <option value={asset.id} key={asset.id}>{propertyLabel(asset.id)}</option>)}</select></Field><p className="field-hint">Les fiches sont communes à l’espace familial. Les indicateurs utilisent le membre et la date de référence de la barre globale, ainsi que ce filtre de bien.</p></section>
        {tab === 'leases' && <>
          {card('Baux', 'Ajouter un bail', 'lease', rows(rentals.leases.filter(lease => matchesLease(lease.id)), ['Unité et locataire', 'Période', 'Notes', 'Actions'], lease => <tr key={lease.id}><td><strong>{unitName(lease.unit_id)}</strong>{entityName(lease.tenant_entity_id)}</td><td><Period from={lease.valid_from} to={lease.valid_to} /></td><td>{lease.notes || '—'}</td><td>{actions('lease', lease)}</td></tr>))}
          {card('Conditions de bail', 'Ajouter des conditions de bail', 'terms', rows(rentals.lease_terms.filter(terms => matchesLease(terms.lease_id)), ['Bail', 'Loyer et charges', 'Période', 'Dépôt contractuel', 'Actions'], terms => <tr key={terms.id}><td>{leaseName(terms.lease_id)}</td><td><strong>{terms.rent} {terms.currency}</strong>Charges {terms.charges} {terms.currency}<small>{frequencies[terms.frequency] || terms.frequency} · échéance au jour {terms.due_day}</small></td><td><Period from={terms.valid_from} to={terms.valid_to} /></td><td>{terms.deposit_required} {terms.currency}<small>Montant requis, distinct d’un dépôt reçu</small></td><td>{actions('terms', terms)}</td></tr>))}
          <section className="card"><div className="section-title"><div><h2>Générer les échéances contractuelles</h2><p>Les dates sélectionnent les échéances exigibles dans [début, fin). Une seconde génération conserve les échéances déjà présentes.</p></div><button type="button" className="button" disabled={pending || readOnly || !rentals.leases.length} onClick={() => { setGeneration(true); setEditor(null); setRemoval(null); setGenerationReason(''); }}>Générer les échéances</button></div><p className="field-hint">Les périodes suivent le calendrier de la fréquence contractuelle, avec prorata des jours civils et arrondi à l’unité monétaire à la moitié au pair par échéance. Une correction de conditions ne réécrit pas les échéances ni paiements existants. Vérifiez les références signalées par le moteur et enregistrez une correction motivée si nécessaire.</p></section>
          {card('Échéances enregistrées', 'Ajouter une échéance rectificative', 'due', rows(rentals.dues.filter(due => matchesLease(due.lease_id)), ['Bail et période courue', 'Exigibilité', 'Loyer / charges', 'Actions'], due => <tr key={due.id}><td>{leaseName(due.lease_id)}<small>Du {due.period_start} au {due.period_end} (exclu)</small><details><summary>Sources de l’échéance</summary><p className="hash">Conditions : {due.terms_revision_id}<br />Clé : {due.generation_key}</p></details></td><td>{due.due_on}</td><td>{due.rent} / {due.charges} {due.currency}</td><td>{actions('due', due)}</td></tr>))}
        </>}
        {tab === 'payments' && <>
          <RegistryNotice>Un paiement correspond à une réception ou un remboursement réel déclaré. Le motif de non-rapprochement est obligatoire pour une réception déclarée sans ligne de trésorerie liée. Les comptes permettent aussi de poster ensemble le paiement et son mouvement bancaire. Une affectation répartit ce même paiement entre les échéances ; elle ne crée pas une deuxième entrée de trésorerie. Les dépôts de garantie ne se saisissent pas comme des loyers.</RegistryNotice>
          {card('Paiements locatifs', 'Enregistrer un paiement', 'payment', rows(rentals.payments.filter(payment => matchesLease(payment.lease_id)), ['Bail et réception', 'Montant', 'Bénéficiaire déclaré', 'Référence et rapprochement', 'Actions'], payment => <tr key={payment.id}><td>{leaseName(payment.lease_id)}<small>{payment.effective_on} · {payment.kind === 'refund' ? 'Remboursement' : 'Réception'}</small></td><td>{payment.amount} {payment.currency}</td><td>{entityName(payment.recipient_entity_id)}</td><td>{payment.reference || 'Sans référence'}<small>{payment.unlinked_reason}</small></td><td>{actions('payment', payment)}</td></tr>))}
          {card('Affectations aux échéances', 'Affecter un paiement', 'allocation', rows(rentals.allocations.filter(allocation => { const payment = rentals.payments.find(item => item.id === allocation.payment_id); return payment && matchesLease(payment.lease_id); }), ['Paiement', 'Échéance', 'Composante et montant', 'Date', 'Actions'], allocation => <tr key={allocation.id}><td>{rentals.payments.find(item => item.id === allocation.payment_id)?.reference || allocation.payment_id}</td><td>{rentals.dues.find(item => item.id === allocation.due_id)?.due_on || allocation.due_id}</td><td>{allocation.component === 'rent' ? 'Loyer' : 'Charges'} · {allocation.amount} {allocation.currency}</td><td>{allocation.effective_on}</td><td>{actions('allocation', allocation)}</td></tr>))}
          {card('Dépenses immobilières', 'Ajouter une dépense', 'expense', rows(rentals.expenses.filter(expense => !selectedAsset || expense.asset_id === selectedAsset), ['Bien', 'Nature et montant', 'Dates', 'Notes', 'Actions'], expense => <tr key={expense.id}><td>{assetName(expense.asset_id)}</td><td>{expenseKinds[expense.kind] || expense.kind}<small>{expense.amount} {expense.currency}{expense.recoverable ? ' · récupérable' : ''}</small></td><td>Économique : {expense.effective_on}<small>{expense.paid_on ? `Décaissée : ${expense.paid_on}` : 'Décaissement non renseigné'}</small></td><td>{expense.notes || '—'}</td><td>{actions('expense', expense)}</td></tr>))}
        </>}
        {tab === 'reports' && <section className="card"><h2>Indicateurs locatifs sourcés</h2><p>La période est [début, fin). Les impayés sont observés à la date de référence globale {context.date}. Revenu couru et sommes reçues sont présentés séparément.</p><form onSubmit={event => { void calculate(event); }}><fieldset disabled={pending}><div className="form-grid"><Field id="rental-report-company" label="Attribution des indicateurs"><select id="rental-report-company" value={reportCompany} onChange={event => setReportCompany(event.currentTarget.value)}><option value="">{context.personId ? 'Membre sélectionné dans le contexte global' : 'Famille du contexte global'}</option>{registry.entities.filter(entity => entity.kind === 'company').map(entity => <option key={entity.id} value={entity.id}>{entity.display_name} · société propriétaire</option>)}</select></Field><Field id="rental-report-country" label="Pays des biens à la date de référence"><select id="rental-report-country" value={reportCountry} onChange={event => setReportCountry(event.currentTarget.value)}><option value="">Tous les pays</option><option value="FR">France</option><option value="TN">Tunisie</option><option value="CH">Suisse</option><option value="AE">Émirats arabes unis</option></select></Field></div><p className="field-hint">Le filtre pays utilise le profil daté du bien à la date de référence. Un pays historique inconnu n’est pas attribué par hypothèse.</p><label className="checkbox-label"><input id="rental-report-whole-property" type="checkbox" checked={wholeProperty} disabled={!selectedAsset || !!reportCompany} onChange={event => setWholeProperty(event.currentTarget.checked)} />Analyser le bien entier</label><p className="field-hint">Sans cette option explicite, sélectionner un bien conserve la part attribuable au périmètre familial ou individuel global. L’analyse du bien entier exige un bien sélectionné et remplace ce périmètre pour ce calcul seulement.</p><div className="form-grid"><Field id="rental-report-from" label="Début de la période locative"><input id="rental-report-from" type="date" required value={fromDate} onChange={event => { setFromDate(event.currentTarget.value); setReport(null); }} /></Field><Field id="rental-report-to" label="Fin de la période locative (exclue)"><input id="rental-report-to" type="date" required value={toDate} onChange={event => { setToDate(event.currentTarget.value); setReport(null); }} /></Field></div><button type="submit" className="button">Calculer les indicateurs locatifs</button></fieldset></form>{report && <RentalReportView report={report} />}</section>}
      </>}
      {editor && !readOnly && <EditorFrame title={`${editor.record ? 'Corriger' : 'Créer'} · ${titles[editor.kind]}`} pending={pending} onCancel={() => setEditor(null)}><RentalForm editor={editor} registry={registry} properties={properties} rentals={rentals} update={update} setReason={reason => setEditor(current => current && ({ ...current, reason }))} setRefundLines={refundLines => setEditor(current => current && ({ ...current, refundLines }))} pending={pending} onSubmit={save} /></EditorFrame>}
      {generation && !readOnly && <EditorFrame title="Génération des échéances" pending={pending} onCancel={() => setGeneration(false)}><form onSubmit={event => { void generate(event); }}><fieldset disabled={pending}><Field id="generation-lease" label="Bail à générer"><select id="generation-lease" required value={generationLease} onChange={event => setGenerationLease(event.currentTarget.value)}><option value="">Choisir…</option>{rentals.leases.map(lease => <option key={lease.id} value={lease.id}>{leaseName(lease.id)}</option>)}</select></Field><div className="form-grid"><Field id="generation-from" label="Début de génération"><input id="generation-from" type="date" required value={generationFrom} onChange={event => setGenerationFrom(event.currentTarget.value)} /></Field><Field id="generation-to" label="Fin de génération (exclue)"><input id="generation-to" type="date" required value={generationTo} onChange={event => setGenerationTo(event.currentTarget.value)} /></Field></div><Field id="generation-reason" label="Motif de la génération"><textarea id="generation-reason" required value={generationReason} onChange={event => setGenerationReason(event.currentTarget.value)} /></Field><button type="submit" className="button" disabled={!generationReason.trim()}>Confirmer la génération</button></fieldset></form></EditorFrame>}
      {removal && !readOnly && <section className="card restore-review"><h2>Annuler l’enregistrement locatif</h2><p>L’enregistrement sera écarté des connaissances actuelles ; ses révisions seront conservées. Une annulation laissant une dépendance ou une affectation incohérente sera refusée.</p><form onSubmit={event => { event.preventDefault(); void mutate(() => api.voidRegistryRecord(removal.id, removal.revision_id, removalReason.trim()), 'L’enregistrement a été annulé avec conservation de son historique.'); }}><fieldset disabled={pending}><Field id="rental-void-reason" label="Motif de l’annulation locative"><textarea id="rental-void-reason" required value={removalReason} onChange={event => setRemovalReason(event.currentTarget.value)} /></Field><div className="button-row"><button type="submit" className="button danger" disabled={!removalReason.trim()}>Confirmer l’annulation locative</button><button type="button" className="button secondary" onClick={() => setRemoval(null)}>Conserver l’enregistrement</button></div></fieldset></form></section>}
    </>}
  </div>;
}

function RentalForm({ editor, registry, properties, rentals, update, setReason, setRefundLines, pending, onSubmit }: {
  editor: Editor; registry: RegistrySnapshot; properties: PropertySnapshot; rentals: RentalSnapshot;
  update: (key: string, value: string) => void; setReason: (reason: string) => void; setRefundLines: (lines: RefundAllocationInput[]) => void; pending: boolean; onSubmit: (event: FormEvent) => Promise<void>;
}) {
  const v = editor.values; const kind = editor.kind;
  const input = (key: string, label: string, type = 'text', required = true, hint?: string) => <Field id={`rental-${key}`} label={label} hint={hint}><input id={`rental-${key}`} required={required} type={type} inputMode={['rent', 'charges', 'amount', 'deposit_required'].includes(key) ? 'decimal' : undefined} value={v[key] || ''} onChange={event => update(key, event.currentTarget.value)} /></Field>;
  const select = (key: string, label: string, options: { id: string; label: string }[], required = true) => <Field id={`rental-${key}`} label={label}><select id={`rental-${key}`} required={required} value={v[key] || ''} onChange={event => update(key, event.currentTarget.value)}><option value="">Choisir…</option>{options.map(option => <option key={option.id} value={option.id}>{option.label}</option>)}</select></Field>;
  const propertyLabel = (id: string) => { const asset = registry.assets.find(item => item.id === id); return `${asset ? assetAt(registry, asset, editor.values.effective_on || editor.values.valid_from).name : id} · ${id}`; };
  const unitLabel = (id: string) => { const unit = properties.units.find(item => item.id === id); return unit ? `${propertyLabel(unit.property_asset_id)} · ${unit.reference} · ${unit.name}` : id; };
  const leaseOptions = rentals.leases.map(lease => ({ id: lease.id, label: `${unitLabel(lease.unit_id)} · ${registry.entities.find(entity => entity.id === lease.tenant_entity_id)?.display_name || lease.tenant_entity_id} · ${lease.valid_from} → ${lease.valid_to || 'sans fin'} · ${lease.id}` }));
  const currency = select('currency', 'Devise de l’enregistrement', displayCurrencies.map(code => ({ id: code, label: code })));
  const dates = <div className="form-grid">{input('valid_from', 'Début de validité', 'date')}{input('valid_to', 'Fin de validité (exclue)', 'date', false)}</div>;
  const terms = [...new Map([...rentals.lease_terms, ...rentals.lease_term_revisions].filter(term => term.lease_id === v.lease_id).map(term => [term.revision_id, term])).values()];
  const selectedPayment = rentals.payments.find(payment => payment.id === v.payment_id);
  const originalAllocations = rentals.allocations.filter(allocation => allocation.payment_id === v.original_payment_id);
  const refundableDues = rentals.dues.filter(due => originalAllocations.some(allocation => allocation.due_id === due.id));
  return <form onSubmit={event => { void onSubmit(event); }}><fieldset disabled={pending}>
    {kind === 'lease' && <>{select('unit_id', 'Unité louée', properties.units.map(unit => ({ id: unit.id, label: `${unitLabel(unit.id)} · ${unit.id}` })))}{select('tenant_entity_id', 'Locataire identifié', registry.entities.map(entity => ({ id: entity.id, label: entity.display_name })))}{dates}<p className="field-hint">Un bail reste rattaché à son unité. Pour un nouveau contrat ou une nouvelle unité, créez un nouveau bail. Les contrats d’une même unité ne se chevauchent pas.</p></>}
    {['terms', 'due', 'payment'].includes(kind) && select('lease_id', 'Bail concerné', leaseOptions)}
    {kind === 'terms' && <><div className="form-grid">{input('rent', 'Loyer contractuel hors charges')}{input('charges', 'Charges contractuelles')}</div>{currency}<div className="form-grid">{select('frequency', 'Fréquence contractuelle', Object.entries(frequencies).map(([id, label]) => ({ id, label })))}{select('due_day', 'Jour d’exigibilité', Array.from({ length: 31 }, (_, index) => ({ id: String(index + 1), label: String(index + 1) })))}</div>{input('deposit_required', 'Dépôt de garantie requis', 'text', true, 'Montant contractuel uniquement. Un dépôt reçu ne constitue pas un revenu locatif.')}{dates}<p className="field-hint">Pour une révision de loyer, fermez le segment précédent et ajoutez les nouvelles conditions à partir de leur date d’effet. Les échéances existantes gardent leur source exacte.</p></>}
    {kind === 'due' && <>{select('terms_revision_id', 'Révision exacte des conditions', terms.map(term => ({ id: term.revision_id, label: `${term.valid_from} · ${term.rent} ${term.currency} · ${term.revision_id}` })))}<div className="form-grid">{input('period_start', 'Début de période courue', 'date')}{input('period_end', 'Fin de période courue (exclue)', 'date')}</div>{input('due_on', 'Date d’exigibilité', 'date')}<div className="form-grid">{input('rent', 'Loyer exigible')}{input('charges', 'Charges exigibles')}</div>{currency}{input('generation_key', 'Clé stable de l’échéance', 'text', true, 'Une clé unique identifie cette échéance à travers ses corrections. Pour les échéances courantes, privilégiez la génération contractuelle.')}<p className="field-hint">Une rectification est un fait motivé ; aucun paiement existant n’est remplacé. Une diminution sous le montant déjà affecté sera refusée.</p></>}
    {kind === 'payment' && <>{select('kind', 'Nature du paiement', [{ id: 'receipt', label: 'Réception' }, { id: 'refund', label: 'Remboursement' }])}{kind === 'payment' && v.kind === 'refund' && select('original_payment_id', 'Paiement d’origine remboursé', rentals.payments.filter(payment => payment.kind === 'receipt' && payment.lease_id === v.lease_id).map(payment => ({ id: payment.id, label: `${payment.effective_on} · ${payment.amount} ${payment.currency} · ${payment.reference || payment.id}` })))}{input('amount', 'Montant du paiement', 'text', true, v.kind === 'refund' ? 'Saisir un montant négatif ; le paiement d’origine reste conservé.' : 'Saisir le montant positif réellement reçu, même s’il couvre plusieurs échéances.')}{currency}{input('effective_on', 'Date de réception ou remboursement', 'date')}{select('recipient_entity_id', 'Bénéficiaire de l’encaissement déclaré', registry.entities.map(entity => ({ id: entity.id, label: entity.display_name })))}{input('reference', 'Référence du paiement', 'text', false)}<Field id="rental-unlinked_reason" label="Motif de non-rapprochement bancaire"><textarea id="rental-unlinked_reason" required value={v.unlinked_reason} onChange={event => update('unlinked_reason', event.currentTarget.value)} /></Field><p className="field-hint">Cette déclaration ne crée ni compte ni mouvement bancaire implicite. Dans les comptes, rapprochez une ligne existante ou enregistrez ensemble la source et sa trésorerie.</p></>}
    {kind === 'payment' && v.kind === 'refund' && <section className="refund-lines"><h3>Reprises d’affectations du remboursement</h3><p className="field-hint">Si le paiement d’origine est déjà affecté, indiquez les reprises négatives à enregistrer dans la même opération que le remboursement. Sans reprise, seul le montant non affecté reste remboursable. Aucun montant n’est déduit automatiquement.</p>{editor.refundLines.map((line, index) => <div className="refund-line" key={index}><div className="form-grid"><Field id={`refund-due-${index}`} label={`Échéance reprise ${index + 1}`}><select id={`refund-due-${index}`} required value={line.due_id} onChange={event => setRefundLines(editor.refundLines.map((item, current) => current === index ? { ...item, due_id: event.currentTarget.value } : item))}><option value="">Choisir…</option>{refundableDues.map(due => <option key={due.id} value={due.id}>{due.due_on} · {due.id}</option>)}</select></Field><Field id={`refund-component-${index}`} label={`Composante reprise ${index + 1}`}><select id={`refund-component-${index}`} value={line.component} onChange={event => setRefundLines(editor.refundLines.map((item, current) => current === index ? { ...item, component: event.currentTarget.value } : item))}><option value="rent">Loyer</option><option value="charges">Charges</option></select></Field><Field id={`refund-amount-${index}`} label={`Montant repris ${index + 1}`} hint="Montant négatif dans la devise du remboursement."><input id={`refund-amount-${index}`} required inputMode="decimal" value={line.amount} onChange={event => setRefundLines(editor.refundLines.map((item, current) => current === index ? { ...item, amount: event.currentTarget.value } : item))} /></Field></div><button type="button" className="text-button" onClick={() => setRefundLines(editor.refundLines.filter((_, current) => current !== index))}>Retirer la reprise {index + 1}</button></div>)}<button type="button" className="button secondary" disabled={!v.original_payment_id || refundableDues.length === 0} onClick={() => setRefundLines([...editor.refundLines, { due_id: '', component: 'rent', amount: '' }])}>Ajouter une reprise d’affectation</button></section>}
    {kind === 'allocation' && <>{select('payment_id', 'Paiement à affecter', rentals.payments.map(payment => ({ id: payment.id, label: `${payment.effective_on} · ${payment.amount} ${payment.currency} · ${payment.reference || payment.id}` })))}{select('due_id', 'Échéance réglée', rentals.dues.filter(due => !selectedPayment || due.lease_id === selectedPayment.lease_id).map(due => ({ id: due.id, label: `${due.due_on} · ${due.rent} + ${due.charges} ${due.currency} · ${due.id}` })))}{select('component', 'Composante réglée', [{ id: 'rent', label: 'Loyer' }, { id: 'charges', label: 'Charges récupérables' }])}{input('amount', 'Montant affecté')}{currency}{input('effective_on', 'Date d’affectation', 'date')}<p className="field-hint">Un paiement peut régler plusieurs échéances ; une échéance peut recevoir plusieurs paiements. Les dépassements et mélanges de devises sont refusés.</p></>}
    {kind === 'expense' && <>{select('asset_id', 'Bien concerné par la dépense', registry.assets.filter(asset => properties.properties.some(property => property.asset_id === asset.id)).map(asset => ({ id: asset.id, label: propertyLabel(asset.id) })))}{select('kind', 'Nature de la dépense', Object.entries(expenseKinds).map(([id, label]) => ({ id, label })))}{input('amount', 'Montant de la dépense')}{currency}{input('effective_on', 'Date économique de la dépense', 'date')}{input('paid_on', 'Date de décaissement', 'date', false)}<label className="checkbox-label"><input id="rental-recoverable" type="checkbox" checked={v.recoverable === 'true'} onChange={event => update('recoverable', String(event.currentTarget.checked))} />Charge récupérable sur le locataire</label><p className="field-hint">Les travaux en capital, intérêts, principal et frais de financement restent distincts de l’exploitation. Une charge récupérable n’est pas réputée remboursée sans échéance et paiement.</p></>}
    {['lease', 'terms', 'due', 'expense'].includes(kind) && <Field id="rental-notes" label="Notes locatives"><textarea id="rental-notes" value={v.notes || ''} onChange={event => update('notes', event.currentTarget.value)} /></Field>}
    <Field id="rental-reason" label="Motif de l’enregistrement locatif"><textarea id="rental-reason" required value={editor.reason} onChange={event => setReason(event.currentTarget.value)} /></Field><button type="submit" className="button" disabled={!editor.reason.trim()}>{editor.record ? 'Enregistrer la correction locative' : 'Créer l’enregistrement locatif'}</button>
  </fieldset></form>;
}

function RentalReportView({ report }: { report: RentalReport }) {
  const money = (value: string | null) => value === null ? 'Non calculable' : `${value} EUR`;
  const indicators = [
    ['Loyers contractuels annualisés', report.contractual_annual_rent_eur], ['Loyers exigibles sur la période', report.rent_due_eur],
    ['Loyers encaissés sur la période', report.rent_received_eur], ['Loyers impayés à la date de référence', report.outstanding_rent_eur],
    ['Charges exigibles', report.charges_due_eur], ['Charges encaissées', report.charges_received_eur],
    ['Charges impayées', report.outstanding_charges_eur], ['Paiements non affectés', report.unallocated_payments_eur],
    ['Loyers économiques courus', report.economic_rent_eur], ['Dépenses d’exploitation', report.operating_expenses_eur],
    ['Revenu net d’exploitation', report.noi_eur], ['Cash-flow après financement', report.cash_flow_after_financing_eur],
  ] as const;
  return <div className="rental-report"><div className="section-title"><h3>Résultats locatifs à la séquence {report.knowledge_sequence}</h3><span className="badge">{report.status === 'complete' ? 'Références disponibles' : 'Données partielles'}</span></div><p>Du {report.from_date} au {report.to_date} (exclu) · situation des impayés au {report.date}.</p><dl className="rental-indicators">{indicators.map(([label, value]) => <div key={label}><dt>{label}</dt><dd>{money(value)}</dd></div>)}<div><dt>Rendement locatif brut</dt><dd>{report.gross_yield_percent === null ? 'Non calculable' : `${report.gross_yield_percent} %`}</dd></div><div><dt>Rendement net avant financement</dt><dd>{report.net_yield_percent === null ? 'Non calculable' : `${report.net_yield_percent} %`}</dd></div><div><dt>Occupation contractuelle</dt><dd>{report.occupation.percent === null ? 'Non calculable' : `${report.occupation.percent} %`}<small>{report.occupation.occupied_unit_days} / {report.occupation.available_unit_days} jours-unités disponibles</small></dd></div></dl><p className="scope-note">{report.occupation.basis === 'contractual' ? 'L’occupation est déduite des périodes de bail, sans constat physique.' : report.occupation.basis} Les indicateurs de revenu économique suivent les droits pendant la période courue ; les encaissements suivent leur bénéficiaire déclaré et leur date. Un revenu conservé dans une société n’est pas une distribution à ses associés. Le cash-flow après financement exige des sources rapprochées et une couverture suffisante ; une valeur absente ne vaut pas zéro. Les impayés sont attribués proportionnellement aux droits historiques des jours courus ; l’attribution juridique préférentielle et la cession individualisée des créances ne sont pas encore gérées. Le bilan financier utilise les droits explicites sur les créances pour les rapprocher avec les comptes et passifs ; une couverture insuffisante y reste signalée.</p>
    {report.dues.length > 0 && <div className="table-scroll"><table><caption>Échéances à la date de référence · montants contractuels complets avant attribution</caption><thead><tr><th scope="col">Échéance</th><th scope="col">Loyer / charges</th><th scope="col">Réglé</th><th scope="col">Restant</th><th scope="col">Retard</th></tr></thead><tbody>{report.dues.map(due => <tr key={due.due_id}><td>{due.due_on}<small>{due.period_start} → {due.period_end} (exclu)</small><details><summary>Sources de la créance</summary><p className="hash">{due.source_ids.join(', ')}</p></details></td><td>{due.rent} / {due.charges} {due.currency}</td><td>{due.allocated_rent} / {due.allocated_charges} {due.currency}</td><td>{due.outstanding_rent} / {due.outstanding_charges} {due.currency}</td><td>{due.overdue_days > 0 ? `${due.overdue_days} jour(s)` : 'Pas de retard courant'}{due.settled_on && <small>Soldée le {due.settled_on}</small>}{due.historical_late && <small>Retard historique conservé</small>}</td></tr>)}</tbody></table></div>}
    {Object.keys(report.exact_totals).length > 0 && <details><summary>Totaux exacts avant arrondi</summary><dl className="details">{Object.entries(report.exact_totals).map(([key, value]) => <div key={key}><dt>{exactLabels[key] || 'Autre composante exacte'}</dt><dd className="hash">{value.num}/{value.den}</dd></div>)}</dl></details>}
    <details><summary>Sources et taux de conversion</summary><p className="hash">{report.source_ids.join(', ') || 'Aucune source financière applicable dans cette sélection.'}</p>{report.conversions.length > 0 && <><p className="field-hint">Cotations unitaires consultées pour les dates et devises du calcul ; chaque montant source reste conservé dans son enregistrement.</p>{report.conversions.map((conversion, index) => <FxTrace key={index} result={conversion} />)}</>}</details>
    {report.warnings.length > 0 && <RegistryNotice><ul>{report.warnings.map((warning, index) => <li key={`${warning.code}-${warning.object_id}-${index}`}>{warning.message}</li>)}</ul></RegistryNotice>}
  </div>;
}
