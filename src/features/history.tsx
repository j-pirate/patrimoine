import { useEffect, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { api, errorMessage } from '../services/api';
import type { RegistryHistory, VariationReport } from '../services/phase4-types';
import { Field, RegistryNotice } from './registry-ui';
import type { ViewContext } from './registry-ui';

const kinds: Record<string, string> = {
  entity: 'Personne ou société', membership: 'Périmètre familial', participation: 'Participation', asset: 'Bien',
  ownership: 'Propriété', valuation: 'Estimation', company_policy: 'Méthode de société', exchange_rate: 'Taux de change',
  profile_period: 'Période de profil', document: 'Document', document_link: 'Association documentaire', workspace: 'Paramètres',
  property: 'Fiche immobilière', rental_unit: 'Unité locative', property_event: 'Acquisition ou cession',
  lease: 'Bail', lease_terms: 'Conditions de bail', rent_due: 'Échéance locative',
  rent_payment: 'Paiement locatif', rent_allocation: 'Affectation locative', rental_expense: 'Dépense immobilière',
};

export default function HistoryPage({ history, historyError, context, onChooseSequence, onReload, onBusyChange }: {
  history: RegistryHistory | null; historyError: string; context: ViewContext;
  onChooseSequence: (sequence: number | null) => void; onReload: () => Promise<void>;
  onBusyChange: (busy: boolean) => void;
}) {
  const [fromDate, setFromDate] = useState(`${context.date.slice(0, 4)}-01-01`);
  const [toDate, setToDate] = useState(context.date);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const [variation, setVariation] = useState<VariationReport | null>(null);
  const requestId = useRef(0);
  useEffect(() => { onBusyChange(pending); return () => onBusyChange(false); }, [pending, onBusyChange]);
  useEffect(() => { requestId.current += 1; setVariation(null); setError(''); setToDate(context.date); }, [context.date, context.personId, context.knowledgeSequence]);
  async function explain(event: FormEvent) {
    event.preventDefault(); setPending(true); setError(''); setVariation(null);
    const selected = ++requestId.current;
    try {
      const result = await api.explainVariation({ context: context.personId ? 'person' : 'family', person_id: context.personId,
        from_date: fromDate, to_date: toDate, knowledge_sequence: context.knowledgeSequence ?? null });
      if (selected === requestId.current) setVariation(result);
    } catch (failure) { if (selected === requestId.current) setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  return <div className="registry-workspace">
    <section className="card"><div className="section-title"><div><h2>Journal des modifications</h2><p>Retrouvez vos modifications et consultez le coffre tel qu’il apparaissait après chacune d’elles, en lecture seule.</p></div><button type="button" className="button secondary" disabled={pending} onClick={() => { void onReload(); }}>Actualiser le journal</button></div>
      <details><summary>Comprendre la date et le numéro de modification</summary><p>La date de référence choisit le jour auquel les biens, les droits et les soldes sont évalués. Le numéro de modification, aussi appelé séquence de connaissance, fixe la dernière saisie prise en compte. Une correction rétroactive conserve les versions précédentes dans le journal.</p></details>
      {historyError && <RegistryNotice error>{historyError}</RegistryNotice>}
      {history && <><p className="field-hint">Dernière séquence du coffre : {history.latest_sequence}. {context.knowledgeSequence == null ? 'Consultation avec les connaissances actuelles.' : `Lecture seule de la séquence ${context.knowledgeSequence}.`}</p>
        {history.events.length ? <div className="table-scroll"><table><caption className="sr-only">Révisions du registre et du coffre</caption><thead><tr><th scope="col">Séquence et saisie</th><th scope="col">Objet</th><th scope="col">Motif</th><th scope="col">Consultation</th></tr></thead><tbody>{[...history.events].sort((a, b) => b.sequence - a.sequence).map((event, index) => <tr key={`${event.sequence}-${event.revision_id}-${index}`}>
          <td><strong>{event.sequence}</strong><small>{event.recorded_at}</small></td><td>{event.label || event.object_id}<small>{kinds[event.kind] || 'Autre modification'}{event.is_void ? ' · annulation' : ''}</small><details><summary>Identifiants</summary><p className="hash">Objet : {event.object_id}<br />Révision : {event.revision_id || 'Sans révision métier'}</p></details></td><td>{event.reason}</td><td><button type="button" className="button secondary small-button" disabled={pending || context.knowledgeSequence === event.sequence} onClick={() => onChooseSequence(event.sequence)}>Consulter la séquence {event.sequence}</button></td>
        </tr>)}</tbody></table></div> : <p className="muted">Aucune modification métier enregistrée dans ce coffre.</p>}
        {context.knowledgeSequence != null && <button type="button" className="button" disabled={pending} onClick={() => onChooseSequence(null)}>Revenir aux connaissances actuelles</button>}
      </>}
    </section>
    <section className="card"><h2>Comparer les estimations du registre</h2><p>Choisissez deux dates pour voir comment les estimations enregistrées ont évolué.</p><details><summary>Ce que la comparaison couvre</summary><p>Ce calcul porte sur les estimations du registre. Le tableau de bord rassemble aussi les lots de métaux valorisés, la trésorerie et les dettes. Une variation d’estimation ne mesure pas un rendement.</p></details>
      <form onSubmit={event => { void explain(event); }}><fieldset disabled={pending}><div className="form-grid"><Field id="variation-from" label="Début de comparaison"><input id="variation-from" type="date" required value={fromDate} onChange={event => { setFromDate(event.currentTarget.value); setVariation(null); }} /></Field><Field id="variation-to" label="Fin de comparaison"><input id="variation-to" type="date" required value={toDate} onChange={event => { setToDate(event.currentTarget.value); setVariation(null); }} /></Field></div><button type="submit" className="button" disabled={pending}>{pending ? 'Comparaison en cours…' : 'Expliquer la variation'}</button></fieldset></form>
      {error && <RegistryNotice error>{error}</RegistryNotice>}
      {variation && <div className="variation-result"><div className="summary-grid"><div><span>Sous-total connu initial</span><strong>{variation.opening.status === 'non_calculable' ? 'Non calculable' : `${variation.opening.gross_eur} EUR`}</strong></div><div><span>Sous-total connu final</span><strong>{variation.closing.status === 'non_calculable' ? 'Non calculable' : `${variation.closing.gross_eur} EUR`}</strong></div><div><span>Variation des sous-totaux connus</span><strong>{variation.delta_eur} EUR</strong></div></div>
        <p>Comparaison du {variation.from_date} au {variation.to_date}, calculée avec les connaissances de la séquence {variation.knowledge_sequence}. Actualiser le journal ne recalcule pas cette comparaison.</p>
        <p className="scope-note">Ordre conventionnel : périmètre et méthode, droits et actifs, valorisations, puis change. Les effets croisés sont affectés à la dernière étape concernée. Les apports ne sont pas présentés comme une performance.</p>
        <div className="table-scroll"><table><caption className="sr-only">Décomposition de la variation du sous-total</caption><thead><tr><th scope="col">Composante</th><th scope="col">Effet EUR</th><th scope="col">Références du calcul</th></tr></thead><tbody>{variation.steps.map(step => <tr key={step.code}><td>{step.label}</td><td>{step.amount_eur === null ? 'Non attribuable' : `${step.amount_eur} EUR`}</td><td><details><summary>Hypothèses et sources</summary><p>Dates figées : périmètre {step.dates.membership_date}, méthode {step.dates.method_date}, droits {step.dates.rights_date}, estimation {step.dates.valuation_date}, change {step.dates.fx_date}.</p><p>{step.amount_eur === null ? 'Données de référence insuffisantes.' : `Sous-total avant ${step.before_eur} EUR ; après ${step.after_eur} EUR.`}</p><p className="hash">{step.source_ids.join(', ') || 'Aucune source applicable'}</p>{step.warnings.length > 0 && <ul>{step.warnings.map((warning, index) => <li key={index}>{warning.message}</li>)}</ul>}</details></td></tr>)}</tbody></table></div>
        <p><strong>Résidu non expliqué : {variation.residual_eur === null ? 'Non calculable' : `${variation.residual_eur} EUR`}</strong></p>
        {variation.warnings.length > 0 && <RegistryNotice><ul>{variation.warnings.map((warning, index) => <li key={index}>{warning.message}</li>)}</ul></RegistryNotice>}
      </div>}
    </section>
  </div>;
}
