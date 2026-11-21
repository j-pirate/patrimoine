import { useEffect, useRef, useState } from 'react';
import type { Dispatch, FormEvent, ReactNode, SetStateAction } from 'react';
import { refreshAfterCommit } from '../services/mutations';
import { api, displayCurrencies, errorMessage } from '../services/api';
import type { Asset, CompanyPolicy, ConsolidationReport, Entity, Fraction, Membership, Ownership, Participation, RegistryRecord, RegistrySnapshot, Valuation } from '../services/registry-types';
import type { ProfilePeriod } from '../services/phase4-types';
import { FxTrace } from './exchange-rates';
import { activeAt, assetAt, entityAt, EditorFrame, Field, Period, RegistryNotice, useFocusedRegion } from './registry-ui';
import type { ViewContext } from './registry-ui';

type Page = 'family' | 'companies' | 'assets';
type EditorKind = 'person' | 'company' | 'membership' | 'participation' | 'asset' | 'ownership' | 'asset_valuation' | 'company_valuation' | 'company_policy' | 'profile_period';
type RecordItem = Entity | Membership | Participation | Asset | Ownership | Valuation | CompanyPolicy | ProfilePeriod;
interface Editor { kind: EditorKind; record: RecordItem | null; values: Record<string, string>; reason: string }
interface Confirmation { action: 'archive' | 'unarchive' | 'void'; kind: EditorKind; record: RecordItem }
const countries = [['FR', 'France'], ['TN', 'Tunisie'], ['CH', 'Suisse'], ['AE', 'Émirats arabes unis']] as const;
const editorTitles: Record<EditorKind, string> = {
  person: 'Personne physique', company: 'Société', membership: 'Appartenance au périmètre familial',
  participation: 'Participation dans une société', asset: 'Actif générique', ownership: 'Propriété d’un actif',
  asset_valuation: 'Estimation d’un actif', company_valuation: 'Valeur globale des capitaux propres',
  company_policy: 'Méthode de consolidation d’une société', profile_period: 'Période de profil descriptif',
};

function fractionLabel(value: Fraction | null): string { return value ? `${value.num}/${value.den}` : 'Inconnu'; }

function makeEditor(kind: EditorKind, record: RecordItem | null, context: ViewContext): Editor {
  const values: Record<string, string> = {
    kind: kind === 'company' ? 'company' : 'person', display_name: '', first_name: '', last_name: '',
    birth_date: '', residence_country: '', preferred_currency: 'EUR', activity: '', notes: '', archived: 'no',
    person_id: context.personId || '', valid_from: context.date, valid_to: '', relationship: '',
    owner_entity_id: '', company_id: '', name: '', country: '', category: '', asset_id: '',
    target_id: '', target_kind: kind === 'profile_period' ? 'entity' : kind === 'company_valuation' ? 'company' : 'asset',
    amount: '', currency: 'EUR', effective_on: context.date, source: '', mode: 'detailed',
    coverage_complete: 'no', liability_qualified: 'no', profile_revision_id: '',
  };
  if (record) {
    for (const [key, value] of Object.entries(record)) {
      if (typeof value === 'boolean') values[key] = value ? 'yes' : 'no';
      else if (typeof value === 'string') values[key] = value;
      else if (value === null) values[key] = '';
    }
  }
  for (const key of ['legal_share', 'economic_share', 'voting_share'] as const) {
    const fraction = record && key in record ? (record as Participation)[key] : null;
    values[`${key}_known`] = fraction ? 'yes' : 'no';
    values[`${key}_num`] = fraction?.num || '';
    values[`${key}_den`] = fraction?.den || '';
  }
  return { kind, record, values, reason: '' };
}

function periodIncluded(item: { valid_from: string; valid_to: string | null }, date: string, allPeriods: boolean) {
  return allPeriods || activeAt(item.valid_from, item.valid_to, date);
}

function reachableEntities(registry: RegistrySnapshot, context: ViewContext): Set<string> {
  const reachable = new Set<string>(context.personId ? [context.personId] : registry.memberships
    .filter(item => activeAt(item.valid_from, item.valid_to, context.date)).map(item => item.person_id));
  let changed = true;
  while (changed) {
    changed = false;
    for (const edge of registry.participations) {
      if (activeAt(edge.valid_from, edge.valid_to, context.date) && reachable.has(edge.owner_entity_id) && !reachable.has(edge.company_id)) {
        reachable.add(edge.company_id); changed = true;
      }
    }
  }
  return reachable;
}

export default function RegistryPage({ page, registry, context, onChanged, onBusyChange }: {
  page: Page; registry: RegistrySnapshot; context: ViewContext;
  onChanged: () => Promise<void>; onBusyChange: (busy: boolean) => void;
}) {
  const [editor, setEditor] = useState<Editor | null>(null);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [confirmationReason, setConfirmationReason] = useState('');
  const [pending, setPending] = useState(false);
  const [consolidationBusy, setConsolidationBusy] = useState(false);
  const confirmationFocus = useFocusedRegion(!!confirmation);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [viewed, setViewed] = useState<Entity | Asset | null>(null);
  const viewFocus = useFocusedRegion(viewed !== null);
  const [allRegistry, setAllRegistry] = useState(false);
  const [allPeriods, setAllPeriods] = useState(false);
  const [showArchived, setShowArchived] = useState(false);
  const editorRef = useRef<HTMLDivElement>(null);
  const readOnly = context.knowledgeSequence != null;
  useEffect(() => { onBusyChange(pending || consolidationBusy); return () => onBusyChange(false); }, [pending, consolidationBusy, onBusyChange]);
  useEffect(() => { setEditor(null); setConfirmation(null); setError(''); setMessage(''); }, [page, context.knowledgeSequence]);
  useEffect(() => { setViewed(null); }, [page, context.personId, context.date, context.knowledgeSequence, registry.knowledge_sequence]);
  useEffect(() => { if (editor || confirmation) editorRef.current?.scrollIntoView?.({ block: 'start', behavior: 'smooth' }); }, [editor, confirmation]);

  const people = registry.entities.filter(item => item.kind === 'person');
  const companies = registry.entities.filter(item => item.kind === 'company');
  const reachable = reachableEntities(registry, context);
  const relevantEntity = (id: string) => allRegistry || reachable.has(id);
  const activeMemberships = registry.memberships.filter(item => activeAt(item.valid_from, item.valid_to, context.date));
  const activeParticipations = registry.participations.filter(item => activeAt(item.valid_from, item.valid_to, context.date));
  const activeOwnerships = registry.ownerships.filter(item => activeAt(item.valid_from, item.valid_to, context.date));
  const members = new Set(activeMemberships.map(item => item.person_id));
  const heldCompanies = new Set(activeParticipations.map(item => item.company_id));
  const heldAssets = new Set(activeOwnerships.map(item => item.asset_id));
  const relevantCompany = (id: string) => relevantEntity(id) || (!context.personId && !heldCompanies.has(id));
  const visibleProfile = (item: Entity | Asset) => showArchived || !item.archived;
  const entityName = (id: string) => { const item = registry.entities.find(item => item.id === id); return item ? entityAt(registry, item, context.date).display_name : id; };
  const assetName = (id: string) => { const item = registry.assets.find(item => item.id === id); return item ? assetAt(registry, item, context.date).name : id; };
  const assetRelevant = (id: string) => allRegistry || (!context.personId && !heldAssets.has(id)) || activeOwnerships.some(edge => edge.asset_id === id && reachable.has(edge.owner_entity_id));
  const period = (item: { valid_from: string; valid_to: string | null }) => periodIncluded(item, context.date, allPeriods);

  function openEditor(kind: EditorKind, record: RecordItem | null = null) {
    if (readOnly) return;
    const next = makeEditor(kind, record, context);
    if (kind === 'profile_period' && !record) next.values.target_kind = page === 'assets' ? 'asset' : 'entity';
    setEditor(next); setViewed(null); setConfirmation(null); setError(''); setMessage('');
  }
  function openConfirmation(action: Confirmation['action'], kind: EditorKind, record: RecordItem) {
    if (readOnly) return;
    setConfirmation({ action, kind, record }); setConfirmationReason(''); setEditor(null); setViewed(null); setError(''); setMessage('');
  }
  async function mutate(action: () => Promise<unknown>, success: string) {
    setPending(true); setError(''); setMessage('');
    try { await action(); setEditor(null); setConfirmation(null); await refreshAfterCommit(onChanged, success, setMessage); }
    catch (reason) { setError(errorMessage(reason)); }
    finally { setPending(false); }
  }
  async function save(event: FormEvent) {
    event.preventDefault(); if (!editor || readOnly) return;
    const { kind, record, values: v, reason } = editor;
    const meta = { id: record?.id || null, expected_revision_id: record?.revision_id || null, reason: reason.trim() };
    const fraction = (key: string): Fraction | null => v[`${key}_known`] === 'yes' ? { num: v[`${key}_num`].trim(), den: v[`${key}_den`].trim() } : null;
    const dated = { valid_from: v.valid_from, valid_to: v.valid_to || null };
    const tasks: Record<EditorKind, () => Promise<unknown>> = {
      person: () => api.saveEntity({ ...meta, kind: 'person', display_name: v.display_name.trim(), first_name: v.first_name.trim() || null, last_name: v.last_name.trim() || null, birth_date: v.birth_date || null, residence_country: v.residence_country || null, preferred_currency: v.preferred_currency, activity: '', notes: v.notes, archived: v.archived === 'yes' }),
      company: () => api.saveEntity({ ...meta, kind: 'company', display_name: v.display_name.trim(), first_name: null, last_name: null, birth_date: null, residence_country: v.residence_country || null, preferred_currency: v.preferred_currency, activity: v.activity, notes: v.notes, archived: v.archived === 'yes' }),
      membership: () => api.saveMembership({ ...meta, person_id: v.person_id, ...dated, relationship: v.relationship }),
      participation: () => api.saveParticipation({ ...meta, owner_entity_id: v.owner_entity_id, company_id: v.company_id, legal_share: fraction('legal_share'), economic_share: fraction('economic_share'), voting_share: fraction('voting_share'), ...dated }),
      asset: () => api.saveAsset({ ...meta, name: v.name.trim(), country: v.country, category: v.category.trim(), notes: v.notes, archived: v.archived === 'yes' }),
      ownership: () => api.saveOwnership({ ...meta, asset_id: v.asset_id, owner_entity_id: v.owner_entity_id, legal_share: fraction('legal_share'), economic_share: fraction('economic_share'), ...dated }),
      asset_valuation: () => api.saveValuation({ ...meta, target_kind: 'asset', target_id: v.target_id, amount: v.amount.trim(), currency: v.currency, effective_on: v.effective_on, source: v.source }),
      company_valuation: () => api.saveValuation({ ...meta, target_kind: 'company', target_id: v.target_id, amount: v.amount.trim(), currency: v.currency, effective_on: v.effective_on, source: v.source }),
      company_policy: () => api.saveCompanyPolicy({ ...meta, company_id: v.company_id, mode: v.mode as CompanyPolicy['mode'], coverage_complete: v.coverage_complete === 'yes', liability_qualified: v.liability_qualified === 'yes', ...dated }),
      profile_period: () => api.saveProfilePeriod({ ...meta, target_kind: v.target_kind as ProfilePeriod['target_kind'], target_id: v.target_id, profile_revision_id: v.profile_revision_id, ...dated }),
    };
    await mutate(async () => {
      await tasks[kind]();
      if (!record && (kind === 'person' || context.personId)) setAllRegistry(true);
    }, record ? 'La correction a été enregistrée. Les versions précédentes sont conservées.' : 'L’enregistrement a été créé dans le registre local.');
  }
  async function confirmMutation(event: FormEvent) {
    event.preventDefault(); if (!confirmation || readOnly) return;
    const { action, kind, record } = confirmation;
    const { id, revision_id, ...fields } = record;
    const reason = confirmationReason.trim();
    await mutate(() => {
      if (action === 'void') return api.voidRegistryRecord(id, revision_id, reason);
      const archived = action === 'archive';
      const meta = { id, expected_revision_id: revision_id, reason };
      return kind === 'asset' ? api.saveAsset({ ...fields as Omit<Asset, keyof RegistryRecord>, ...meta, archived })
        : api.saveEntity({ ...fields as Omit<Entity, keyof RegistryRecord>, ...meta, archived });
    }, action === 'void' ? 'L’enregistrement a été annulé avec conservation de son historique.' : action === 'archive' ? 'La fiche a été archivée. Ses droits et son historique sont conservés.' : 'La fiche a été réactivée.');
  }
  function actions(kind: EditorKind, record: RecordItem, profile = false) {
    return <div className="row-actions">
      {('display_name' in record || 'name' in record) && <button type="button" className="button secondary small-button" disabled={pending} aria-label={`Voir ${'display_name' in record ? entityName(record.id) : assetName(record.id)}`} onClick={() => { setViewed(record); setEditor(null); setConfirmation(null); }}>Voir</button>}
      <button type="button" className="button secondary small-button" title={profile ? 'Modifier la fiche actuelle' : undefined} disabled={pending || readOnly} onClick={() => openEditor(kind, record)}>Modifier</button>
      {profile ? <button type="button" className="text-button" disabled={pending || readOnly} onClick={() => openConfirmation((record as Entity | Asset).archived ? 'unarchive' : 'archive', kind, record)}>{(record as Entity | Asset).archived ? 'Réactiver' : 'Archiver'}</button> : <button type="button" className="text-button" disabled={pending || readOnly} onClick={() => openConfirmation('void', kind, record)}>Annuler l’enregistrement</button>}
    </div>;
  }
  function table(title: string, headers: string[], rows: ReactNode[], addLabel: string, kind: EditorKind, empty = 'Aucune information pour la sélection actuelle.') {
    return <section className="card registry-table"><div className="section-title"><h2>{title}</h2><button type="button" className="button secondary" disabled={pending || readOnly} onClick={() => openEditor(kind)}>{addLabel}</button></div>{rows.length ? <div className="table-scroll"><table><caption className="sr-only">{title}</caption><thead><tr>{headers.map(header => <th key={header} scope="col">{header}</th>)}</tr></thead><tbody>{rows}</tbody></table></div> : <p className="muted">{empty}</p>}</section>;
  }

  const displayedProfile = viewed && ('display_name' in viewed ? entityAt(registry, viewed, context.date) : assetAt(registry, viewed, context.date));
  const displayedEstimate = displayedProfile && 'name' in displayedProfile
    ? registry.valuations.filter(item => item.target_kind === 'asset' && item.target_id === displayedProfile.id && item.effective_on <= context.date)
      .sort((a, b) => b.effective_on.localeCompare(a.effective_on))[0]
    : undefined;

  return <div className="registry-workspace" aria-busy={pending}>
    {readOnly && <RegistryNotice><strong>Lecture seule historique.</strong> Les données et leurs versions sont celles connues à la séquence {context.knowledgeSequence}. Revenez aux connaissances actuelles pour modifier le registre.</RegistryNotice>}
    <details className="card registry-filters advanced-disclosure">
      <summary>Filtres et options d’affichage{allRegistry || allPeriods || showArchived ? ' · filtres actifs' : ''}</summary>
      <label className="checkbox-label"><input type="checkbox" checked={allRegistry} disabled={pending} onChange={event => setAllRegistry(event.target.checked)} /><span>{page === 'family' ? 'Inclure les personnes extérieures à la famille' : 'Afficher toutes les fiches, au-delà du périmètre sélectionné'}</span></label>
      <label className="checkbox-label"><input type="checkbox" checked={allPeriods} disabled={pending} onChange={event => setAllPeriods(event.target.checked)} /><span>Afficher aussi les périodes passées et futures</span></label>
      <label className="checkbox-label"><input type="checkbox" checked={showArchived} disabled={pending} onChange={event => setShowArchived(event.target.checked)} /><span>Afficher les fiches archivées</span></label>
      <p className="field-hint">Les liens affichés sont ceux en vigueur au {context.date}. Une date de fin marque le premier jour hors période. Les noms et adresses suivent les profils datés disponibles. À défaut, la fiche connue pour cette consultation est affichée.</p>
    </details>
    {error && <RegistryNotice error><strong>Enregistrement impossible.</strong><p>{error}</p><p>En cas de conflit de révision, rechargez le registre et rouvrez la fiche. Votre saisie n’a pas remplacé une version plus récente.</p><button type="button" className="button secondary" disabled={pending} onClick={() => { setEditor(null); setConfirmation(null); void mutate(onChanged, 'Le registre a été rechargé. Rouvrez la fiche pour reprendre votre correction.'); }}>Recharger le registre</button></RegistryNotice>}
    {message && <RegistryNotice>{message}</RegistryNotice>}
    <div ref={editorRef}>
      {displayedProfile && <section className="card" ref={viewFocus} tabIndex={-1} aria-label="Détails de la fiche">
        <div className="section-title"><h2>{'name' in displayedProfile ? displayedProfile.name : displayedProfile.display_name}</h2><button type="button" className="button secondary" onClick={() => setViewed(null)}>Retour à la liste</button></div>
        <dl className="details">
          {'name' in displayedProfile ? <>
            <div><dt>Catégorie et pays</dt><dd>{displayedProfile.category} · {displayedProfile.country}</dd></div>
            <div><dt>Dernière estimation du bien entier</dt><dd>{displayedEstimate ? `${displayedEstimate.amount} ${displayedEstimate.currency} · ${displayedEstimate.effective_on}` : 'À renseigner'}</dd></div>
            <div><dt>Propriétaires déclarés</dt><dd>{activeOwnerships.filter(item => item.asset_id === displayedProfile.id).map(item => entityName(item.owner_entity_id)).join(', ') || 'À renseigner'}</dd></div>
          </> : <>
            <div><dt>{displayedProfile.kind === 'company' ? 'Pays d’immatriculation' : 'Pays de résidence'}</dt><dd>{displayedProfile.residence_country || 'À renseigner'}</dd></div>
            {displayedProfile.kind === 'company' ? <>
              <div><dt>Activité</dt><dd>{displayedProfile.activity || 'À renseigner'}</dd></div>
              <div><dt>Associés déclarés</dt><dd>{activeParticipations.filter(item => item.company_id === displayedProfile.id).map(item => entityName(item.owner_entity_id)).join(', ') || 'À renseigner'}</dd></div>
            </> : <div><dt>Appartenance à la famille</dt><dd>{members.has(displayedProfile.id) ? 'Membre à la date de consultation' : 'Extérieur à la famille à la date de consultation'}</dd></div>}
          </>}
          {displayedProfile.notes && <div><dt>Notes</dt><dd>{displayedProfile.notes}</dd></div>}
        </dl>
      </section>}
      {editor && !readOnly && <EditorFrame title={`${editor.record ? 'Modifier' : 'Créer'} · ${editorTitles[editor.kind]}`} pending={pending} onCancel={() => setEditor(null)}><RegistryForm editor={editor} setEditor={setEditor} registry={registry} pending={pending} onSubmit={save} /></EditorFrame>}
      {confirmation && !readOnly && <section ref={confirmationFocus} tabIndex={-1} className="card restore-review" aria-label="Confirmer la modification du registre"><h2>{confirmation.action === 'void' ? 'Confirmer l’annulation' : confirmation.action === 'archive' ? 'Confirmer l’archivage' : 'Confirmer la réactivation'}</h2><p>{confirmation.action === 'void' ? 'Cette correction annule cet enregistrement dans l’état courant de connaissance. Ses révisions restent conservées. Pour terminer une période à une date donnée, modifiez sa date de fin.' : 'L’historique et les relations de propriété sont conservés. Archiver une fiche ne cède pas ses actifs et ne la retire pas automatiquement du périmètre familial.'}</p><form onSubmit={event => { void confirmMutation(event); }}><fieldset disabled={pending}><label htmlFor="confirmation-reason">Motif de la modification</label><textarea id="confirmation-reason" required maxLength={1000} value={confirmationReason} onChange={event => setConfirmationReason(event.target.value)} /><div className="button-row"><button type="submit" className="button danger" disabled={pending || !confirmationReason.trim()}>{confirmation.action === 'void' ? 'Confirmer l’annulation' : confirmation.action === 'archive' ? 'Confirmer l’archivage' : 'Confirmer la réactivation'}</button><button type="button" className="button secondary" onClick={() => setConfirmation(null)}>Conserver l’état actuel</button></div></fieldset></form></section>}
    </div>
    {page === 'family' && <>
      {table('Personnes, membres et tiers', ['Personne', 'Lien avec la famille', 'Résidence', 'Actions'], people.filter(item => visibleProfile(item) && (allRegistry || (context.personId ? item.id === context.personId : members.has(item.id)))).map(item => <tr key={item.id}><td><strong>{entityName(item.id)}</strong>{item.archived && <small>Archivé</small>}</td><td>{members.has(item.id) ? 'Membre de la famille' : 'Extérieur à la famille'}</td><td>{entityAt(registry, item, context.date).residence_country || 'Non renseignée'}</td><td>{actions('person', item, true)}</td></tr>), 'Créer une personne', 'person', people.length ? 'Aucune personne dans cette vue. Les filtres permettent d’afficher les personnes extérieures à la famille.' : 'Aucun membre enregistré. Ajoutez un membre de la famille ou créez la fiche d’un contact, par exemple un locataire.')}
      <details className="card advanced-disclosure">
        <summary>Périodes d’appartenance à la famille</summary>
        <p className="field-hint">Les droits d’un membre sont pris en compte dans les totaux familiaux aux dates où il fait partie de la famille. Un locataire ou un copropriétaire extérieur peut avoir sa propre fiche en tant que contact.</p>
        {table('Périodes du périmètre familial', ['Personne', 'Période', 'Lien familial', 'Actions'], registry.memberships.filter(item => period(item) && (allRegistry || !context.personId || item.person_id === context.personId)).map(item => <tr key={item.id}><td>{entityName(item.person_id)}</td><td><Period from={item.valid_from} to={item.valid_to} /></td><td>{item.relationship || 'Non renseigné'}</td><td>{actions('membership', item)}</td></tr>), 'Ajouter une période familiale', 'membership')}
      </details>
    </>}
    {page === 'companies' && <>
      {table('Sociétés', ['Nom', 'Pays', 'Activité', 'Actions'], companies.filter(item => visibleProfile(item) && relevantCompany(item.id)).map(item => <tr key={item.id}><td><strong>{entityName(item.id)}</strong>{item.archived && <small>Archivée</small>}{!heldCompanies.has(item.id) && <small>Associés à renseigner</small>}</td><td>{entityAt(registry, item, context.date).residence_country || 'Non renseigné'}</td><td>{entityAt(registry, item, context.date).activity || 'Non renseignée'}</td><td>{actions('company', item, true)}</td></tr>), 'Créer une société', 'company', companies.length ? 'Aucune société dans cette vue. Élargissez la sélection dans les filtres.' : 'Ajoutez une société pour retrouver ses informations et ses associés ici.')}
      {table('Associés et participations', ['Associé', 'Société', 'Parts juridiques / économiques / votes', 'Période', 'Actions'], registry.participations.filter(item => period(item) && (relevantEntity(item.owner_entity_id) || relevantCompany(item.company_id))).map(item => <tr key={item.id}><td>{entityName(item.owner_entity_id)}</td><td>{entityName(item.company_id)}</td><td>{fractionLabel(item.legal_share)} / {fractionLabel(item.economic_share)} / {fractionLabel(item.voting_share)}</td><td><Period from={item.valid_from} to={item.valid_to} /></td><td>{actions('participation', item)}</td></tr>), 'Ajouter une participation', 'participation', 'Ajoutez une participation lorsque vous connaissez les droits d’un associé. Une société et ses associés ont chacun leur propre fiche.')}
      <details className="card advanced-disclosure">
        <summary>Options avancées de consolidation et de valorisation</summary>
        <p className="field-hint">Choisissez de tenir compte des biens et dettes de la société en détail ou de sa valeur nette globale. Le calcul évite le double comptage.</p>
        {table('Méthodes de consolidation datées', ['Société', 'Méthode', 'Qualification', 'Période', 'Actions'], registry.company_policies.filter(item => period(item) && relevantCompany(item.company_id)).map(item => <tr key={item.id}><td>{entityName(item.company_id)}</td><td>{item.mode === 'detailed' ? 'Actifs sous-jacents détaillés' : 'Valeur globale des capitaux propres'}</td><td>{item.coverage_complete ? 'Couverture déclarée complète' : 'Couverture incomplète'}<br />{item.liability_qualified ? 'Responsabilité qualifiée' : 'Responsabilité à qualifier'}</td><td><Period from={item.valid_from} to={item.valid_to} /></td><td>{actions('company_policy', item)}</td></tr>), 'Définir une méthode', 'company_policy')}
        {valuationTable('company')}
      </details>
    </>}
    {page === 'assets' && <>
      {table('Biens enregistrés', ['Bien', 'Pays', 'Catégorie', 'Actions'], registry.assets.filter(item => visibleProfile(item) && assetRelevant(item.id)).map(item => <tr key={item.id}><td><strong>{assetName(item.id)}</strong>{item.archived && <small>Archivé</small>}{!heldAssets.has(item.id) && <small>Propriétaire à renseigner</small>}</td><td>{assetAt(registry, item, context.date).country}</td><td>{assetAt(registry, item, context.date).category}</td><td>{actions('asset', item, true)}</td></tr>), 'Créer un actif', 'asset', registry.assets.length ? 'Aucun bien dans cette vue. Élargissez la sélection dans les filtres.' : 'Les biens ajoutés dans les différentes rubriques apparaissent ici. Créez un actif pour suivre un autre type de bien.')}
      {table('Propriétaires et droits', ['Bien', 'Propriétaire juridique', 'Part juridique', 'Part économique', 'Période', 'Actions'], registry.ownerships.filter(item => period(item) && (allRegistry || relevantEntity(item.owner_entity_id))).map(item => <tr key={item.id}><td>{assetName(item.asset_id)}</td><td>{entityName(item.owner_entity_id)}</td><td>{fractionLabel(item.legal_share)}</td><td>{fractionLabel(item.economic_share)}</td><td><Period from={item.valid_from} to={item.valid_to} /></td><td>{actions('ownership', item)}</td></tr>), 'Attribuer une propriété', 'ownership', 'Ajoutez les droits de propriété lorsqu’ils sont connus. La part économique est la part de la valeur qui revient à chaque propriétaire.')}
      {valuationTable('asset')}
      <ConsolidationPanel context={context} knowledgeSequence={registry.knowledge_sequence} onBusyChange={setConsolidationBusy} />
    </>}
    <details className="card advanced-disclosure"><summary>Profils datés et historique des fiches</summary>
    {table('Périodes des profils descriptifs', ['Personne, société ou bien', 'Profil appliqué', 'Période', 'Actions'], registry.profile_periods.filter(item => period(item) && (page === 'assets' ? item.target_kind === 'asset' && assetRelevant(item.target_id) : item.target_kind === 'entity' && registry.entities.some(entity => entity.id === item.target_id && entity.kind === (page === 'family' ? 'person' : 'company')) && (page === 'family' ? allRegistry || (context.personId ? item.target_id === context.personId : members.has(item.target_id)) : relevantCompany(item.target_id)))).map(item => {
      const profile = item.target_kind === 'asset' ? [...registry.assets, ...registry.asset_profile_revisions].find(profile => profile.id === item.target_id && profile.revision_id === item.profile_revision_id) : [...registry.entities, ...registry.profile_revisions].find(profile => profile.id === item.target_id && profile.revision_id === item.profile_revision_id);
      return <tr key={item.id}><td>{item.target_kind === 'asset' ? assetName(item.target_id) : entityName(item.target_id)}</td><td>{profile && ('name' in profile ? `${profile.name} · ${profile.country}` : `${profile.display_name} · ${profile.residence_country || 'Pays non renseigné'}`)}<small className="hash">{item.profile_revision_id}</small></td><td><Period from={item.valid_from} to={item.valid_to} /></td><td>{actions('profile_period', item)}</td></tr>;
    }), 'Définir une période de profil', 'profile_period')}
    </details>
  </div>;

  function valuationTable(kind: 'asset' | 'company') {
    const relevant = kind === 'asset' ? assetRelevant : relevantCompany;
    return table(kind === 'asset' ? 'Estimations des biens' : 'Valeurs globales des capitaux propres', ['Bien ou société', 'Estimation d’origine', 'Date', 'Source', 'Actions'], registry.valuations.filter(item => item.target_kind === kind && relevant(item.target_id) && (allPeriods || item.effective_on <= context.date)).map(item => <tr key={item.id}><td>{kind === 'asset' ? assetName(item.target_id) : entityName(item.target_id)}</td><td>{item.amount} {item.currency}</td><td>{item.effective_on}</td><td>{item.source || 'Non renseignée'}</td><td>{actions(kind === 'asset' ? 'asset_valuation' : 'company_valuation', item)}</td></tr>), kind === 'asset' ? 'Ajouter une estimation' : 'Ajouter une valeur globale', kind === 'asset' ? 'asset_valuation' : 'company_valuation', 'Aucune estimation pour cette sélection. Renseignez le montant dans sa devise d’origine et indiquez d’où il vient.');
  }
}

function RegistryForm({ editor, setEditor, registry, pending, onSubmit }: {
  editor: Editor; setEditor: Dispatch<SetStateAction<Editor | null>>; registry: RegistrySnapshot; pending: boolean; onSubmit: (event: FormEvent) => Promise<void>;
}) {
  const { kind, values: v } = editor;
  const change = (key: string, value: string) => setEditor(current => current && ({ ...current, values: { ...current.values, [key]: value } }));
  const input = (key: string, label: string, required = false, type = 'text', hint?: string) => <Field key={key} id={`registry-${key}`} label={label} hint={hint}><input id={`registry-${key}`} type={type} required={required} value={v[key]} maxLength={key === 'amount' ? 100 : 200} onChange={event => change(key, event.currentTarget.value)} aria-describedby={hint ? `registry-${key}-help` : undefined} /></Field>;
  const select = (key: string, label: string, items: { id: string; label: string }[], required = true) => <Field key={key} id={`registry-${key}`} label={label}><select id={`registry-${key}`} required={required} value={v[key]} onChange={event => change(key, event.currentTarget.value)}><option value="">Choisir…</option>{items.map(item => <option key={item.id} value={item.id}>{item.label}</option>)}</select></Field>;
  const entityOptions = (type?: string) => registry.entities.filter(item => !type || item.kind === type).map(item => ({ id: item.id, label: `${item.display_name}${item.archived ? ' (archivé)' : ''}` }));
  const dates = <div className="form-grid">{input('valid_from', 'Date de début', true, 'date')}{input('valid_to', 'Date de fin (exclue)', false, 'date', 'Laisser vide si la période n’a pas de fin connue.')}</div>;
  const country = (key: string, label: string, required = false) => select(key, label, countries.map(([id, name]) => ({ id, label: `${name} (${id})` })), required);
  const checkbox = (key: string, label: string) => <label className="checkbox-label" key={key}><input type="checkbox" checked={v[key] === 'yes'} onChange={event => change(key, event.target.checked ? 'yes' : 'no')} /><span>{label}</span></label>;
  const ratio = (key: string, label: string) => <div className="ratio-field" key={key}>{checkbox(`${key}_known`, `${label} connu`)}{v[`${key}_known`] === 'yes' ? <div className="ratio-inputs"><Field id={`registry-${key}-num`} label={`${label} · numérateur`}><input id={`registry-${key}-num`} required inputMode="numeric" pattern="[0-9]+" value={v[`${key}_num`]} onChange={event => change(`${key}_num`, event.target.value)} /></Field><span aria-hidden="true">/</span><Field id={`registry-${key}-den`} label={`${label} · dénominateur`}><input id={`registry-${key}-den`} required inputMode="numeric" pattern="[0-9]+" value={v[`${key}_den`]} onChange={event => change(`${key}_den`, event.target.value)} /></Field></div> : <p className="field-hint">Inconnu : aucune valeur n’est déduite. Zéro, 100 % et inconnu sont distincts.</p>}</div>;
  const notes = <Field id="registry-notes" label="Notes" hint="Facultatif. Par exemple un contact ou la référence d’un dossier."><textarea id="registry-notes" aria-describedby="registry-notes-help" maxLength={10000} value={v.notes} onChange={event => change('notes', event.target.value)} /></Field>;
  const profileCandidates = v.target_kind === 'asset' ? [...registry.assets, ...registry.asset_profile_revisions].filter(item => item.id === v.target_id) : [...registry.entities, ...registry.profile_revisions].filter(item => item.id === v.target_id);
  const profileOptions = [...new Map(profileCandidates.map(item => [item.revision_id, item])).values()].map(item => ({ id: item.revision_id, label: `${'name' in item ? `${item.name} · ${item.country}` : `${item.display_name} · ${item.residence_country || 'Pays non renseigné'}`} · révision ${item.revision_id}` }));
  return <form onSubmit={event => { void onSubmit(event); }}><fieldset disabled={pending}>
    {(kind === 'person' || kind === 'company') && <>
      {input('display_name', kind === 'person' ? 'Nom affiché de la personne' : 'Nom de la société', true)}
      {kind === 'person' && <div className="form-grid">{input('first_name', 'Prénom')}{input('last_name', 'Nom de famille')}</div>}
      {country('residence_country', kind === 'person' ? 'Pays de résidence' : 'Pays d’immatriculation')}
      {kind === 'company' && input('activity', 'Activité', false, 'text', 'Facultatif. Par exemple gestion locative ou commerce.')}
      {kind === 'person' && !editor.record && <p className="field-hint">Cette fiche convient à un contact, par exemple un locataire. Pour une personne de la famille, vous pouvez aussi ajouter sa période d’appartenance après la création de sa fiche.</p>}
      <details className="advanced-disclosure" onInvalid={event => { event.currentTarget.open = true; }}>
        <summary>Options avancées</summary>
        {kind === 'person' && input('birth_date', 'Date de naissance', false, 'date', 'Facultatif.')}
        {select('preferred_currency', 'Devise préférée', displayCurrencies.map(id => ({ id, label: id })))}
        {notes}
      </details>
    </>}
    {kind === 'membership' && <>{select('person_id', 'Personne à inclure dans la famille', entityOptions('person'))}{input('relationship', 'Lien familial (facultatif)')}{dates}</>}
    {kind === 'asset' && <>{input('name', 'Nom de l’actif', true)}<div className="form-grid">{country('country', 'Pays de l’actif', true)}{input('category', 'Catégorie du bien', true, 'text', 'Par exemple véhicule, mobilier ou collection.')}</div><p className="field-hint">Utilisez les rubriques Immobilier, Comptes bancaires ou Or et argent pour les biens qui nécessitent un suivi dédié.</p><details className="advanced-disclosure"><summary>Options avancées</summary>{notes}</details></>}
    {(kind === 'participation' || kind === 'ownership') && <>
      {kind === 'participation' ? select('company_id', 'Société détenue', entityOptions('company')) : select('asset_id', 'Actif concerné', registry.assets.map(item => ({ id: item.id, label: item.name })))}
      {select('owner_entity_id', kind === 'participation' ? 'Associé ou société propriétaire' : 'Propriétaire identifié', entityOptions())}
      <p className="field-hint">La part juridique est celle indiquée dans les actes. La part économique correspond à la part de la valeur qui revient à cette personne ou société. Pour 50 %, saisissez 1 et 2. Pour 30 %, saisissez 3 et 10.</p>
      {ratio('legal_share', 'Droit juridique')}{ratio('economic_share', 'Droit économique')}{kind === 'participation' && ratio('voting_share', 'Droit de vote')}{dates}
    </>}
    {(kind === 'asset_valuation' || kind === 'company_valuation') && <>
      {select('target_id', kind === 'asset_valuation' ? 'Actif à valoriser' : 'Société à valoriser', kind === 'asset_valuation' ? registry.assets.map(item => ({ id: item.id, label: item.name })) : entityOptions('company'))}
      <div className="form-grid">{input('amount', kind === 'company_valuation' ? `Valeur nette des capitaux propres (${v.currency})` : `Montant de l’estimation (${v.currency})`, true, 'text', 'Saisissez le montant dans sa devise. Exemple pour 1 250,50 € : 1250,50.')}{select('currency', 'Devise de l’estimation', displayCurrencies.map(id => ({ id, label: id })))}{input('effective_on', 'Date de valorisation', true, 'date')}</div>{input('source', 'Source ou méthode de l’estimation', true, 'text', 'Par exemple une expertise immobilière ou le prix moyen d’annonces comparables.')}
      <p className="field-hint">Le montant d’origine est conservé dans sa devise. La consolidation en EUR utilise les taux historiques applicables et leurs sources. Une nouvelle estimation datée s’ajoute à l’historique ; « Modifier » corrige une estimation existante.</p>
    </>}
    {kind === 'company_policy' && <>
      {select('company_id', 'Société concernée', entityOptions('company'))}{select('mode', 'Méthode de consolidation', [{ id: 'detailed', label: 'Actifs et passifs sous-jacents détaillés' }, { id: 'global_equity', label: 'Valeur globale des capitaux propres' }])}
      {checkbox('coverage_complete', 'Le périmètre des actifs et passifs connus est déclaré complet')}{checkbox('liability_qualified', 'La responsabilité et les engagements des associés ont été qualifiés')}
      <p className="field-hint">Une déclaration de complétude ne remplace pas la saisie des données. Le bilan financier rapproche les comptes et financements séparément ; ce contrôle d’attribution du registre ne constitue pas un bilan net exhaustif.</p>{dates}
    </>}
    {kind === 'profile_period' && <>
      <RegistryNotice>Une période applique les métadonnées d’une révision existante à l’objet choisi. Elle ne modifie aucun droit, montant ni appartenance familiale. Créez ou corrigez d’abord la fiche si le profil souhaité n’existe pas encore.</RegistryNotice>
      <Field id="registry-target_kind" label="Type d’objet du profil"><select id="registry-target_kind" value={v.target_kind} onChange={event => { const target_kind = event.currentTarget.value; setEditor(current => current && ({ ...current, values: { ...current.values, target_kind, target_id: '', profile_revision_id: '' } })); }}><option value="entity">Personne ou société</option><option value="asset">Actif</option></select></Field>
      <Field id="registry-target_id" label="Objet du profil"><select id="registry-target_id" required value={v.target_id} onChange={event => { const target_id = event.currentTarget.value; setEditor(current => current && ({ ...current, values: { ...current.values, target_id, profile_revision_id: '' } })); }}><option value="">Choisir…</option>{(v.target_kind === 'asset' ? registry.assets.map(item => ({ id: item.id, label: item.name })) : entityOptions()).map(item => <option key={item.id} value={item.id}>{item.label}</option>)}</select></Field>
      {select('profile_revision_id', 'Révision exacte du profil', profileOptions)}{dates}
      <p className="field-hint">Les périodes d’un même objet ne doivent pas se chevaucher. La fin est exclue. Les révisions anciennes restent proposées avec leur identifiant complet.</p>
    </>}
    {editor.record && (kind === 'person' || kind === 'company' || kind === 'asset') && <p className="field-hint"><strong>Modifier le profil de base des connaissances actuelles.</strong> Les profils déjà rattachés à une période gardent leur version d’origine.</p>}
    <Field id="registry-reason" label={editor.record ? 'Motif de la correction' : 'Motif de l’enregistrement'} hint="Indiquez brièvement la raison pour pouvoir la retrouver dans l’historique."><textarea id="registry-reason" aria-describedby="registry-reason-help" required maxLength={1000} value={editor.reason} onChange={event => { const reason = event.currentTarget.value; setEditor(current => current && ({ ...current, reason })); }} /></Field>
    <button type="submit" className="button" disabled={pending || !editor.reason.trim()}>{pending ? 'Enregistrement…' : editor.record ? 'Enregistrer la correction' : 'Créer l’enregistrement'}</button>
  </fieldset></form>;
}

function ConsolidationPanel({ context, knowledgeSequence, onBusyChange }: { context: ViewContext; knowledgeSequence: number; onBusyChange: (busy: boolean) => void }) {
  const [report, setReport] = useState<ConsolidationReport | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const requestId = useRef(0);
  useEffect(() => { requestId.current += 1; setReport(null); setError(''); }, [context.personId, context.date, context.knowledgeSequence, knowledgeSequence]);
  useEffect(() => { onBusyChange(pending); return () => onBusyChange(false); }, [pending, onBusyChange]);
  async function check() {
    setPending(true); setError('');
    const selected = ++requestId.current;
    try { const result = await api.getConsolidation({ context: context.personId ? 'person' : 'family', person_id: context.personId, date: context.date, knowledge_sequence: context.knowledgeSequence ?? null }); if (selected === requestId.current) setReport(result); }
    catch (reason) { if (selected === requestId.current) setError(errorMessage(reason)); }
    finally { setPending(false); }
  }
  return <section className="card consolidation-check" aria-busy={pending}><div className="section-title"><div><h2>Contrôle des attributions</h2><p>Vérifiez les contributions directes et indirectes à la date de référence, avec leurs sources.</p></div><button type="button" className="button" disabled={pending} onClick={() => { void check(); }}>{pending ? 'Calcul en cours…' : 'Contrôler les attributions'}</button></div>
    <p className="scope-note">Actifs et participations connus, convertis en EUR selon les taux historiques applicables. Le bilan financier ci-dessous complète ce contrôle du registre avec les comptes et les financements. Ce contrôle ne présente pas un patrimoine net exhaustif.</p>
    {error && <RegistryNotice error>{error}</RegistryNotice>}
    {report && <><div className="control-result"><span>Sous-total des contributions valorisables</span><strong>{report.status === 'non_calculable' ? 'Non calculable' : `${report.gross_eur} EUR`}</strong><small>{report.date} · {report.status === 'complete' ? 'Données du contrôle complètes' : 'Données incomplètes ou non calculables'} · moteur {report.engine_version}</small></div>
      {report.global_equity_eur !== '0.00' && report.global_equity_eur !== '0' && <p className="field-hint">Ce montant comprend des participations évaluées par leurs capitaux propres ; il ne décrit pas le brut des actifs sous-jacents.</p>}
      {report.warnings.length > 0 && <RegistryNotice><strong>Points à vérifier</strong><ul>{report.warnings.map((warning, index) => <li key={`${warning.object_id}-${warning.code}-${index}`}>{warning.message}</li>)}</ul></RegistryNotice>}
      {report.contributions.length ? <div className="table-scroll"><table><caption className="sr-only">Contributions calculées</caption><thead><tr><th scope="col">Actif / participation</th><th scope="col">Droit économique</th><th scope="col">Valeur d’origine</th><th scope="col">Contribution EUR</th><th scope="col">Traçabilité</th></tr></thead><tbody>{report.contributions.map((line, index) => <tr key={`${line.object_id}-${index}`}><td>{line.label}<small>{line.kind === 'metal' ? (line.category === 'gold' ? 'Or physique' : line.category === 'silver' ? 'Argent physique' : 'Métal physique') : line.category} · {line.country || 'Pays non renseigné'}</small></td><td>{fractionLabel(line.weight)}</td><td>{line.amount_original} {line.currency}<small>{line.valuation_date || 'Date non renseignée'}</small></td><td>{line.amount_eur} EUR</td><td><details><summary>Voir les sources</summary><p className="hash">Objet : {line.object_id}<br />Valorisation : {line.valuation_id || 'Aucune'}<br />Révision : {line.valuation_revision_id || 'Aucune'}</p><p className="hash">Droits : {line.source_ids.join(', ') || 'Aucun'}</p><p className="hash">Contribution exacte : {fractionLabel(line.exact_eur)} EUR</p>{line.conversion ? <FxTrace result={line.conversion} /> : <p>Conversion non fournie par le calcul pour cette ligne.</p>}</details></td></tr>)}</tbody></table></div> : <p className="muted">Aucune contribution valorisable dans ce contexte. Consultez les éventuelles alertes.</p>}
    </>}
  </section>;
}
