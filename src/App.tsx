import { lazy, Suspense, useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { Dispatch, FormEvent, ReactNode, SetStateAction } from 'react';
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
import GuidedEntry from './features/guided-entry';
import type { GuidedKind } from './features/guided-entry';
import Onboarding from './features/onboarding';
import type { TourDestination, TourStepId } from './features/discovery-tour';
import { entityAt, today } from './features/registry-ui';
import type { ViewContext } from './features/registry-ui';
import type { RegistrySnapshot } from './services/registry-types';
import type { RegistryHistory } from './services/phase4-types';

const MetalsPage = lazy(() => import('./features/metals'));
const CompaniesPage = lazy(() => import('./features/companies'));
const DocumentsPage = lazy(() => import('./features/documents'));
const PortabilityPage = lazy(() => import('./features/portability'));
const DashboardPage = lazy(() => import('./features/dashboard'));
const DiscoveryTour = lazy(() => import('./features/discovery-tour'));

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
  ['help', 'Aide', '?'],
  ['settings', 'Paramètres', '⚙'],
] as const;
type Page = (typeof sections)[number][0];
type CashSection = 'exchange' | 'cash';
type BusyListener = (busy: boolean) => void;

const navigationGroups: { label?: string; pages: Page[] }[] = [
  { pages: ['overview', 'family', 'property', 'bank'] },
  { label: 'Autres biens et dettes', pages: ['cash', 'metals', 'companies', 'debt', 'assets'] },
  { pages: ['backup', 'help'] },
  { label: 'Outils et réglages', pages: ['documents', 'history', 'settings'] },
];

const pageDescriptions: Record<Page, string> = {
  overview: 'Consultez vos biens, vos dettes et le total qui en résulte.',
  family: 'Retrouvez les membres de votre famille et les personnes liées à vos biens.',
  assets: 'Retrouvez vos biens, leurs propriétaires et les valeurs enregistrées.',
  property: 'Retrouvez vos biens, leurs estimations et les loyers qui leur sont liés.',
  bank: 'Suivez le solde et les mouvements de vos comptes.',
  cash: 'Suivez vos réserves d’espèces et enregistrez les taux utiles à vos conversions.',
  metals: 'Retrouvez vos lots, leur poids de métal pur et les cours enregistrés.',
  companies: 'Consultez les sociétés et les parts détenues par chaque associé.',
  debt: 'Suivez les montants empruntés ou prêtés et les remboursements.',
  documents: 'Conservez vos justificatifs dans votre coffre et reliez-les à un bien ou une personne.',
  history: 'Retrouvez les modifications enregistrées et consultez un état antérieur du coffre.',
  backup: 'Protégez votre coffre avec une copie complète ou exportez les données pour les consulter ailleurs.',
  help: 'Apprenez à utiliser Patrimoine ou reprenez vos premières saisies.',
  settings: 'Personnalisez le nom de votre espace et sa devise d’affichage.',
};

const pageHelp: Partial<Record<Page, TourStepId>> = {
  overview: 'overview', family: 'family', property: 'property', bank: 'bank',
  cash: 'bank', metals: 'metals', companies: 'company', assets: 'property', backup: 'backup',
};

const guidedPages: Partial<Record<Page, { kind: GuidedKind; label: string }>> = {
  family: { kind: 'family', label: 'Ajouter un membre de la famille' },
  property: { kind: 'property', label: 'Ajouter un bien immobilier' },
  bank: { kind: 'bank', label: 'Ajouter un compte bancaire' },
  cash: { kind: 'cash', label: 'Ajouter une réserve d’espèces' },
  metals: { kind: 'metals', label: 'Ajouter un lot de métal' },
  companies: { kind: 'company', label: 'Ajouter une société' },
};

function sectionTitle(page: Page) {
  return sections.find(([id]) => id === page)?.[1] ?? 'Patrimoine';
}

function Navigation({ page, busy, onNavigate }: { page: Page; busy: boolean; onNavigate: (page: Page) => void }) {
  const [choices, setChoices] = useState<Record<string, { page: Page; open: boolean }>>({});
  return <nav aria-label="Navigation principale">
    {navigationGroups.map(({ label, pages }) => {
      const items = pages.map(id => {
        const section = sections.find(([sectionId]) => sectionId === id)!;
        return <button key={id} type="button" disabled={busy} onClick={() => onNavigate(id)} aria-current={page === id ? 'page' : undefined} className={page === id ? 'nav-item active' : 'nav-item'}><span aria-hidden="true">{section[2]}</span>{section[1]}</button>;
      });
      if (!label) return <div className="nav-group" key={pages[0]}>{items}</div>;
      const current = pages.includes(page);
      const choice = choices[label];
      const open = choice?.page === page ? choice.open : current || !!choice?.open;
      const groupId = `nav-group-${pages[0]}`;
      return <div className="nav-group" key={label}>
        <button type="button" className="nav-item nav-disclosure" aria-expanded={open} aria-controls={groupId} disabled={busy} onClick={() => setChoices(groups => ({ ...groups, [label]: { page, open: !open } }))}>
          <span aria-hidden="true">{open ? '▾' : '›'}</span>{label}
        </button>
        <div className="nav-children" id={groupId} hidden={!open}>{items}</div>
      </div>;
    })}
  </nav>;
}

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
  const [navigationTrail, setNavigationTrail] = useState<{ page: Page; cashSection: CashSection }[]>([]);
  const [cashSection, setCashSection] = useState<CashSection>('cash');
  const [status, setStatus] = useState<VaultStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [startupError, setStartupError] = useState('');
  const [operationPending, setOperationPending] = useState(false);
  const [entryPending, setEntryPending] = useState(false);
  const [onboardingRequest, setOnboardingRequest] = useState(0);
  const [tourStep, setTourStep] = useState<TourStepId | null>(null);
  const [estateStart, setEstateStart] = useState<{ tab: 'properties' | 'leases'; revision: number }>({ tab: 'properties', revision: 0 });
  const busy = operationPending || entryPending;
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
  useLayoutEffect(() => { headingRef.current?.focus(); }, [page]);

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
    setOnboardingRequest(0);
    setContext(current => ({ ...current, knowledgeSequence: null }));
    await refreshAll();
  }, [refreshAll]);
  useEffect(() => { void refresh(); void refreshRegistry().catch(() => undefined); void refreshHistory(); }, [refresh, refreshRegistry, refreshHistory]);
  const navigate = (next: Page, section: CashSection = 'cash') => {
    if (next === 'cash') setCashSection(section);
    if (next === 'property' && page !== 'property') setEstateStart({ tab: 'properties', revision: 0 });
    if (next !== page) setNavigationTrail(trail => [...trail, { page, cashSection }]);
    setPage(next);
  };
  const previous = navigationTrail.at(-1);
  function goBack() {
    if (!previous) return;
    setNavigationTrail(trail => trail.slice(0, -1));
    setCashSection(previous.cashSection);
    setPage(previous.page);
  }
  const navigateSearch = (next: string) => {
    if (next === 'cash-accounts') navigate('cash');
    else if (next === 'cash') navigate('cash', 'exchange');
    else if (sections.some(([id]) => id === next)) navigate(next as Page);
  };
  function followTour(destination: TourDestination) {
    if (destination === 'property' || destination === 'rentals') {
      navigate('property');
      setEstateStart(current => ({ tab: destination === 'rentals' ? 'leases' : 'properties', revision: current.revision + 1 }));
      return;
    }
    navigate(destination);
  }
  const readableRegistry = !registryError && !knowledgeLoading && registry && (context.knowledgeSequence == null || registry.knowledge_sequence === context.knowledgeSequence) ? registry : null;
  const registryPage = page === 'family' || page === 'companies' || page === 'assets' || page === 'cash' || page === 'property' || page === 'bank' || page === 'debt' || page === 'metals';
  const entryAction = page === 'cash' && cashSection !== 'cash' ? undefined : guidedPages[page];
  const moduleProps = readableRegistry ? { registry: readableRegistry, context, onChanged: afterMutation, onBusyChange: setOperationPending } : null;
  const openAssets = () => navigate('assets');
  const openBank = () => navigate('bank');
  const configure = () => setOnboardingRequest(request => request + 1);
  const canConfigure = !!status && !!readableRegistry && !status.recovery_required && !busy && !registryLoading && context.knowledgeSequence == null;
  const helpStep = pageHelp[page];

  return <>
    <div className="app-shell">
      <a className="skip-link" href="#content">Aller au contenu</a>
      <aside className="sidebar">
        <div className="brand"><span className="brand-symbol" aria-hidden="true">P</span><div>Patrimoine</div></div>
        <div className="workspace"><strong>{status?.settings.workspace_name || 'Mon patrimoine familial'}</strong></div>
        <Navigation page={page} busy={busy} onNavigate={navigate} />
        <div className="sidebar-footer"><span className="status-dot" aria-hidden="true" />Stockage local</div>
      </aside>
      <div className="main-shell">
        <header className="topbar">
          {previous && <button type="button" className="text-button" disabled={busy} onClick={goBack}><span aria-hidden="true">← </span>Retour à {sectionTitle(previous.page)}</button>}
          {registry && !registryLoading && !knowledgeLoading && <GlobalSearch registry={registry} context={context} disabled={busy} onNavigate={navigateSearch} />}
        </header>
        <main id="content" tabIndex={-1} aria-busy={busy || loading}>
          <div className="page-heading">
            <div><h1 ref={headingRef} tabIndex={-1}>{sectionTitle(page)}</h1><p>{pageDescriptions[page]}</p></div>
            {helpStep && <button type="button" className="button secondary small-button page-help" disabled={busy} onClick={() => setTourStep(helpStep)}>Aide sur cette rubrique</button>}
          </div>
          {loading && <p role="status" className="muted">Ouverture du coffre local…</p>}
          {writeMessage && <Notice kind="success">{writeMessage}</Notice>}
          {status?.recovery_required && <Notice kind="error">{status.recovery_message || 'Le coffre actif a été chargé. Fermez et rouvrez l’application pour terminer la reprise.'}</Notice>}
          {startupError && <Notice kind="error"><strong>Le coffre local est inaccessible.</strong><p>{startupError}</p><p>Ouvrez l’application desktop Tauri pour accéder aux données locales.</p><button type="button" className="button secondary" onClick={() => { void refresh(); }} disabled={loading}>Réessayer</button></Notice>}
          {page === 'help' && <section className="card help-actions" aria-label="Aide de Patrimoine">
            <div><h2>Tour d’horizon illustré</h2><p>Explorez les principales rubriques à votre rythme avec des exemples.</p><button type="button" className="button" disabled={busy} onClick={() => setTourStep('overview')}>Découvrir Patrimoine</button></div>
            <div><h2>Configuration guidée</h2><p>Retrouvez les étapes facultatives pour saisir vos premières fiches.</p><button type="button" className="button secondary" disabled={!canConfigure} onClick={configure}>Reprendre la configuration</button>
              {context.knowledgeSequence != null && <p className="field-hint">Revenez aux données actuelles pour enregistrer des fiches. <button type="button" className="text-button" disabled={busy || knowledgeLoading} onClick={() => chooseSequence(null)}>Revenir aux données actuelles</button></p>}
            </div>
          </section>}
          {status && page !== 'help' && <>
            <ViewControls registry={registry} context={context} history={history} busy={busy} loading={registryLoading} setContext={setContext} chooseSequence={chooseSequence} />
            {context.knowledgeSequence != null && <Notice>Lecture seule historique · séquence {context.knowledgeSequence}. Les paramètres, sauvegardes et exports intégraux concernent toujours le coffre actuel. <button type="button" className="text-button" disabled={busy || knowledgeLoading} onClick={() => chooseSequence(null)}>Revenir aux données actuelles</button></Notice>}
            {readableRegistry && entryAction && !status.recovery_required && <PageEntry key={`${status.generation_id}:${page}:${context.personId}:${context.date}:${context.knowledgeSequence}`} action={entryAction} registry={readableRegistry} date={context.date} displayCurrency={status.settings.display_currency} disabled={busy || registryLoading || context.knowledgeSequence != null} onHelp={page === 'property' ? () => setTourStep('property') : undefined} onSaved={afterMutation} onBusyChange={setEntryPending} />}
            {page === 'overview' && <>
              {registryLoading && <p role="status">Chargement du périmètre patrimonial…</p>}
              {registryError && <Notice kind="error"><strong>Le périmètre est inaccessible.</strong><p>{registryError}</p><button type="button" className="button secondary" onClick={() => { void refreshRegistry().catch(() => undefined); }}>Recharger le périmètre</button></Notice>}
              {registry && !registryError && !registryLoading && !knowledgeLoading && <Suspense fallback={<p role="status">Ouverture du tableau de bord…</p>}><DashboardPage registry={registry} context={context} status={status} onNavigate={navigateSearch} /></Suspense>}
            </>}
            {page === 'settings' && <>
              <SettingsPage draft={settingsDraft} onDraftChange={setSettingsDraft} onSaved={afterMutation} onBusyChange={setOperationPending} />
              {registry && !status.recovery_required && <section className="card settings-guide">
                <h2>Configuration guidée</h2><p>Reprenez vos premières saisies, une étape à la fois.</p>
                <button type="button" className="button secondary" disabled={!canConfigure} onClick={configure}>Reprendre la configuration</button>
              </section>}
            </>}
            {page === 'documents' && <Suspense fallback={<p role="status">Ouverture du gestionnaire documentaire…</p>}><DocumentsPage context={context} onChanged={afterMutation} onBusyChange={setOperationPending} /></Suspense>}
            {page === 'backup' && <Suspense fallback={<p role="status">Ouverture de la portabilité du coffre…</p>}><PortabilityPage currentSchema={status.schema_version} onRestored={afterRestore} onBusyChange={setOperationPending} /></Suspense>}
            {page === 'history' && <HistoryPage history={history} historyError={historyError} context={context} onChooseSequence={chooseSequence} onReload={refreshHistory} onBusyChange={setOperationPending} />}
            {registryPage && <>
              {registryLoading && <p role="status">Chargement du registre local…</p>}
              {registryError && <Notice kind="error"><strong>Le registre est inaccessible.</strong><p>{registryError}</p><button type="button" className="button secondary" disabled={operationPending || registryLoading} onClick={() => { void refreshRegistry().catch(() => undefined); }}>Recharger le registre</button></Notice>}
              {moduleProps && <>
                {page === 'cash' && <CashWorkspace {...moduleProps} section={cashSection} onSectionChange={setCashSection} entryPending={entryPending} onNavigateRegistry={openAssets} />}
                {page === 'bank' && <TreasuryPage {...moduleProps} mode="bank" onNavigateRegistry={openAssets} />}
                {page === 'debt' && <DebtsPage {...moduleProps} onNavigateTreasury={openBank} />}
                {page === 'property' && <RealEstatePage {...moduleProps} key={estateStart.revision} initialTab={estateStart.tab} onHelpRentals={() => setTourStep('rentals')} onNavigateFamily={() => navigate('family')} onNavigateRegistry={openAssets} />}
                {page === 'metals' && <Suspense fallback={<p role="status">Ouverture du module métaux…</p>}><MetalsPage {...moduleProps} onNavigateRegistry={openAssets} /></Suspense>}
                {page === 'companies' && <CompaniesWorkspace {...moduleProps} onNavigateTreasury={openBank} />}
                {page === 'assets' && <AssetsWorkspace {...moduleProps} />}
                {page === 'family' && <RegistryPage {...moduleProps} page="family" />}
              </>}
            </>}
          </>}
          {!status && !loading && !startupError && <Notice>Aucun coffre n’a pu être chargé.</Notice>}
        </main>
        <footer className="main-footer"><span>Vos données restent sur votre ordinateur.</span><span>{status ? `Version ${status.app_version}` : 'Patrimoine'}</span></footer>
      </div>
    </div>
    {status && registry && !status.recovery_required && <Onboarding status={status} registry={registry} openRequested={onboardingRequest} onSaved={afterMutation} onComplete={() => navigate('overview')} onDiscover={() => { navigate('overview'); setTourStep('overview'); }} />}
    {tourStep && <Suspense fallback={<p role="status">Ouverture de l’aide illustrée…</p>}><DiscoveryTour key={tourStep} initialStep={tourStep} onClose={() => setTourStep(null)} onNavigate={followTour} /></Suspense>}
  </>;
}

function ViewControls({ registry, context, history, busy, loading, setContext, chooseSequence }: {
  registry: RegistrySnapshot | null;
  context: ViewContext;
  history: RegistryHistory | null;
  busy: boolean;
  loading: boolean;
  setContext: Dispatch<SetStateAction<ViewContext>>;
  chooseSequence: (sequence: number | null) => void;
}) {
  const currentSequence = context.knowledgeSequence ?? null;
  const sequences = new Set(history?.events.map(event => event.sequence));
  if (currentSequence !== null) sequences.add(currentSequence);
  sequences.delete(0);
  const people = registry?.entities.filter(entity => entity.kind === 'person') ?? [];
  return <section className="context-bar" aria-label="Contexte patrimonial">
    <div className="view-owner">
      <label htmlFor="global-context">Périmètre de consultation</label>
      <select id="global-context" disabled={busy || loading || !registry} value={context.personId || 'family'} onChange={event => {
        const personId = event.currentTarget.value === 'family' ? null : event.currentTarget.value;
        setContext(current => ({ ...current, personId }));
      }}>
        <option value="family">Patrimoine familial</option>
        {registry && people.map(entity => <option key={entity.id} value={entity.id}>{entityAt(registry, entity, context.date).display_name}{entity.archived ? ' (archivé)' : ''}</option>)}
      </select>
    </div>
    <details className="view-history">
      <summary>{context.date !== today() ? `Consultation au ${context.date}` : 'Date et historique'}</summary>
      <div className="view-history-fields">
        <div><label htmlFor="global-date">Date de référence</label><input id="global-date" type="date" value={context.date} aria-describedby="view-date-help" disabled={busy} onChange={event => {
          const date = event.currentTarget.value;
          if (date) setContext(current => ({ ...current, date }));
        }} /><p id="view-date-help">Date pour laquelle vous voulez voir la valeur de vos biens et le montant de vos dettes.</p></div>
        <div><label htmlFor="global-knowledge">Connaissances disponibles</label><select id="global-knowledge" disabled={busy || loading} aria-describedby="view-knowledge-help" value={currentSequence ?? 'current'} onChange={event => chooseSequence(event.currentTarget.value === 'current' ? null : Number(event.currentTarget.value))}>
          <option value="current">Connaissances actuelles</option><option value="0">Séquence 0 · état initial</option>
          {[...sequences].sort((a, b) => b - a).map(sequence => <option key={sequence} value={sequence}>Séquence {sequence} · lecture seule</option>)}
        </select><p id="view-knowledge-help">Pour retrouver ce qui était connu au moment d’une modification. Les états précédents s’ouvrent en lecture seule.</p></div>
      </div>
    </details>
  </section>;
}

function PageEntry({ action, registry, date, displayCurrency, disabled, onHelp, onSaved, onBusyChange }: {
  action: { kind: GuidedKind; label: string };
  registry: RegistrySnapshot;
  date: string;
  displayCurrency: string;
  disabled: boolean;
  onHelp?: () => void;
  onSaved: () => Promise<void>;
  onBusyChange: BusyListener;
}) {
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const restoreFocus = useRef(false);
  useLayoutEffect(() => { if (open) heading.current?.focus(); }, [open]);
  useLayoutEffect(() => {
    if (!open && !disabled && restoreFocus.current) {
      trigger.current?.focus();
      restoreFocus.current = false;
    }
  }, [open, disabled]);
  const close = () => { restoreFocus.current = true; setOpen(false); };
  return <div className="page-entry">
    {!open ? <button ref={trigger} type="button" className="button" disabled={disabled} onClick={() => setOpen(true)}><span aria-hidden="true">+</span>{action.label}</button> :
      <section className="card form-card" aria-labelledby="guided-entry-title">
        <div className="section-title"><h2 id="guided-entry-title" ref={heading} tabIndex={-1}>{action.label}</h2><button type="button" className="button secondary small-button" disabled={disabled} onClick={close}>Annuler</button></div>
        <GuidedEntry kind={action.kind} registry={registry} date={date} displayCurrency={displayCurrency} onBusyChange={onBusyChange} onSaved={async () => { await onSaved(); close(); }} />
        {onHelp && <button type="button" className="text-button" disabled={disabled} onClick={onHelp}>Comment fonctionne la propriété partagée ?</button>}
      </section>}
  </div>;
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

function CashWorkspace({ section, onSectionChange, entryPending, ...props }: WorkspaceProps & { onNavigateRegistry: () => void; section: CashSection; onSectionChange: (section: CashSection) => void; entryPending: boolean }) {
  const [busy, setBusy] = useState(false);
  const reportBusy = useCallback((value: boolean) => { setBusy(value); props.onBusyChange(value); }, [props.onBusyChange]);
  return <><div className="module-tabs"><button type="button" className={`button ${section === 'cash' ? '' : 'secondary'}`} disabled={busy || entryPending} aria-pressed={section === 'cash'} onClick={() => onSectionChange('cash')}>Caisses physiques</button><button type="button" className={`button ${section === 'exchange' ? '' : 'secondary'}`} disabled={busy || entryPending} aria-pressed={section === 'exchange'} onClick={() => onSectionChange('exchange')}>Taux et conversions</button></div>{section === 'exchange' ? <ExchangeRatesPage {...props} onBusyChange={reportBusy} /> : <TreasuryPage mode="cash" {...props} onBusyChange={reportBusy} />}</>;
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
