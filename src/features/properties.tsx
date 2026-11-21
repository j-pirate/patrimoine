import { useEffect, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { api, displayCurrencies, errorMessage } from '../services/api';
import { refreshAfterCommit } from '../services/mutations';
import type { Property, PropertyEvent, PropertyReport, PropertySnapshot, RentalUnit } from '../services/property-types';
import type { RegistryRecord, RegistrySnapshot } from '../services/registry-types';
import { FxTrace } from './exchange-rates';
import { activeAt, assetAt, EditorFrame, Field, Period, RegistryNotice } from './registry-ui';
import type { ViewContext } from './registry-ui';

const propertyTypes = { apartment: 'Appartement', house: 'Maison', building: 'Immeuble', land: 'Terrain', commercial: 'Local commercial', office: 'Bureau', garage: 'Garage', other: 'Autre' };
const propertyStatuses = { owner_occupied: 'Occupé par le propriétaire', rented: 'Loué', vacant: 'Vacant', renovation: 'En travaux', for_sale: 'En vente' };
const eventKinds: Record<string, string> = { acquisition: 'Acquisition', opening_tracking: 'Début de suivi', disposal: 'Cession complète' };
type EditorKind = 'property' | 'unit' | 'event' | 'disposal';
type PropertyRecord = Property | RentalUnit | PropertyEvent;
interface Editor { kind: EditorKind; record: PropertyRecord | null; values: Record<string, string>; available: boolean; confirmed: boolean; reason: string }

export function PropertyWorkspace({ registry, snapshot, context, onChanged, onBusyChange, onNavigateRegistry }: {
  registry: RegistrySnapshot; snapshot: PropertySnapshot; context: ViewContext;
  onChanged: () => Promise<void>; onBusyChange: (busy: boolean) => void; onNavigateRegistry?: () => void;
}) {
  const [editor, setEditor] = useState<Editor | null>(null);
  const [removal, setRemoval] = useState<RegistryRecord | null>(null);
  const [removalReason, setRemovalReason] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [showAllDates, setShowAllDates] = useState(false);
  const [selectedAsset, setSelectedAsset] = useState('');
  const [report, setReport] = useState<PropertyReport | null>(null);
  const requestId = useRef(0);
  const readOnly = context.knowledgeSequence != null;
  const correctingDisposal = editor?.kind === 'event' && editor.record !== null && editor.values.kind === 'disposal';
  const disposalAction = editor?.kind === 'disposal' || correctingDisposal;
  useEffect(() => { onBusyChange(pending); return () => onBusyChange(false); }, [pending, onBusyChange]);
  useEffect(() => {
    requestId.current += 1; setReport(null); setEditor(null); setRemoval(null); setError('');
  }, [context.date, context.knowledgeSequence, registry.knowledge_sequence, snapshot.knowledge_sequence]);
  useEffect(() => { setMessage(''); }, [context.date, context.knowledgeSequence]);
  useEffect(() => {
    if (selectedAsset && !registry.assets.some(asset => asset.id === selectedAsset)) setSelectedAsset('');
  }, [registry.assets, selectedAsset]);
  function assetName(id: string) {
    const asset = registry.assets.find(item => item.id === id);
    return asset ? assetAt(registry, asset, context.date).name : id;
  }
  function open(kind: EditorKind, record: PropertyRecord | null = null) {
    setRemoval(null); setError(''); setMessage('');
    const values: Record<string, string> = {
      asset_id: selectedAsset, property_asset_id: selectedAsset, property_type: 'apartment', city: '', address: '',
      surface_m2: '', status: 'vacant', valid_from: context.date, valid_to: '', notes: '', reference: '', name: '',
      kind: 'acquisition', effective_on: context.date, price: '', fees: '', currency: 'EUR',
    };
    if (record) for (const [key, value] of Object.entries(record)) if (typeof value === 'string' || value === null) values[key] = value ?? '';
    setEditor({ kind, record, values, available: record && 'available_for_rent' in record ? record.available_for_rent : true, confirmed: false, reason: '' });
  }
  function change(key: string, value: string) { setEditor(current => current && ({ ...current, confirmed: false, values: { ...current.values, [key]: value, ...(key === 'kind' && value === 'opening_tracking' ? { price: '', fees: '' } : {}) } })); }
  async function save(event: FormEvent) {
    event.preventDefault(); if (!editor || readOnly) return;
    const { values: v, kind } = editor;
    if (disposalAction && !editor.confirmed) return;
    setPending(true); setError(''); setMessage('');
    const meta = { id: editor.record?.id ?? null, expected_revision_id: editor.record?.revision_id ?? null, reason: editor.reason.trim() };
    try {
      if (kind === 'property') await api.saveProperty({ ...meta, asset_id: v.asset_id, property_type: v.property_type, city: v.city.trim(), address: v.address.trim(), surface_m2: v.surface_m2.trim() || null, status: v.status, valid_from: v.valid_from, valid_to: v.valid_to || null, notes: v.notes });
      else if (kind === 'unit') await api.saveRentalUnit({ ...meta, property_asset_id: v.property_asset_id, reference: v.reference.trim(), name: v.name.trim(), surface_m2: v.surface_m2.trim() || null, valid_from: v.valid_from, valid_to: v.valid_to || null, available_for_rent: editor.available, notes: v.notes });
      else if (kind === 'event') await api.savePropertyEvent({ ...meta, asset_id: correctingDisposal && editor.record && 'asset_id' in editor.record ? editor.record.asset_id : v.asset_id, kind: correctingDisposal ? 'disposal' : v.kind, effective_on: v.effective_on, price: v.price.trim() || null, fees: v.fees.trim() || null, currency: v.currency, notes: v.notes });
      else await api.disposeProperty({ asset_id: v.asset_id, effective_on: v.effective_on, price: v.price.trim() || null, fees: v.fees.trim() || null, currency: v.currency, reason: editor.reason.trim(), expected_sequence: snapshot.knowledge_sequence });
      setEditor(null); setReport(null);
      await refreshAfterCommit(onChanged, correctingDisposal ? 'La correction de cession et le recalcul des droits ont été enregistrés ensemble. Les anciennes versions restent conservées.' : kind === 'disposal' ? 'La cession et la clôture des droits ont été enregistrées ensemble. Les données historiques restent conservées.' : 'L’enregistrement immobilier a été conservé avec son historique.', setMessage);
    } catch (failure) { setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  async function remove(event: FormEvent) {
    event.preventDefault(); if (!removal || readOnly) return;
    setPending(true); setError(''); setMessage('');
    try { await api.voidRegistryRecord(removal.id, removal.revision_id, removalReason.trim()); setRemoval(null); await refreshAfterCommit(onChanged, 'L’enregistrement a été annulé ; ses anciennes versions restent conservées.', setMessage); }
    catch (failure) { setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  async function inspect(event: FormEvent) {
    event.preventDefault(); if (!selectedAsset) return;
    const selected = ++requestId.current;
    setPending(true); setError(''); setReport(null);
    try {
      const result = await api.getPropertyReport({ asset_id: selectedAsset, date: context.date, knowledge_sequence: context.knowledgeSequence ?? null });
      if (selected === requestId.current) setReport(result);
    } catch (failure) { if (selected === requestId.current) setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  function actions(kind: 'property' | 'unit' | 'event', record: PropertyRecord, label: string) {
    return <div className="row-actions"><button type="button" className="button secondary small-button" disabled={pending || readOnly} aria-label={`Modifier ${label}`} onClick={() => open(kind, record)}>Modifier</button><button type="button" className="text-button" disabled={pending || readOnly} aria-label={`Annuler ${label}`} onClick={() => { setEditor(null); setRemoval(record); setRemovalReason(''); setError(''); }}>Annuler</button></div>;
  }
  const propertyAssets = registry.assets.filter(asset => snapshot.properties.some(item => item.asset_id === asset.id));
  const propertyRows = snapshot.properties.filter(item => (!selectedAsset || item.asset_id === selectedAsset) && (showAllDates || activeAt(item.valid_from, item.valid_to, context.date)));
  const unitRows = snapshot.units.filter(item => (!selectedAsset || item.property_asset_id === selectedAsset) && (showAllDates || activeAt(item.valid_from, item.valid_to, context.date)));
  const eventRows = snapshot.events.filter(item => (!selectedAsset || item.asset_id === selectedAsset) && (showAllDates || item.effective_on <= context.date));
  const money = (amount: string | null, currency = 'EUR') => amount === null ? 'Non calculable' : `${amount} ${currency}`;
  const v = editor?.values;
  function textField(key: string, label: string, required = false, type = 'text', hint?: string) {
    if (!editor || !v) return null;
    const prefix = editor.kind === 'event' ? 'lifecycle' : editor.kind;
    const id = `${prefix}-${key}`;
    return <Field id={id} label={label} hint={hint}><input id={id} type={type} required={required} value={v[key] ?? ''} inputMode={['price', 'fees', 'surface_m2'].includes(key) ? 'decimal' : undefined} maxLength={type === 'date' ? undefined : 1000} onChange={event => change(key, event.currentTarget.value)} /></Field>;
  }
  function selectField(key: string, label: string, options: { id: string; label: string }[], disabled = false) {
    if (!editor || !v) return null;
    const prefix = editor.kind === 'event' ? 'lifecycle' : editor.kind;
    const id = `${prefix}-${key}`;
    return <Field id={id} label={label}><select id={id} value={v[key] ?? ''} required disabled={disabled} onChange={event => change(key, event.currentTarget.value)}><option value="">Choisir…</option>{options.map(option => <option key={option.id} value={option.id}>{option.label}</option>)}</select></Field>;
  }
  const propertyLabel = (id: string) => `${assetName(id)} · ${id}`;
  const assetOptions = registry.assets.filter(asset => !asset.archived || asset.id === v?.asset_id).map(asset => ({ id: asset.id, label: propertyLabel(asset.id) }));
  const propertyOptions = propertyAssets.map(asset => ({ id: asset.id, label: propertyLabel(asset.id) }));

  return <div className="registry-workspace">
    <RegistryNotice>Chaque fiche complète un actif du registre. Les unités locatives sont des subdivisions techniques : elles n’ajoutent aucune valeur au patrimoine. Les fiches décrivent le bien entier ; le contrôle des droits familiaux et individuels reste dans « Patrimoine ».</RegistryNotice>
    {readOnly && <RegistryNotice><strong>Lecture seule historique.</strong> Connaissances à la séquence {context.knowledgeSequence}. Revenez aux connaissances actuelles pour modifier l’immobilier.</RegistryNotice>}
    {error && <RegistryNotice error><strong>Opération impossible.</strong><p>{error}</p><p>Une révision concurrente doit être rechargée avant correction. Les valeurs saisies sont conservées dans le formulaire.</p><button type="button" className="button secondary" disabled={pending} onClick={() => { setPending(true); void onChanged().then(() => { setEditor(null); setRemoval(null); setError(''); }).catch(failure => setError(errorMessage(failure))).finally(() => setPending(false)); }}>Recharger l’immobilier</button></RegistryNotice>}
    {message && <RegistryNotice>{message}</RegistryNotice>}
    <section className="card registry-filters"><Field id="property-filter" label="Bien consulté"><select id="property-filter" value={selectedAsset} disabled={pending} onChange={event => { setSelectedAsset(event.currentTarget.value); requestId.current += 1; setReport(null); }}><option value="">Tous les biens</option>{propertyAssets.map(asset => <option key={asset.id} value={asset.id}>{propertyLabel(asset.id)}</option>)}</select></Field><label className="checkbox-label"><input type="checkbox" checked={showAllDates} onChange={event => setShowAllDates(event.currentTarget.checked)} />Afficher toutes les périodes et les événements futurs</label></section>
    <section className="card registry-table"><div className="section-title"><div><h2>Fiches immobilières</h2><p>Les périodes finissent à une date exclusive. Le statut « Vendu » résulte d’une cession complète.</p></div><button type="button" className="button" disabled={pending || readOnly || registry.assets.length === 0} onClick={() => open('property')}>Ajouter une fiche immobilière</button></div>
      {registry.assets.length === 0 && <p>Créez d’abord l’actif, ses propriétaires et ses estimations dans « Patrimoine ».{onNavigateRegistry && <button type="button" className="button secondary" disabled={pending} onClick={onNavigateRegistry}>Ouvrir le registre patrimonial</button>}</p>}
      {propertyRows.length ? <div className="table-scroll"><table><caption className="sr-only">Fiches immobilières connues</caption><thead><tr><th scope="col">Bien et adresse</th><th scope="col">Type et surface</th><th scope="col">Situation et période</th><th scope="col">Actions</th></tr></thead><tbody>{propertyRows.map(item => {
        const sold = snapshot.events.some(event => event.asset_id === item.asset_id && event.kind === 'disposal' && event.effective_on <= context.date);
        return <tr key={item.id}><td><strong>{assetName(item.asset_id)}</strong><small>{item.city || 'Ville non renseignée'} · {item.address || 'Adresse non renseignée'}</small>{item.notes && <small>{item.notes}</small>}</td><td>{propertyTypes[item.property_type as keyof typeof propertyTypes] ?? item.property_type}<small>{item.surface_m2 === null ? 'Surface non renseignée' : `${item.surface_m2} m²`}</small></td><td>{sold ? 'Vendu' : propertyStatuses[item.status as keyof typeof propertyStatuses] ?? item.status}<small><Period from={item.valid_from} to={item.valid_to} /></small></td><td>{actions('property', item, `la fiche de ${assetName(item.asset_id)}`)}</td></tr>;
      })}</tbody></table></div> : <p className="muted">Aucune fiche immobilière dans les dates affichées.</p>}
      {onNavigateRegistry && registry.assets.length > 0 && <button type="button" className="text-button" disabled={pending} onClick={onNavigateRegistry}>Gérer les actifs, droits et estimations dans « Patrimoine »</button>}
    </section>
    <section className="card registry-table"><div className="section-title"><div><h2>Unités locatives</h2><p>Un bien simple utilise une unité ; un immeuble peut en avoir plusieurs.</p></div><button type="button" className="button secondary" disabled={pending || readOnly || propertyAssets.length === 0} onClick={() => open('unit')}>Ajouter une unité</button></div>
      {unitRows.length ? <div className="table-scroll"><table><caption className="sr-only">Unités techniques de location</caption><thead><tr><th scope="col">Unité et bien</th><th scope="col">Disponibilité</th><th scope="col">Période de suivi</th><th scope="col">Actions</th></tr></thead><tbody>{unitRows.map(item => <tr key={item.id}><td><strong>{item.reference} · {item.name}</strong><small>{assetName(item.property_asset_id)}{item.surface_m2 !== null ? ` · ${item.surface_m2} m²` : ''}</small>{item.notes && <small>{item.notes}</small>}</td><td>{item.available_for_rent ? 'Disponible à la location' : 'Indisponible à la location'}</td><td><Period from={item.valid_from} to={item.valid_to} /></td><td>{actions('unit', item, `l’unité ${item.reference}`)}</td></tr>)}</tbody></table></div> : <p className="muted">Aucune unité dans les dates affichées.</p>}
    </section>
    <section className="card registry-table"><div className="section-title"><div><h2>Acquisitions et cessions</h2><p>Les coûts restent distincts des estimations. Les prix et frais inconnus ne deviennent pas nuls.</p></div><div className="button-row"><button type="button" className="button secondary" disabled={pending || readOnly || propertyAssets.length === 0} onClick={() => open('event')}>Enregistrer une acquisition</button><button type="button" className="button secondary" disabled={pending || readOnly || propertyAssets.length === 0} onClick={() => open('disposal')}>Enregistrer une cession</button></div></div>
      {eventRows.length ? <div className="table-scroll"><table><caption className="sr-only">Historique du cycle de vie immobilier</caption><thead><tr><th scope="col">Bien et événement</th><th scope="col">Date</th><th scope="col">Prix et frais</th><th scope="col">Actions</th></tr></thead><tbody>{[...eventRows].sort((a, b) => a.effective_on.localeCompare(b.effective_on)).map(item => <tr key={item.id}><td>{assetName(item.asset_id)}<small>{eventKinds[item.kind] ?? item.kind}</small>{item.notes && <small>{item.notes}</small>}</td><td>{item.effective_on}</td><td>Prix : {item.price === null ? 'Inconnu' : `${item.price} ${item.currency}`}<small>Frais : {item.fees === null ? 'Inconnus' : `${item.fees} ${item.currency}`}</small></td><td>{item.kind === 'disposal' ? <><small>Clôture atomique des droits conservée dans l’historique.</small><button type="button" className="button secondary small-button" disabled={pending || readOnly} onClick={() => open('event', item)}>Corriger la cession</button></> : actions('event', item, `l’événement du ${item.effective_on}`)}</td></tr>)}</tbody></table></div> : <p className="muted">Aucun événement immobilier dans les dates affichées.</p>}
    </section>
    {editor && !readOnly && v && <EditorFrame title={correctingDisposal ? 'Corriger une cession complète' : editor.kind === 'property' ? `${editor.record ? 'Corriger' : 'Créer'} une fiche immobilière` : editor.kind === 'unit' ? `${editor.record ? 'Corriger' : 'Créer'} une unité locative` : editor.kind === 'event' ? `${editor.record ? 'Corriger' : 'Enregistrer'} un événement immobilier` : 'Préparer une cession complète'} pending={pending} onCancel={() => setEditor(null)}><form onSubmit={event => { void save(event); }}><fieldset disabled={pending}>
      {editor.kind === 'property' && <>
        {selectField('asset_id', 'Actif immobilier existant', assetOptions)}
        <div className="form-grid">{selectField('property_type', 'Type de bien', Object.entries(propertyTypes).map(([id, label]) => ({ id, label })))}{selectField('status', 'Situation du bien', Object.entries(propertyStatuses).map(([id, label]) => ({ id, label })))}</div>
        <div className="form-grid">{textField('city', 'Ville')}{textField('address', 'Adresse')}</div>{textField('surface_m2', 'Surface en m²', false, 'text', 'Décimal exact ; laisser vide si inconnue.')}
      </>}
      {editor.kind === 'unit' && <>
        {selectField('property_asset_id', 'Bien de l’unité', propertyOptions)}<div className="form-grid">{textField('reference', 'Référence de l’unité', true)}{textField('name', 'Nom de l’unité', true)}</div>{textField('surface_m2', 'Surface de l’unité en m²')}
        <label className="checkbox-label"><input id="unit-available_for_rent" type="checkbox" checked={editor.available} onChange={event => { const available = event.currentTarget.checked; setEditor(current => current && ({ ...current, available })); }} />Unité disponible à la location dans cette période</label>
      </>}
      {(editor.kind === 'property' || editor.kind === 'unit') && <div className="form-grid">{textField('valid_from', 'Début de validité', true, 'date')}{textField('valid_to', 'Fin de validité (exclue)', false, 'date')}</div>}
      {(editor.kind === 'event' || editor.kind === 'disposal') && <>
        {selectField('asset_id', 'Bien concerné', propertyOptions, correctingDisposal)}
        {editor.kind === 'event' && selectField('kind', 'Nature de l’événement', correctingDisposal ? [{ id: 'disposal', label: 'Cession complète' }] : [{ id: 'acquisition', label: 'Acquisition documentée' }, { id: 'opening_tracking', label: 'Début du suivi sans acquisition supposée' }], correctingDisposal)}
        {textField('effective_on', disposalAction ? 'Date de cession' : 'Date de l’événement', true, 'date')}
        {editor.kind === 'event' && v.kind === 'opening_tracking' ? <p className="field-hint">Le début du suivi ne suppose ni prix ni date d’acquisition. Aucun coût d’achat n’est déduit de cet événement.</p> : <div className="form-grid">{textField('price', disposalAction ? 'Prix de cession' : 'Prix d’achat', false, 'text', 'Vide = prix inconnu ; zéro doit être saisi explicitement.')}{textField('fees', disposalAction ? 'Frais de cession' : 'Frais d’acquisition', false, 'text', 'Vide = frais inconnus ; zéro doit être saisi explicitement.')}{selectField('currency', 'Devise des prix et frais', displayCurrencies.map(code => ({ id: code, label: code })))}</div>}
      </>}
      {editor.kind !== 'disposal' && <Field id={`${editor.kind === 'event' ? 'lifecycle' : editor.kind}-notes`} label="Notes"><textarea id={`${editor.kind === 'event' ? 'lifecycle' : editor.kind}-notes`} value={v.notes} maxLength={10000} onChange={event => change('notes', event.currentTarget.value)} /></Field>}
      <Field id={`${editor.kind === 'event' ? 'lifecycle' : editor.kind}-reason`} label="Motif de l’enregistrement ou de la correction"><textarea id={`${editor.kind === 'event' ? 'lifecycle' : editor.kind}-reason`} required maxLength={2000} value={editor.reason} onChange={event => { const reason = event.currentTarget.value; setEditor(current => current && ({ ...current, reason })); }} /></Field>
      {editor.kind === 'disposal' && <><RegistryNotice>Cette opération clôture ensemble les droits de propriété suivis à la date de cession. Le bien, ses estimations et ses relations historiques sont conservés. Aucun encaissement bancaire n’est créé. Vérifiez les périodes de baux et les créances locatives séparément.</RegistryNotice><label className="checkbox-label"><input id="disposal-confirm" type="checkbox" checked={editor.confirmed} onChange={event => { const confirmed = event.currentTarget.checked; setEditor(current => current && ({ ...current, confirmed })); }} />Je confirme la cession complète et la clôture des droits suivis.</label></>}
      {correctingDisposal && <><RegistryNotice>Corriger la date de cession recalcule dans la même opération les périodes des droits fermés par cette cession. Le bien concerné reste identique. La correction du prix, des frais ou des notes conserve aussi les anciennes versions. Aucun mouvement bancaire n’est créé.</RegistryNotice><label className="checkbox-label"><input id="lifecycle-disposal-confirm" type="checkbox" checked={editor.confirmed} onChange={event => { const confirmed = event.currentTarget.checked; setEditor(current => current && ({ ...current, confirmed })); }} />Je confirme la correction de la cession et le recalcul des droits suivis.</label></>}
      <button type="submit" className={`button${disposalAction ? ' danger' : ''}`} disabled={pending || !editor.reason.trim() || (disposalAction && !editor.confirmed)}>{correctingDisposal ? 'Confirmer la correction de cession' : editor.kind === 'disposal' ? 'Confirmer la cession complète' : editor.record ? 'Enregistrer la correction immobilière' : editor.kind === 'property' ? 'Enregistrer la fiche immobilière' : editor.kind === 'unit' ? 'Enregistrer l’unité' : 'Enregistrer l’événement immobilier'}</button>
    </fieldset></form></EditorFrame>}
    {removal && !readOnly && <section className="card restore-review"><h2>Confirmer l’annulation de l’enregistrement immobilier</h2><p>Les anciennes versions restent conservées. Si d’autres enregistrements dépendent de cet objet, l’annulation sera refusée.</p><form onSubmit={event => { void remove(event); }}><fieldset disabled={pending}><Field id="property-removal-reason" label="Motif de l’annulation"><textarea id="property-removal-reason" required value={removalReason} maxLength={2000} onChange={event => setRemovalReason(event.currentTarget.value)} /></Field><div className="button-row"><button type="submit" className="button danger" disabled={!removalReason.trim()}>Confirmer l’annulation immobilière</button><button type="button" className="button secondary" onClick={() => setRemoval(null)}>Conserver l’enregistrement</button></div></fieldset></form></section>}
    <section className="card property-report"><h2>Coût et valeur du bien</h2><p>Le calcul décrit le bien entier à la date {context.date}. Le coût ne remplace jamais une estimation manquante.</p><form onSubmit={event => { void inspect(event); }}><fieldset disabled={pending}><Field id="property-report-asset" label="Bien à contrôler"><select id="property-report-asset" required value={selectedAsset} onChange={event => { setSelectedAsset(event.currentTarget.value); requestId.current += 1; setReport(null); }}><option value="">Choisir…</option>{propertyAssets.map(asset => <option key={asset.id} value={asset.id}>{propertyLabel(asset.id)}</option>)}</select></Field><button type="submit" className="button" disabled={pending || !selectedAsset}>Contrôler la valeur du bien</button></fieldset></form>
      {report && <div className="property-result"><h3>{report.label} · {report.date}</h3><div className="summary-grid"><div><span>Coût total d’acquisition</span><strong>{money(report.acquisition_cost, report.acquisition_currency ?? '')}</strong></div><div><span>Estimation applicable</span><strong>{money(report.estimated_value_eur)}</strong></div><div><span>Écart de valeur en EUR</span><strong>{money(report.unrealized_gain_eur)}</strong></div></div><dl className="details"><div><dt>Prix d’achat</dt><dd>{report.acquisition ? money(report.acquisition.price, report.acquisition.currency) : 'Inconnu'}</dd></div><div><dt>Frais d’acquisition</dt><dd>{report.acquisition ? money(report.acquisition.fees, report.acquisition.currency) : 'Inconnus'}</dd></div><div><dt>Coût historique converti en EUR</dt><dd>{money(report.acquisition_cost_eur)}</dd></div><div><dt>Estimation d’origine et date</dt><dd>{report.valuation ? `${report.valuation.amount} ${report.valuation.currency} · ${report.valuation.effective_on} · ${report.valuation.source}` : 'Non calculable'}</dd></div><div><dt>Plus-value latente dans la devise d’origine</dt><dd>{money(report.unrealized_gain_original, report.unrealized_gain_currency ?? '')}</dd></div><div><dt>Dette restante affectée</dt><dd>{report.debt_remaining_eur === null ? 'Non calculable · couverture des financements insuffisante' : money(report.debt_remaining_eur)}</dd></div><div><dt>Valeur nette avant frais et fiscalité de cession</dt><dd>{money(report.net_before_disposal_eur)}</dd></div></dl><p className="scope-note">L’écart EUR compare le coût converti à la date d’acquisition et l’estimation convertie à la date d’observation. Il ne représente pas un rendement. Les unités ne sont jamais ajoutées à la valeur du bien.</p>
        {report.acquisition_conversions.length > 0 && <details><summary>Sources de conversion du coût historique</summary>{report.acquisition_conversions.map((conversion, index) => <FxTrace key={index} result={conversion} />)}</details>}
        {report.valuation_conversion && <details><summary>Source de conversion de l’estimation</summary><FxTrace result={report.valuation_conversion} /></details>}
        {(report.financing_conversions?.length ?? 0) > 0 && <details><summary>Sources de conversion des financements affectés</summary>{report.financing_conversions!.map((conversion, index) => <FxTrace key={index} result={conversion} />)}</details>}
        {report.disposal && <RegistryNotice>Cession enregistrée le {report.disposal.effective_on}. La fiche reste consultable ; les droits clos déterminent son exclusion des attributions courantes.</RegistryNotice>}
        {report.warnings.length > 0 && <RegistryNotice><ul>{report.warnings.map((warning, index) => <li key={`${warning.code}-${index}`}>{warning.message}</li>)}</ul></RegistryNotice>}
        <details><summary>Sources exactes du contrôle</summary><p className="hash">{report.source_ids.join(', ') || 'Aucune source applicable'}</p></details>
      </div>}
    </section>
  </div>;
}

export default PropertyWorkspace;
