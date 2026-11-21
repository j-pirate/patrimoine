import { useEffect, useId, useLayoutEffect, useRef, useState } from 'react';
import { api, errorMessage } from '../services/api';
import type { VaultStatus } from '../services/api';
import type { RegistrySnapshot } from '../services/registry-types';
import GuidedEntry, { newGuidedDraft } from './guided-entry';
import type { GuidedDraft, GuidedKind, GuidedReceipt } from './guided-entry';
import { today } from './registry-ui';
import './onboarding.css';

const steps = [
  { title: 'Bienvenue dans Patrimoine', description: 'Retrouvez au même endroit les informations dont vous avez besoin pour suivre votre patrimoine familial.' },
  { title: 'Famille', description: 'Ajoutez les membres dont vous souhaitez suivre les biens. Vous pouvez en enregistrer plusieurs.' },
  { title: 'Immobilier', description: 'Ajoutez un bien pour pouvoir suivre sa valeur et, plus tard, ses loyers.' },
  { title: 'Comptes et espèces', description: 'Ajoutez un compte ou indiquez combien vous avez dans une réserve d’espèces au début du suivi.' },
  { title: 'Or et argent', description: 'Décrivez un lot d’objets de même forme, poids et pureté. Vous pourrez ajouter les cours plus tard.' },
  { title: 'Sociétés', description: 'Créez une fiche pour la société, puis renseignez ses associés si vous les connaissez.' },
  { title: 'Votre configuration', description: 'Retrouvez ci-dessous les informations enregistrées pendant la configuration. Vous pourrez les modifier et les compléter depuis les rubriques.' },
] as const;

const entries = [
  { label: 'Famille', kinds: ['family'] },
  { label: 'Immobilier', kinds: ['property'] },
  { label: 'Comptes et espèces', kinds: ['bank', 'cash'] },
  { label: 'Or et argent', kinds: ['metals'] },
  { label: 'Sociétés', kinds: ['company'] },
] as const satisfies readonly { label: string; kinds: readonly GuidedKind[] }[];

type AccountKind = 'bank' | 'cash';
type SavedEntry = GuidedReceipt & { kind: GuidedKind };
interface OnboardingState {
  version: 1;
  disposition: 'new' | 'active' | 'paused' | 'completed' | 'skipped';
  step: number;
  accountKind: AccountKind;
  drafts: Partial<Record<GuidedKind, GuidedDraft>>;
  receipts: SavedEntry[];
}

interface OnboardingProps {
  status: VaultStatus;
  registry: RegistrySnapshot;
  openRequested: number;
  onSaved: () => void | Promise<void>;
  onComplete?: () => void;
  onDiscover?: () => void;
}

function newState(): OnboardingState {
  return { version: 1, disposition: 'new', step: 0, accountKind: 'bank', drafts: {}, receipts: [] };
}

function isKind(value: string): value is GuidedKind {
  return entries.some(entry => (entry.kinds as readonly string[]).includes(value));
}

function isDraft(draft: GuidedDraft | null): draft is GuidedDraft {
  return !!draft && (draft.requestId === null || typeof draft.requestId === 'string') && !!draft.values
    && Object.keys(newGuidedDraft('', '').values).every(key => typeof draft.values[key as keyof GuidedDraft['values']] === 'string')
    && Array.isArray(draft.owners) && draft.owners.every(row => row && [row.id, row.entityId, row.legalPercent, row.economicPercent].every(value => typeof value === 'string'));
}

function parseState(json: string | null): OnboardingState {
  if (json == null) return newState();
  try {
    const value = JSON.parse(json) as OnboardingState | null;
    if (value?.version === 1 && Number.isInteger(value.step) && value.step >= 0 && value.step < steps.length
      && ['new', 'active', 'paused', 'completed', 'skipped'].includes(value.disposition)
      && (value.accountKind === 'bank' || value.accountKind === 'cash')
      && value.drafts && Object.entries(value.drafts).every(([kind, draft]) => isKind(kind) && isDraft(draft))
      && Array.isArray(value.receipts) && value.receipts.every(record => record && isKind(record.kind)
        && typeof record.id === 'string' && typeof record.name === 'string')) {
      return value;
    }
  } catch {
    // Surface the same message for malformed or unsupported configuration snapshots.
  }
  throw new Error('La configuration locale n’a pas pu être relue. Les informations du coffre restent accessibles depuis les rubriques.');
}

export default function Onboarding(props: OnboardingProps) {
  return <Wizard key={props.status.generation_id} {...props} />;
}

function Wizard({ status, registry, openRequested, onSaved, onComplete, onDiscover }: OnboardingProps) {
  const [state, setState] = useState(newState);
  const current = useRef(state);
  const [initialized, setInitialized] = useState(false);
  const [visible, setVisible] = useState(false);
  const [pending, setPending] = useState(false);
  const [navigating, setNavigating] = useState(false);
  const [issue, setIssue] = useState('');
  const [notice, setNotice] = useState('');
  const mounted = useRef(true);
  const navigation = useRef(false);
  const queue = useRef<Promise<void>>(Promise.resolve());
  const writeVersion = useRef(0);
  const request = useRef(openRequested);
  const dialog = useRef<HTMLDialogElement>(null);
  const body = useRef<HTMLDivElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const previouslyFocused = useRef<HTMLElement | null>(null);
  const labelId = useId();
  const date = today();
  const currency = status.settings.display_currency;
  const step = state.step;
  const finalStep = steps.length - 1;
  const entry = entries[step - 1];
  const kind = step === 3 ? state.accountKind : entry?.kinds[0];
  const receipts = entry ? state.receipts.filter(item => (entry.kinds as readonly GuidedKind[]).includes(item.kind)) : [];
  const busy = pending || navigating;

  function apply(next: OnboardingState) {
    current.current = next;
    if (mounted.current) setState(next);
  }

  function persist(next: OnboardingState, coalesce = false) {
    const version = ++writeVersion.current;
    const saved = queue.current.catch(() => undefined).then(async () => {
      if (coalesce && version !== writeVersion.current) return;
      try {
        await api.writeOnboardingState(status.generation_id, JSON.stringify(next));
        if (mounted.current) setIssue('');
      } catch (failure) {
        if (mounted.current) setIssue(`La configuration n’a pas pu être conservée. ${errorMessage(failure)}`);
        throw failure;
      }
    });
    queue.current = saved;
    return saved;
  }

  async function applyAfterSave(next: OnboardingState) {
    await persist(next);
    apply(next);
  }

  useEffect(() => {
    mounted.current = true;
    let active = true;
    async function initialize() {
      let saved = newState();
      try {
        saved = parseState(await api.readOnboardingState(status.generation_id));
      } catch (failure) {
        if (active) setIssue(errorMessage(failure));
      }
      if (!active) return;
      apply(saved);
      setVisible(!status.recovery_required && (saved.disposition === 'active'
        || saved.disposition === 'new' && status.mutation_sequence === 0 && status.document_count === 0));
      setInitialized(true);
    }
    void initialize();
    return () => { active = false; mounted.current = false; };
  }, [status.generation_id]);

  useEffect(() => {
    if (!initialized || openRequested === request.current) return;
    request.current = openRequested;
    setNotice('');
    // A completed or skipped configuration can be opened again from the welcome.
    const existing = current.current;
    if (existing.disposition === 'completed' || existing.disposition === 'skipped') {
      apply({ ...existing, step: 0 });
    }
    setVisible(true);
  }, [initialized, openRequested]);

  useLayoutEffect(() => {
    const element = dialog.current;
    if (!visible || !element) return;
    previouslyFocused.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    element.showModal();
    return () => {
      if (element.open) element.close();
      const previous = previouslyFocused.current;
      requestAnimationFrame(() => {
        if (document.querySelector('dialog[open]')) return;
        if (previous?.isConnected && previous !== document.body) previous.focus();
        else document.querySelector<HTMLElement>('main h1')?.focus();
      });
    };
  }, [visible]);

  useLayoutEffect(() => {
    if (!visible) return;
    if (body.current) body.current.scrollTop = 0;
    heading.current?.focus({ preventScroll: true });
  }, [visible, step]);

  async function changeStep(change: () => Promise<void>) {
    if (navigation.current || pending) return;
    navigation.current = true;
    setNavigating(true);
    try { await change(); }
    finally {
      navigation.current = false;
      if (mounted.current) setNavigating(false);
    }
  }

  function navigate(next: OnboardingState) {
    void changeStep(async () => {
      try { await applyAfterSave(next); }
      catch { return; }
      setNotice('');
    });
  }

  function goTo(stepNumber: number) {
    navigate({ ...current.current, step: stepNumber, disposition: 'active' });
  }

  function close(disposition: OnboardingState['disposition'], afterClose?: () => void) {
    void changeStep(async () => {
      try { await applyAfterSave({ ...current.current, disposition }); }
      catch { /* The caller can leave the wizard when storage is unavailable. */ }
      if (mounted.current) {
        setVisible(false);
        afterClose?.();
      }
    });
  }

  function withDraft(entryKind: GuidedKind, draft: GuidedDraft): OnboardingState {
    const prior = current.current;
    return { ...prior, disposition: 'active', drafts: { ...prior.drafts, [entryKind]: draft } };
  }

  function updateDraft(entryKind: GuidedKind, draft: GuidedDraft) {
    const next = withDraft(entryKind, draft);
    apply(next);
    void persist(next, true).catch(() => undefined);
    setNotice('');
  }

  async function saved(entryKind: GuidedKind, receipt: GuidedReceipt, nextDraft: GuidedDraft) {
    const prior = current.current;
    const alreadyRecorded = prior.receipts.some(value => value.kind === entryKind && value.id === receipt.id);
    await applyAfterSave({
      ...prior,
      drafts: { ...prior.drafts, [entryKind]: nextDraft },
      receipts: alreadyRecorded ? prior.receipts : [...prior.receipts, { ...receipt, kind: entryKind }],
    });
    if (!mounted.current) return;
    setNotice(`« ${receipt.name} » a été enregistré.`);
    try {
      await onSaved();
    } catch {
      if (mounted.current) setNotice(`« ${receipt.name} » a été enregistré. Fermez et rouvrez la rubrique pour actualiser la liste.`);
    }
  }

  if (!visible) return issue ? <p className="notice notice-error onboarding-background-notice" role="alert">{issue}</p> : null;
  return <dialog ref={dialog} className="onboarding-dialog" aria-labelledby={labelId} onCancel={event => {
    event.preventDefault();
    if (!pending) close('paused');
  }}>
    <div className="onboarding-progress">
      <span>Étape {step + 1} sur {steps.length}</span>
      <progress value={step + 1} max={steps.length} aria-label="Progression de la configuration" />
    </div>
    <div className="onboarding-body" ref={body}>
      <h2 id={labelId} ref={heading} tabIndex={-1}>{steps[step].title}</h2>
      <p>{steps[step].description}</p>
      {issue && <p className="notice notice-error" role="alert">{issue}</p>}
      {step === 0 && <>
        <ul className="onboarding-scope">
          <li>La famille et ses biens immobiliers</li>
          <li>Les comptes, les espèces, l’or et l’argent</li>
          <li>Les sociétés, les dettes et les loyers</li>
        </ul>
        <p>Vos données restent sur cet ordinateur. Vous pouvez passer chaque étape et reprendre la configuration à tout moment depuis Aide ou Paramètres.</p>
      </>}
      {kind && <>
        {step === 3 && <div className="module-tabs" aria-label="Type de compte" role="group">
          <button type="button" className={kind === 'bank' ? 'button' : 'button secondary'} aria-pressed={kind === 'bank'} disabled={busy}
            onClick={() => navigate({ ...current.current, accountKind: 'bank' })}>Compte bancaire</button>
          <button type="button" className={kind === 'cash' ? 'button' : 'button secondary'} aria-pressed={kind === 'cash'} disabled={busy}
            onClick={() => navigate({ ...current.current, accountKind: 'cash' })}>Caisse d’espèces</button>
        </div>}
        {receipts.length > 0 && <ul className="onboarding-receipts" aria-label="Informations enregistrées à cette étape">
          {receipts.map(item => <li key={`${item.kind}:${item.id}`}>{item.name}</li>)}
        </ul>}
        {notice && <p className="notice notice-success" role="status">{notice}</p>}
        {status.recovery_required ? <p className="notice notice-error">Rouvrez Patrimoine pour terminer la reprise du coffre avant d’enregistrer une saisie.</p> :
          <GuidedEntry key={kind} kind={kind} registry={registry} date={date} displayCurrency={currency}
            draft={state.drafts[kind] ?? newGuidedDraft(date, currency)}
            disabled={navigating} onDraftChange={value => updateDraft(kind, value)} onBeforeSave={value => applyAfterSave(withDraft(kind, value))}
            onSaved={(receipt, next) => saved(kind, receipt, next)} onBusyChange={setPending} />}
        <p className="field-hint">Les champs en cours de saisie restent dans le brouillon. Utilisez le bouton d’enregistrement pour les ajouter à votre patrimoine.</p>
      </>}
      {step === finalStep && (state.receipts.length ? <dl className="onboarding-recap">
        {entries.map(entry => {
          const items = state.receipts.filter(item => (entry.kinds as readonly GuidedKind[]).includes(item.kind));
          return items.length > 0 && <div key={entry.label}>
            <dt>{entry.label}</dt><dd><ul>{items.map(item => <li key={`${item.kind}:${item.id}`}>{item.name}</li>)}</ul></dd>
          </div>;
        })}
      </dl> : <p role="status">Aucune information n’a été enregistrée pendant la configuration. Vous pouvez accéder à votre patrimoine et le compléter plus tard.</p>)}
    </div>
    <div className="onboarding-footer">
      {step === 0 ? <>
        <button type="button" className="button secondary" disabled={busy} onClick={() => close('skipped')}>Passer et accéder à l’application</button>
        <button type="button" className="button" disabled={busy} onClick={() => goTo(1)}>Commencer la configuration</button>
      </> : <>
        <button type="button" className="button secondary" disabled={busy} onClick={() => goTo(step - 1)}>Précédent</button>
        <button type="button" className="text-button" disabled={busy} onClick={() => close('paused')}>Quitter et reprendre plus tard</button>
        {step < finalStep ? <button type="button" className="button secondary onboarding-next" disabled={busy} onClick={() => goTo(step + 1)}>
          {receipts.length ? 'Continuer' : 'Passer cette étape'}
        </button> : <>
          {onDiscover && <button type="button" className="button secondary onboarding-next" disabled={busy} onClick={() => close('completed', onDiscover)}>Découvrir Patrimoine</button>}
          <button type="button" className="button" disabled={busy} onClick={() => close('completed', onComplete)}>Accéder à mon patrimoine</button>
        </>}
      </>}
    </div>
  </dialog>;
}
