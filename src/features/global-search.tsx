import { useEffect, useMemo, useRef, useState } from 'react';
import { api, errorMessage } from '../services/api';
import type { DocumentLibrarySnapshot } from '../services/documents-types';
import type { RegistrySnapshot } from '../services/registry-types';
import type { TreasuryAccount } from '../services/treasury-types';
import { assetAt, entityAt } from './registry-ui';
import type { ViewContext } from './registry-ui';
import './global-search.css';

interface SearchEntry { id: string; label: string; text: string; kind: string; page: string; retired: boolean }
const moduleNames: Record<string, string> = { family: 'Famille', assets: 'Patrimoine', property: 'Immobilier et loyers', bank: 'Comptes bancaires', cash: 'Espèces et devises', 'cash-accounts': 'Espèces et devises', metals: 'Or et argent', companies: 'Sociétés', debt: 'Dettes et financements', documents: 'Documents', history: 'Historique' };
function pageFor(kind: string): string {
  if (kind.startsWith('metal_')) return 'metals';
  if (kind.startsWith('company_') || kind === 'participation') return 'companies';
  if (kind.startsWith('debt_')) return 'debt';
  if (kind === 'exchange_rate') return 'cash';
  if (['property', 'rental_unit', 'property_event', 'lease', 'lease_terms', 'rent_due', 'rent_payment', 'rent_allocation', 'rental_expense', 'deposit_movement'].includes(kind)) return 'property';
  if (['treasury_account', 'treasury_transaction', 'account_reconciliation', 'cash_link'].includes(kind)) return 'bank';
  if (['asset', 'ownership', 'valuation', 'finance_coverage', 'rent_claim', 'rent_claim_adjustment'].includes(kind)) return 'assets';
  if (['entity', 'membership'].includes(kind)) return 'family';
  return 'history';
}
function normalized(value: string): string { return value.normalize('NFD').replace(/\p{M}/gu, '').toLocaleLowerCase('fr-FR'); }

export default function GlobalSearch({ registry, context, disabled, onNavigate }: {
  registry: RegistrySnapshot; context: ViewContext; disabled: boolean; onNavigate: (page: string) => void;
}) {
  const [opened, setOpened] = useState(false);
  const [query, setQuery] = useState('');
  const [limit, setLimit] = useState(30);
  const [library, setLibrary] = useState<DocumentLibrarySnapshot | null>(null);
  const [accounts, setAccounts] = useState<TreasuryAccount[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [retry, setRetry] = useState(0);
  const requestId = useRef(0);
  const toggle = useRef<HTMLButtonElement>(null);
  const knowledge = context.knowledgeSequence ?? registry.knowledge_sequence;
  useEffect(() => {
    const id = ++requestId.current;
    setLibrary(null); setAccounts([]); setError('');
    if (!opened) { setLoading(false); return; }
    setLoading(true);
    void (async () => {
      try {
        const [result, treasury] = await Promise.all([api.listDocumentLibraryAt(knowledge), api.listTreasuryAt(knowledge)]);
        if (id !== requestId.current) return;
        if (!result || !treasury || result.knowledge_sequence !== knowledge || treasury.knowledge_sequence !== knowledge) throw new Error('Les résultats ne correspondent pas aux connaissances sélectionnées. Rechargez la recherche.');
        setAccounts(treasury.accounts);
        setLibrary(result);
      } catch (failure) { if (id === requestId.current) setError(errorMessage(failure)); }
      finally { if (id === requestId.current) setLoading(false); }
    })();
    return () => { requestId.current += 1; };
  }, [opened, knowledge, retry]);
  const entries = useMemo((): SearchEntry[] => {
    if (!library) return [];
    const targets = library.targets.map(target => {
      const entity = registry.entities.find(item => item.id === target.id);
      const account = accounts.find(item => item.id === target.id);
      const asset = registry.assets.find(item => item.id === (account?.asset_id ?? target.id));
      const label = entity ? entityAt(registry, entity, context.date).display_name : asset ? assetAt(registry, asset, context.date).name : target.label;
      const page = entity?.kind === 'company' ? 'companies' : account?.kind === 'cash' ? 'cash-accounts' : target.kind === 'treasury_account' && !account ? 'history' : pageFor(target.kind);
      return { id: target.id, label, text: [label, target.label, target.id, target.kind, moduleNames[page], account?.bank_name, account?.reference, account?.storage_location].join(' '), kind: entity?.kind === 'company' ? 'Société' : moduleNames[page], page, retired: target.retired };
    });
    return [...targets, ...library.documents.map(document => ({ id: document.id, label: document.original_name, text: [document.id, document.original_name, document.description, document.mime_type, document.sha256, 'document'].join(' '), kind: document.status === 'archived' ? 'Document archivé' : 'Document', page: 'documents', retired: document.status === 'deleted' }))];
  }, [library, registry, context.date, accounts]);
  const words = normalized(query.trim()).split(/\s+/).filter(Boolean);
  const results = query.trim().length < 2 ? [] : entries.filter(entry => words.every(word => normalized(entry.text).includes(word)));
  function close() { setOpened(false); setQuery(''); setLimit(30); requestAnimationFrame(() => toggle.current?.focus()); }
  return <div className="global-search">
    <button ref={toggle} type="button" className="global-search-toggle" aria-expanded={opened} aria-controls="global-search-panel" disabled={disabled} onClick={() => { if (opened) close(); else setOpened(true); }}>Rechercher dans le coffre</button>
    {opened && <section id="global-search-panel" className="global-search-panel" aria-label="Recherche globale dans le coffre" onKeyDown={event => { if (event.key === 'Escape') { close(); } }}>
      <div className="section-title"><h2>Recherche dans le coffre</h2><button type="button" className="button secondary small-button" onClick={close}>Fermer la recherche</button></div>
      <p className="small">Tous les objets et documents connus à K={knowledge}, y compris ceux retirés. Cette recherche couvre le coffre entier ; elle ne se limite pas au membre choisi. Les libellés datés suivent le {context.date}.</p>
      <label htmlFor="global-search-query">Nom, description documentaire ou identifiant</label>
      <input id="global-search-query" type="search" autoFocus maxLength={200} value={query} onChange={event => { setQuery(event.currentTarget.value); setLimit(30); }} />
      {loading && <p role="status">Chargement des objets et documents…</p>}
      {error && <div role="alert"><p>{error}</p><button type="button" className="button secondary" onClick={() => setRetry(current => current + 1)}>Recharger la recherche</button></div>}
      {!loading && !error && library && (query.trim().length < 2 ? <p className="small">Saisir au moins deux caractères.</p> : <>
        <p role="status">{results.length} résultat{results.length > 1 ? 's' : ''}{results.length > limit ? ` · ${limit} affichés` : ''}.</p>
        <ul className="global-search-results">{results.slice(0, limit).map(entry => <li key={`${entry.kind}-${entry.id}`}><div><strong>{entry.label}</strong><span>{entry.kind}{entry.retired ? ' · retiré' : ''}</span><code>{entry.id}</code></div><button type="button" className="button secondary small-button" disabled={disabled} onClick={() => { onNavigate(entry.page); close(); }}>Ouvrir le module {moduleNames[entry.page]}</button></li>)}</ul>
        {results.length > limit && <button type="button" className="button secondary" onClick={() => setLimit(current => current + 30)}>Afficher les résultats suivants</button>}
      </>)}
    </section>}
  </div>;
}
