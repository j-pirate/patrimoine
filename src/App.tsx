import { lazy, Suspense, useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { FormEvent, ReactNode } from 'react';
import { api, displayCurrencies, errorMessage } from './services/api';
import { refreshAfterCommit } from './services/mutations';
import type { Settings, VaultStatus } from './services/api';
import RegistryPage from './features/registry';
import ExchangeRatesPage from './features/exchange-rates';
import HistoryPage from './features/history';
import RealEstatePage from './features/real-estate';
import TreasuryPage from './features/treasury';
import DebtsPage from './features/debts';
import FinancialPanel from './features/financial-report';
import GlobalSearch from './features/global-search';
import { entityAt, today } from './features/registry-ui';
import type { ViewContext } from './features/registry-ui';
import type { RegistrySnapshot } from './services/registry-types';
import type { RegistryHistory } from './services/phase4-types';

const MetalsPage = lazy(() => import('./features/metals'));
const CompaniesPage = lazy(() => import('./features/companies'));
const DocumentsPage = lazy(() => import('./features/documents'));
const PortabilityPage = lazy(() => import('./features/portability'));
const DashboardPage = lazy(() => import('./features/dashboard'));

const sections = [
  ['overview', 'Tableau de bord', '⌂'],
  ['family', 'Famille', '♧'],
  ['assets', 'Patrimoine', '◇'],
  ['property', 'Immobilier et loyers', '▤'],
  ['bank', 'Comptes bancaires', '▥'],
  ['cash', 'Espèces et devises', '⇄'],
  ['metals', 'Or et argent', '◈'],
  ['companies', 'Sociétés', '▦'],
  ['debt', 'Dettes et financements', '≋'],
  ['documents', 'Documents', '▱'],
  ['history', 'Historique', '▧'],
  ['backup', 'Sauvegarde et export', '↥'],
  ['settings', 'Paramètres', '⚙'],
] as const;
type Page = (typeof sections)[number][0];
type BusyListener = (busy: boolean) => void;

function useOperationBusy(onBusyChange: BusyListener) {
  const [pending, setPending] = useState(false);
  useEffect(() => {
    onBusyChange(pending);
    return () => onBusyChange(false);
  }, [pending, onBusyChange]);
  return [pending, setPending] as const;
}

function Notice({ children, kind = 'info' }: { children: ReactNode; kind?: 'info' | 'success' | 'error' }) {
  return <div className={`notice notice-${kind}`} role={kind === 'error' ? 'alert' : 'status'}>{children}</div>;
}

export default function App() {
  const [page, setPage] = useState<Page>('overview');
  const [cashSection, setCashSection] = useState<'exchange' | 'cash'>('exchange');
  const [status, setStatus] = useState<VaultStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [startupError, setStartupError] = useState('');
  const [operationPending, setOperationPending] = useState(false);
  const [writeMessage, setWriteMessage] = useState('');
  const [settingsDraft, setSettingsDraft] = useState<Settings | null>(null);
  const [registry, setRegistry] = useState<RegistrySnapshot | null>(null);
  const [registryError, setRegistryError] = useState('');
  const [registryLoading, setRegistryLoading] = useState(false);
  const [knowledgeLoading, setKnowledgeLoading] = useState(false);
  const [history, setHistory] = useState<RegistryHistory | null>(null);
  const [historyError, setHistoryError] = useState('');
  const [context, setContext] = useState<ViewContext>({ personId: null, date: today(), knowledgeSequence: null });
  const knowledgeRef = useRef<number | null>(null);
  const registryRequest = useRef(0);
  const knowledgeRequest = useRef(0);
  const headingRef = useRef<HTMLHeadingElement>(null);
  useLayoutEffect(() => { headingRef.current?.focus(); }, [page, cashSection]);

  const refresh = useCallback(async (reportFailure = false) => {
    setLoading(true);
    setStartupError('');
    try { setStatus(await api.getStatus()); }
    catch (error) { setStartupError(errorMessage(error)); if (reportFailure) throw error; }
    finally { setLoading(false); }
  }, []);

  const refreshRegistry = useCallback(async () => {
    const request = ++registryRequest.current;
    const knowledge = knowledgeRef.current;
    setRegistryLoading(true);
    setRegistryError('');
    try {
      const snapshot = knowledge === null ? await api.listRegistry() : await api.listRegistryAt(knowledge);
      if (request !== registryRequest.current) return;
      setRegistry(snapshot);
      setContext(current => current.personId && !snapshot.entities.some(entity => entity.kind === 'person' && entity.id === current.personId)
        ? { ...current, personId: null } : current);
    }
    catch (error) { if (request === registryRequest.current) setRegistryError(errorMessage(error)); throw error; }
    finally { if (request === registryRequest.current) setRegistryLoading(false); }
  }, []);
  const refreshHistory = useCallback(async (reportFailure = false) => {
    setHistoryError('');
    try { setHistory(await api.listRegistryHistory()); }
    catch (error) { setHistoryError(errorMessage(error)); if (reportFailure) throw error; }
  }, []);
  const refreshAll = useCallback(async () => {
    const results = await Promise.allSettled([refresh(true), refreshRegistry(), refreshHistory(true)]);
    const failed = results.find(result => result.status === 'rejected');
    if (failed?.status === 'rejected') throw failed.reason;
  }, [refresh, refreshRegistry, refreshHistory]);
  const chooseSequence = useCallback((sequence: number | null) => {
    const request = ++knowledgeRequest.current;
    knowledgeRef.current = sequence;
    setKnowledgeLoading(true);
    setContext(current => ({ ...current, knowledgeSequence: sequence }));
    void refreshRegistry().catch(() => undefined).finally(() => { if (request === knowledgeRequest.current) setKnowledgeLoading(false); });
  }, [refreshRegistry]);
  const afterMutation = useCallback(() => refreshAfterCommit(refreshAll, 'Les changements ont été enregistrés dans le coffre.', setWriteMessage), [refreshAll]);
  const afterRestore = useCallback(async () => {
    ++knowledgeRequest.current;
    setKnowledgeLoading(false);
    setSettingsDraft(null);
    setWriteMessage('');
    knowledgeRef.current = null;
    setRegistry(null);
    setContext(current => ({ ...current, knowledgeSequence: null }));
    await refreshAll();
  }, [refreshAll]);
  useEffect(() => { void refresh(); void refreshRegistry().catch(() => undefined); void refreshHistory(); }, [refresh, refreshRegistry, refreshHistory]);
  const navigate = (next: Page) => {
    if (next === 'cash') setCashSection('exchange');
    setPage(next);
  };
  const navigateSearch = (next: string) => {
    if (next === 'cash-accounts') {
      setCashSection('cash'); setPage('cash');
    } else if (sections.some(([id]) => id === next)) navigate(next as Page);
  };
  const title = sections.find(([id]) => id === page)?.[1] ?? 'Patrimoine';
  const registryPage = page === 'family' || page === 'companies' || page === 'assets' || page === 'cash' || page === 'property' || page === 'bank' || page === 'debt' || page === 'metals';

  return (
    <div className="app-shell">
      <a className="skip-link" href="#content">Aller au contenu</a>
      <aside className="sidebar">
        <div className="brand"><span className="brand-symbol" aria-hidden="true">P</span><div>Patrimoine<small>Un espace pour votre famille</small></div></div>
        <div className="workspace"><span className="eyebrow">ESPACE LOCAL</span><strong>{status?.settings.workspace_name || 'Mon patrimoine familial'}</strong><span>Administration familiale</span></div>
        <nav aria-label="Navigation principale">
          {sections.map(([id, label, symbol]) => <button key={id} type="button" disabled={operationPending} onClick={() => navigate(id)} aria-current={page === id ? 'page' : undefined} className={page === id ? 'nav-item active' : 'nav-item'}><span aria-hidden="true">{symbol}</span>{label}</button>)}
        </nav>
        <div className="sidebar-footer"><span className="status-dot" aria-hidden="true" />Données locales<span>Sans compte externe</span></div>
      </aside>
      <div className="main-shell">
        <header className="topbar"><span>Gestion patrimoniale familiale</span>{registry && !registryLoading && !knowledgeLoading && <GlobalSearch registry={registry} context={context} disabled={operationPending} onNavigate={navigateSearch} />}<span className="local-label">Linux · stockage local</span></header>
        <main id="content" tabIndex={-1} aria-busy={operationPending || loading}>
          <div className="page-heading"><div><span className="eyebrow">VOTRE ESPACE PATRIMONIAL</span><h1 ref={headingRef} tabIndex={-1}>{title}</h1></div><span className="badge">Données locales</span></div>
          {loading && <p role="status" className="muted">Ouverture du coffre local…</p>}
          {writeMessage && <Notice kind="success">{writeMessage}</Notice>}
          {status?.recovery_required && <Notice kind="error">{status.recovery_message || 'Le coffre actif a été chargé. Fermez et rouvrez l’application pour terminer la reprise.'}</Notice>}
          {startupError && <Notice kind="error"><strong>Le coffre local est inaccessible.</strong><p>{startupError}</p><p>Ouvrez l’application desktop Tauri pour accéder aux données locales.</p><button type="button" className="button secondary" onClick={() => { void refresh(); }} disabled={loading}>Réessayer</button></Notice>}
          {status && <>
            <section className="context-bar" aria-label="Contexte patrimonial">
              <div><label htmlFor="global-context">Périmètre de consultation</label><select id="global-context" disabled={operationPending || registryLoading || !registry} value={context.personId || 'family'} onChange={event => { const personId = event.currentTarget.value === 'family' ? null : event.currentTarget.value; setContext(current => ({ ...current, personId })); }}><option value="family">Patrimoine familial</option>{registry?.entities.filter(entity => entity.kind === 'person').map(entity => <option key={entity.id} value={entity.id}>{entityAt(registry, entity, context.date).display_name}{entity.archived ? ' (archivé)' : ''}</option>)}</select></div>
              <div><label htmlFor="global-date">Date de référence</label><input id="global-date" type="date" value={context.date} disabled={operationPending} onChange={event => { const date = event.currentTarget.value; if (date) setContext(current => ({ ...current, date })); }} /></div>
              <div><label htmlFor="global-knowledge">Connaissances disponibles</label><select id="global-knowledge" disabled={operationPending || registryLoading} value={context.knowledgeSequence ?? 'current'} onChange={event => chooseSequence(event.currentTarget.value === 'current' ? null : Number(event.currentTarget.value))}><option value="current">Connaissances actuelles</option><option value="0">Séquence 0 · état initial</option>{[...new Set([...(history?.events.map(event => event.sequence) ?? []), ...(context.knowledgeSequence != null && context.knowledgeSequence !== 0 ? [context.knowledgeSequence] : [])])].filter(sequence => sequence !== 0).sort((a, b) => b - a).map(sequence => <option key={sequence} value={sequence}>Séquence {sequence} · lecture seule</option>)}</select></div>
              <p>La date économique D et les connaissances K sont indépendantes. Les calculs et métadonnées suivent ces deux références. Les anciennes connaissances sont consultables en lecture seule.</p>
            </section>
            {context.knowledgeSequence != null && <Notice>Lecture seule historique · séquence {context.knowledgeSequence}. Les paramètres, sauvegardes et exports intégraux concernent toujours le coffre actuel.</Notice>}
            {page === 'overview' && <>
              {registryLoading && <p role="status">Chargement du périmètre patrimonial…</p>}
              {registryError && <Notice kind="error"><strong>Le périmètre est inaccessible.</strong><p>{registryError}</p><button type="button" className="button secondary" onClick={() => { void refreshRegistry().catch(() => undefined); }}>Recharger le périmètre</button></Notice>}
              {registry && !registryError && !registryLoading && !knowledgeLoading && <Suspense fallback={<p role="status">Ouverture du tableau de bord…</p>}><DashboardPage registry={registry} context={context} status={status} onNavigate={next => { if (sections.some(([id]) => id === next)) navigate(next as Page); }} /></Suspense>}
            </>}
            {page === 'settings' && <SettingsPage draft={settingsDraft} onDraftChange={setSettingsDraft} onSaved={afterMutation} onBusyChange={setOperationPending} />}
            {page === 'documents' && <Suspense fallback={<p role="status">Ouverture du gestionnaire documentaire…</p>}><DocumentsPage context={context} onChanged={afterMutation} onBusyChange={setOperationPending} /></Suspense>}
            {page === 'backup' && <Suspense fallback={<p role="status">Ouverture de la portabilité du coffre…</p>}><PortabilityPage currentSchema={status.schema_version} onRestored={afterRestore} onBusyChange={setOperationPending} /></Suspense>}
            {page === 'history' && <HistoryPage history={history} historyError={historyError} context={context} onChooseSequence={chooseSequence} onReload={refreshHistory} onBusyChange={setOperationPending} />}
            {registryPage && <>
              {registryLoading && <p role="status">Chargement du registre local…</p>}
              {registryError && <Notice kind="error"><strong>Le registre est inaccessible.</strong><p>{registryError}</p><button type="button" className="button secondary" disabled={operationPending || registryLoading} onClick={() => { void refreshRegistry().catch(() => undefined); }}>Recharger le registre</button></Notice>}
              {registry && !registryError && !knowledgeLoading && (context.knowledgeSequence == null || registry.knowledge_sequence === context.knowledgeSequence) && (page === 'cash' ? <CashWorkspace key={cashSection} initialSection={cashSection} registry={registry} context={context} onChanged={afterMutation} onBusyChange={setOperationPending} onNavigateRegistry={() => navigate('assets')} /> : page === 'bank' ? <TreasuryPage mode="bank" registry={registry} context={context} onChanged={afterMutation} onBusyChange={setOperationPending} onNavigateRegistry={() => navigate('assets')} /> : page === 'debt' ? <DebtsPage registry={registry} context={context} onChanged={afterMutation} onBusyChange={setOperationPending} onNavigateTreasury={() => navigate('bank')} /> : page === 'property' ? <RealEstatePage registry={registry} context={context} onChanged={afterMutation} onBusyChange={setOperationPending} onNavigateRegistry={() => navigate('assets')} /> : page === 'metals' ? <Suspense fallback={<p role="status">Ouverture du module métaux…</p>}><MetalsPage registry={registry} context={context} onChanged={afterMutation} onBusyChange={setOperationPending} onNavigateRegistry={() => navigate('assets')} /></Suspense> : page === 'companies' ? <CompaniesWorkspace registry={registry} context={context} onChanged={afterMutation} onBusyChange={setOperationPending} onNavigateTreasury={() => navigate('bank')} /> : page === 'assets' ? <AssetsWorkspace registry={registry} context={context} onChanged={afterMutation} onBusyChange={setOperationPending} /> : <RegistryPage key={page} page="family" registry={registry} context={context} onChanged={afterMutation} onBusyChange={setOperationPending} />)}
            </>}
          </>}
          {!status && !loading && !startupError && <Notice>Aucun coffre n’a pu être chargé.</Notice>}
        </main>
        <footer className="main-footer"><span>Vos données restent sur votre ordinateur.</span><span>{status ? `Version ${status.app_version} · Schéma ${status.schema_version}` : 'Patrimoine'}</span></footer>
      </div>
    </div>
  );
}

interface WorkspaceProps {
  registry: RegistrySnapshot; context: ViewContext;
  onChanged: () => Promise<void>; onBusyChange: (busy: boolean) => void;
}

function AssetsWorkspace(props: WorkspaceProps) {
  const [registryBusy, setRegistryBusy] = useState(false);
  const [financeBusy, setFinanceBusy] = useState(false);
  useEffect(() => { props.onBusyChange(registryBusy || financeBusy); return () => props.onBusyChange(false); }, [registryBusy, financeBusy, props.onBusyChange]);
  return <div className="registry-workspace"><RegistryPage page="assets" {...props} onBusyChange={setRegistryBusy} /><FinancialPanel {...props} onBusyChange={setFinanceBusy} /></div>;
}

function CompaniesWorkspace(props: WorkspaceProps & { onNavigateTreasury: () => void }) {
  const [section, setSection] = useState<'registry' | 'indicators'>('registry');
  const [busy, setBusy] = useState(false);
  useEffect(() => { props.onBusyChange(busy); return () => props.onBusyChange(false); }, [busy, props.onBusyChange]);
  return <><div className="module-tabs"><button type="button" className={`button ${section === 'registry' ? '' : 'secondary'}`} disabled={busy} onClick={() => setSection('registry')}>Fiches et participations</button><button type="button" className={`button ${section === 'indicators' ? '' : 'secondary'}`} disabled={busy} onClick={() => setSection('indicators')}>Indicateurs et dividendes</button></div>{section === 'registry' ? <RegistryPage page="companies" {...props} onBusyChange={setBusy} /> : <Suspense fallback={<p role="status">Ouverture des observations de sociétés…</p>}><CompaniesPage {...props} onBusyChange={setBusy} /></Suspense>}</>;
}

function CashWorkspace(props: WorkspaceProps & { onNavigateRegistry: () => void; initialSection: 'exchange' | 'cash' }) {
  const [section, setSection] = useState<'exchange' | 'cash'>(props.initialSection);
  const [busy, setBusy] = useState(false);
  const reportBusy = useCallback((value: boolean) => { setBusy(value); props.onBusyChange(value); }, [props.onBusyChange]);
  return <><div className="module-tabs"><button type="button" className={`button ${section === 'exchange' ? '' : 'secondary'}`} disabled={busy} onClick={() => setSection('exchange')}>Taux et conversions</button><button type="button" className={`button ${section === 'cash' ? '' : 'secondary'}`} disabled={busy} onClick={() => setSection('cash')}>Caisses physiques</button></div>{section === 'exchange' ? <ExchangeRatesPage {...props} onBusyChange={reportBusy} /> : <TreasuryPage mode="cash" {...props} onBusyChange={reportBusy} />}</>;
}

function SettingsPage({ draft, onDraftChange, onSaved, onBusyChange }: { draft: Settings | null; onDraftChange: (draft: Settings) => void; onSaved: () => Promise<void>; onBusyChange: BusyListener }) {
  const [settings, setSettings] = useState<Settings>(draft || { workspace_name: '', display_currency: 'EUR' });
  const [loading, setLoading] = useState(true);
  const [settingsLoaded, setSettingsLoaded] = useState(false);
  const [saving, setSaving] = useOperationBusy(onBusyChange);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  useEffect(() => {
    let active = true;
    if (draft) { setSettingsLoaded(true); setLoading(false); return; }
    void api.getSettings().then(value => { if (active) { setSettings(value); setSettingsLoaded(true); } }).catch(reason => { if (active) setError(errorMessage(reason)); }).finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, []);
  async function submit(event: FormEvent) {
    event.preventDefault(); setSaving(true); setError(''); setMessage('');
    try {
      const saved = await api.saveSettings({ ...settings, workspace_name: settings.workspace_name.trim() });
      setSettings(saved); onDraftChange(saved);
      await refreshAfterCommit(onSaved, 'Les paramètres ont été enregistrés dans le coffre local.', setMessage);
    }
    catch (reason) { setError(errorMessage(reason)); }
    finally { setSaving(false); }
  }
  return <section className="card form-card"><h2>Préférences de votre espace</h2><p className="muted">Ces réglages sont conservés dans votre base locale et inclus dans les sauvegardes.</p>{error && <Notice kind="error">{error}</Notice>}{message && <Notice kind="success">{message}</Notice>}{loading ? <p role="status">Chargement des paramètres…</p> : settingsLoaded && <form onSubmit={event => { void submit(event); }} aria-busy={saving}><fieldset disabled={saving}><label htmlFor="workspace-name">Nom de l’espace familial</label><input id="workspace-name" required maxLength={100} value={settings.workspace_name} onChange={event => { const workspace_name = event.currentTarget.value; const next = { ...settings, workspace_name }; setSettings(next); onDraftChange(next); }} autoComplete="organization" /><label htmlFor="display-currency">Devise d’affichage préférée</label><select id="display-currency" aria-describedby="currency-help" value={settings.display_currency} onChange={event => { const display_currency = event.currentTarget.value as Settings['display_currency']; const next = { ...settings, display_currency }; setSettings(next); onDraftChange(next); }}>{displayCurrencies.map(currency => <option key={currency}>{currency}</option>)}</select><p id="currency-help" className="field-hint">Cette préférence est conservée pour votre espace. Les vues consolidées restent en EUR ; les saisies et leurs sources conservent leur devise d’origine.</p><div className="fixed-setting"><span>Devise de consolidation</span><strong>EUR</strong></div><button type="submit" className="button" disabled={!settings.workspace_name.trim() || saving}>{saving ? 'Enregistrement…' : 'Enregistrer les paramètres'}</button></fieldset></form>}</section>;
}
