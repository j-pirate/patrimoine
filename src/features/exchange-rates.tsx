import { useEffect, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { api, displayCurrencies, errorMessage } from '../services/api';
import type { ExchangeRate, FxResult } from '../services/phase4-types';
import type { RegistrySnapshot } from '../services/registry-types';
import { EditorFrame, Field, RegistryNotice } from './registry-ui';
import type { ViewContext } from './registry-ui';

const methods: Record<FxResult['method'], string> = { identity: 'Même devise', direct: 'Cours direct', inverse: 'Cours inversé', cross: 'Conversion croisée', missing: 'Cours indisponible' };

export function FxTrace({ result }: { result: FxResult }) {
  return <div className="fx-trace">
    <p><strong>{methods[result.method]}</strong> · date de conversion : {result.date}</p>
    {result.exact_rate && <p>Taux exact utilisé : {result.exact_rate.num}/{result.exact_rate.den} {result.to_currency} pour 1 {result.from_currency}</p>}
    {result.oldest_rate_date && <p>Cours le plus ancien du chemin : {result.oldest_rate_date}{result.age_days !== null ? ` · ancienneté ${result.age_days} jour(s)` : ''}</p>}
    {result.legs.length > 0 && <ol>{result.legs.map((leg, index) => <li key={`${leg.revision_id}-${index}`}>
      1 {leg.base_currency} = {leg.rate} {leg.quote_currency} · applicable au {leg.effective_on}{leg.inverted ? ' · inversion utilisée' : ''}
      <small>Source : {leg.source || 'Non renseignée'} · révision {leg.revision_id}</small>
    </li>)}</ol>}
    {result.warnings.length > 0 && <RegistryNotice><ul>{result.warnings.map((warning, index) => <li key={`${warning.code}-${index}`}>{warning.message}</li>)}</ul></RegistryNotice>}
  </div>;
}

interface RateEditor { record: ExchangeRate | null; base: string; quote: string; rate: string; date: string; source: string; notes: string; reason: string }

export default function ExchangeRatesPage({ registry, context, onChanged, onBusyChange }: {
  registry: RegistrySnapshot; context: ViewContext; onChanged: () => Promise<void>; onBusyChange: (busy: boolean) => void;
}) {
  const [editor, setEditor] = useState<RateEditor | null>(null);
  const [removal, setRemoval] = useState<ExchangeRate | null>(null);
  const [removalReason, setRemovalReason] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [amount, setAmount] = useState('');
  const [fromCurrency, setFromCurrency] = useState('TND');
  const [toCurrency, setToCurrency] = useState('EUR');
  const [result, setResult] = useState<FxResult | null>(null);
  const requestId = useRef(0);
  const readOnly = context.knowledgeSequence != null;
  useEffect(() => { onBusyChange(pending); return () => onBusyChange(false); }, [pending, onBusyChange]);
  useEffect(() => { requestId.current += 1; setResult(null); setEditor(null); setRemoval(null); setError(''); }, [context.date, context.knowledgeSequence, registry.knowledge_sequence]);
  function edit(record: ExchangeRate | null) {
    setError(''); setMessage(''); setRemoval(null);
    setEditor({ record, base: record?.base_currency || 'TND', quote: record?.quote_currency || 'EUR', rate: record?.rate || '', date: record?.effective_on || context.date, source: record?.source || '', notes: record?.notes || '', reason: '' });
  }
  function change(key: keyof Omit<RateEditor, 'record'>, value: string) { setEditor(current => current && ({ ...current, [key]: value })); }
  async function save(event: FormEvent) {
    event.preventDefault(); if (!editor || readOnly) return;
    setPending(true); setError(''); setMessage('');
    try {
      await api.saveExchangeRate({ id: editor.record?.id || null, expected_revision_id: editor.record?.revision_id || null, reason: editor.reason.trim(), base_currency: editor.base, quote_currency: editor.quote, rate: editor.rate.trim(), effective_on: editor.date, source: editor.source.trim() || null, notes: editor.notes });
      setEditor(null); await onChanged(); setMessage('Le taux a été enregistré. Les anciennes révisions restent conservées.');
    } catch (failure) { setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  async function remove(event: FormEvent) {
    event.preventDefault(); if (!removal || readOnly) return;
    setPending(true); setError(''); setMessage('');
    try { await api.voidRegistryRecord(removal.id, removal.revision_id, removalReason.trim()); setRemoval(null); await onChanged(); setMessage('Le taux a été annulé avec conservation de son historique.'); }
    catch (failure) { setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  async function convert(event: FormEvent) {
    event.preventDefault(); setPending(true); setError(''); setResult(null);
    const selected = ++requestId.current;
    try {
      const converted = await api.convertCurrency({ amount: amount.trim(), from_currency: fromCurrency, to_currency: toCurrency, date: context.date, knowledge_sequence: context.knowledgeSequence ?? null });
      if (selected === requestId.current) setResult(converted);
    } catch (failure) { if (selected === requestId.current) setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  const currencySelect = (id: string, value: string, onChange: (value: string) => void) => <select id={id} value={value} onChange={event => onChange(event.currentTarget.value)}>{displayCurrencies.map(code => <option key={code} value={code}>{code}</option>)}</select>;
  return <div className="registry-workspace">
    <RegistryNotice>Les taux sont saisis manuellement et conservés localement. Convention : <strong>1 unité de devise de départ = taux unités de devise d’arrivée</strong>. Pour la consolidation, privilégiez les cotations vers EUR. Les caisses physiques et leurs mouvements se gèrent dans l’onglet « Caisses physiques ».</RegistryNotice>
    {readOnly && <RegistryNotice><strong>Lecture seule historique.</strong> Vous consultez les données connues à la séquence {context.knowledgeSequence}. Revenez aux connaissances actuelles pour saisir ou corriger un taux.</RegistryNotice>}
    {error && <RegistryNotice error><strong>Opération impossible.</strong><p>{error}</p><p>En cas de conflit de révision, rechargez les connaissances actuelles puis rouvrez le taux. La saisie n’a remplacé aucune version plus récente.</p><button type="button" className="button secondary" disabled={pending} onClick={() => { setPending(true); void onChanged().then(() => { setEditor(null); setRemoval(null); setError(''); }).catch(failure => setError(errorMessage(failure))).finally(() => setPending(false)); }}>Recharger les taux</button></RegistryNotice>}
    {message && <RegistryNotice>{message}</RegistryNotice>}
    <section className="card"><div className="section-title"><div><h2>Taux de change manuels</h2><p>Les observations futures sont conservées, mais ne servent jamais à convertir une date passée.</p></div><button type="button" className="button" disabled={pending || readOnly} onClick={() => edit(null)}>Ajouter un taux</button></div>
      {registry.exchange_rates.length ? <div className="table-scroll"><table><caption className="sr-only">Observations manuelles des taux</caption><thead><tr><th scope="col">Cotation</th><th scope="col">Application</th><th scope="col">Source et notes</th><th scope="col">Actions</th></tr></thead><tbody>{registry.exchange_rates.map(rate => <tr key={rate.id}>
        <td>1 {rate.base_currency} = <strong>{rate.rate} {rate.quote_currency}</strong></td><td><time dateTime={rate.effective_on}>{rate.effective_on}</time>{rate.effective_on > context.date && <small>Futur à la date de référence</small>}</td><td>{rate.source || 'Source non renseignée'}{rate.notes && <small>{rate.notes}</small>}</td>
        <td><div className="row-actions"><button type="button" className="button secondary small-button" disabled={pending || readOnly} onClick={() => edit(rate)} aria-label={`Modifier le taux ${rate.base_currency}/${rate.quote_currency} du ${rate.effective_on}`}>Modifier</button><button type="button" className="text-button" disabled={pending || readOnly} onClick={() => { setRemoval(rate); setRemovalReason(''); setEditor(null); }}>Annuler le taux</button></div></td>
      </tr>)}</tbody></table></div> : <p className="muted">Aucun taux enregistré dans cet état de connaissance. Ajoutez une observation pour valoriser les devises étrangères.</p>}
    </section>
    {editor && !readOnly && <EditorFrame title={editor.record ? 'Corriger un taux manuel' : 'Saisir un taux manuel'} pending={pending} onCancel={() => setEditor(null)}><form onSubmit={event => { void save(event); }}><fieldset disabled={pending}>
      <div className="form-grid"><Field id="rate-base" label="Devise de départ">{currencySelect('rate-base', editor.base, value => change('base', value))}</Field><Field id="rate-quote" label="Devise d’arrivée">{currencySelect('rate-quote', editor.quote, value => change('quote', value))}</Field></div>
      <div className="form-grid"><Field id="rate-value" label="Taux manuel" hint="Nombre d’unités d’arrivée pour 1 unité de départ ; montant strictement positif, virgule ou point."><input id="rate-value" required inputMode="decimal" value={editor.rate} onChange={event => change('rate', event.currentTarget.value)} maxLength={100} /></Field><Field id="rate-date" label="Date d’application"><input id="rate-date" required type="date" value={editor.date} onChange={event => change('date', event.currentTarget.value)} /></Field></div>
      <Field id="rate-source" label="Source (facultative)"><input id="rate-source" value={editor.source} maxLength={1000} onChange={event => change('source', event.currentTarget.value)} /></Field><Field id="rate-notes" label="Notes"><textarea id="rate-notes" value={editor.notes} maxLength={10000} onChange={event => change('notes', event.currentTarget.value)} /></Field>
      <Field id="rate-reason" label="Motif de l’enregistrement ou de la correction"><textarea id="rate-reason" required value={editor.reason} maxLength={1000} onChange={event => change('reason', event.currentTarget.value)} /></Field>
      <p className="field-hint">Une nouvelle date crée une observation. Corriger ce taux remplace uniquement sa révision dans les connaissances actuelles, sans effacer son ancienne valeur.</p><button type="submit" className="button" disabled={!editor.reason.trim()}>{editor.record ? 'Enregistrer la correction' : 'Enregistrer le taux'}</button>
    </fieldset></form></EditorFrame>}
    {removal && !readOnly && <section className="card restore-review"><h2>Confirmer l’annulation du taux</h2><p>Le taux du {removal.effective_on} ({removal.base_currency}/{removal.quote_currency}) ne sera plus retenu dans les connaissances actuelles. Son ancienne révision restera consultable.</p><form onSubmit={event => { void remove(event); }}><fieldset disabled={pending}><Field id="rate-removal-reason" label="Motif de l’annulation"><textarea id="rate-removal-reason" required value={removalReason} onChange={event => setRemovalReason(event.currentTarget.value)} /></Field><div className="button-row"><button type="submit" className="button danger" disabled={!removalReason.trim()}>Confirmer l’annulation du taux</button><button type="button" className="button secondary" onClick={() => setRemoval(null)}>Conserver le taux</button></div></fieldset></form></section>}
    <section className="card"><h2>Vérifier une conversion</h2><p>Date économique : {context.date}. Le moteur retient les cours applicables à cette date et connus dans le contexte sélectionné.</p><form onSubmit={event => { void convert(event); }}><fieldset disabled={pending}><div className="form-grid"><Field id="fx-amount" label="Montant à convertir"><input id="fx-amount" required inputMode="decimal" value={amount} onChange={event => setAmount(event.currentTarget.value)} /></Field><Field id="fx-from" label="Devise du montant">{currencySelect('fx-from', fromCurrency, setFromCurrency)}</Field><Field id="fx-to" label="Devise de destination">{currencySelect('fx-to', toCurrency, setToCurrency)}</Field></div><button type="submit" className="button" disabled={pending || !amount.trim()}>Vérifier la conversion</button></fieldset></form>
      {result && <div className="fx-result"><div className="control-result"><span>{result.amount_original} {result.from_currency}</span><strong>{result.amount_converted === null ? 'Conversion impossible' : `${result.amount_converted} ${result.to_currency}`}</strong></div><FxTrace result={result} /></div>}
    </section>
  </div>;
}
