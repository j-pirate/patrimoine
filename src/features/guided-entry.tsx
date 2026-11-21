import { useEffect, useId, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { api, displayCurrencies, errorCode, errorMessage } from '../services/api';
import type { GuidedRecord } from '../services/guided-types';
import type { Fraction, RegistrySnapshot } from '../services/registry-types';
import { entityAt, Field } from './registry-ui';
import './guided-entry.css';

export type GuidedKind = 'family' | 'property' | 'bank' | 'cash' | 'metals' | 'company';
export type GuidedReceipt = GuidedRecord;

interface GuidedOwnerDraft {
  id: string;
  entityId: string;
  legalPercent: string;
  economicPercent: string;
}

export interface GuidedDraft {
  requestId: string | null;
  values: {
    date: string;
    firstName: string;
    lastName: string;
    name: string;
    country: string;
    city: string;
    propertyType: string;
    propertyStatus: string;
    amount: string;
    currency: string;
    ownerId: string;
    location: string;
    metal: string;
    form: string;
    quantity: string;
    unitWeight: string;
    weightUnit: string;
    purity: string;
    activity: string;
  };
  owners: GuidedOwnerDraft[];
}

interface GuidedEntryProps {
  kind: GuidedKind;
  registry: RegistrySnapshot;
  date: string;
  displayCurrency: string;
  draft?: GuidedDraft;
  onDraftChange?: (draft: GuidedDraft) => void;
  onBeforeSave?: (draft: GuidedDraft) => Promise<void>;
  disabled?: boolean;
  onSaved: (receipt: GuidedReceipt, nextDraft: GuidedDraft) => void | Promise<void>;
  onBusyChange?: (busy: boolean) => void;
}

export function newGuidedDraft(date: string, displayCurrency: string): GuidedDraft {
  return {
    requestId: null,
    values: {
      date, currency: displayCurrency, weightUnit: 'g',
      firstName: '', lastName: '', name: '', country: '', city: '', propertyType: '', propertyStatus: '',
      amount: '', ownerId: '', location: '', metal: '', form: '', quantity: '', unitWeight: '', purity: '', activity: '',
    },
    owners: [],
  };
}

const countries = { FR: 'France', TN: 'Tunisie', CH: 'Suisse', AE: 'Émirats arabes unis' };
const propertyTypes = {
  apartment: 'Appartement', house: 'Maison', building: 'Immeuble', land: 'Terrain',
  commercial: 'Local commercial', office: 'Bureau', garage: 'Garage', other: 'Autre',
};
const propertyStatuses = {
  owner_occupied: 'Occupé par le propriétaire', rented: 'Loué', vacant: 'Vacant',
  renovation: 'En travaux', for_sale: 'En vente',
};
const forms = { bar: 'Barre', ingot: 'Lingot ou lingotin', coin: 'Pièce', other: 'Autre forme' };
const weightUnits = { g: 'Gramme (g)', kg: 'Kilogramme (kg)', troy_oz: 'Once troy (31,1034768 g)' };
const currencies = Object.fromEntries(displayCurrencies.map(currency => [currency, currency]));
const submitLabels: Record<GuidedKind, string> = {
  family: 'Enregistrer le membre',
  property: 'Enregistrer le bien',
  bank: 'Enregistrer le compte',
  cash: 'Enregistrer la caisse',
  metals: 'Enregistrer le lot',
  company: 'Enregistrer la société',
};

function percentage(value: string): Fraction | null {
  const normalized = value.trim().replace(',', '.');
  if (!normalized) return null;
  // Transcribe the decimal percentage as an exact fraction. Rust validates and reduces it.
  if (!/^[0-9]{1,3}(?:\.[0-9]{1,12})?$/.test(normalized)) {
    throw new Error('Saisissez la part en pourcentage, par exemple 25 ou 12,5.');
  }
  const [whole, decimals = ''] = normalized.split('.');
  const num = BigInt(whole + decimals);
  const den = 100n * 10n ** BigInt(decimals.length);
  if (num > den) throw new Error('Une part ne peut pas dépasser 100 %. Vérifiez les pourcentages.');
  return { num: num.toString(), den: den.toString() };
}

function saveGuided(kind: GuidedKind, draft: GuidedDraft & { requestId: string }): Promise<GuidedReceipt> {
  const v = draft.values;
  const common = { request_id: draft.requestId, date: v.date };
  const name = v.name.trim();
  switch (kind) {
    case 'family':
      return api.createGuidedFamily({
        ...common, first_name: v.firstName.trim(), last_name: v.lastName.trim(), residence_country: v.country || null,
      });
    case 'property':
      return api.createGuidedProperty({
        ...common, name, property_type: v.propertyType, country: v.country, city: v.city.trim(), status: v.propertyStatus,
        estimated_value: v.amount.trim() || null, currency: v.currency, sole_owner_entity_id: v.ownerId || null,
      });
    case 'bank':
    case 'cash':
      return api.createGuidedAccount({
        ...common, kind, name, country: v.country, currency: v.currency, opening_balance: v.amount.trim(),
        owner_entity_id: v.ownerId, storage_location: kind === 'cash' ? v.location.trim() : '',
      });
    case 'metals':
      return api.createGuidedMetal({
        ...common, designation: name, metal: v.metal, form: v.form, quantity: v.quantity.trim(),
        unit_weight: v.unitWeight.trim(), weight_unit: v.weightUnit, purity: v.purity.trim(),
        owner_entity_id: v.ownerId, country: v.country, storage_location: v.location.trim(), cost_currency: v.currency,
      });
    case 'company':
      return api.createGuidedCompany({
        ...common, name, country: v.country, activity: v.activity.trim(),
        owners: draft.owners.map(row => ({
          owner_entity_id: row.entityId, legal_share: percentage(row.legalPercent), economic_share: percentage(row.economicPercent),
        })),
      });
  }
}

type ValueKey = keyof GuidedDraft['values'];
interface TextOptions {
  required?: boolean;
  placeholder?: string;
  hint?: string;
  decimal?: boolean;
  maxLength?: number;
}

export default function GuidedEntry({ kind, registry, date, displayCurrency, draft, onDraftChange, onBeforeSave, disabled, onSaved, onBusyChange }: GuidedEntryProps) {
  const [localDraft, setLocalDraft] = useState(() => newGuidedDraft(date, displayCurrency));
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const inFlight = useRef(false);
  const mounted = useRef(true);
  const newOwnerId = useRef<string | null>(null);
  const prefix = useId();
  const data = draft ?? localDraft;
  const v = data.values;
  const possibleOwners = registry.entities
    .filter(entity => !entity.archived)
    .map(entity => entityAt(registry, entity, v.date))
    .filter(entity => !entity.archived)
    .sort((left, right) => left.display_name.localeCompare(right.display_name, 'fr'));
  const ownerOptions = Object.fromEntries(possibleOwners.map(entity => [entity.id, entity.display_name]));
  const needsOwner = kind === 'bank' || kind === 'cash' || kind === 'metals';

  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);

  useEffect(() => {
    onBusyChange?.(pending);
    return () => onBusyChange?.(false);
  }, [onBusyChange, pending]);

  useEffect(() => {
    if (newOwnerId.current) {
      document.getElementById(`${prefix}-${newOwnerId.current}-owner`)?.focus();
      newOwnerId.current = null;
    }
  }, [data.owners, prefix]);

  function store(next: GuidedDraft) {
    onDraftChange?.(next);
    if (!draft) setLocalDraft(next);
  }
  function change(key: ValueKey, value: string) {
    try {
      store({ ...data, values: { ...v, [key]: value } });
      setError('');
    } catch (failure) {
      setError(errorMessage(failure));
    }
  }
  function changeOwners(owners: GuidedOwnerDraft[]) {
    try {
      store({ ...data, owners });
      setError('');
    } catch (failure) {
      setError(errorMessage(failure));
    }
  }
  function addOwner() {
    const id = globalThis.crypto.randomUUID();
    newOwnerId.current = id;
    changeOwners([...data.owners, { id, entityId: '', legalPercent: '', economicPercent: '' }]);
  }
  function updateOwner(id: string, field: keyof Omit<GuidedOwnerDraft, 'id'>, value: string) {
    changeOwners(data.owners.map(owner => owner.id === id ? { ...owner, [field]: value } : owner));
  }
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (inFlight.current || disabled) return;
    setError('');
    inFlight.current = true;
    setPending(true);
    try {
      if (!data.requestId && v.ownerId && !Object.hasOwn(ownerOptions, v.ownerId)) {
        throw new Error('Le propriétaire choisi n’est plus disponible à cette date.');
      }
      if (!data.requestId && kind === 'company') {
        if (new Set(data.owners.map(row => row.entityId)).size !== data.owners.length) {
          throw new Error('Ajoutez chaque associé une seule fois.');
        }
        for (const owner of data.owners) {
          if (!Object.hasOwn(ownerOptions, owner.entityId)) throw new Error('Sélectionnez un associé disponible à cette date.');
          percentage(owner.legalPercent);
          percentage(owner.economicPercent);
        }
      }
      const request = { ...data, requestId: data.requestId ?? globalThis.crypto.randomUUID() };
      if (!data.requestId) {
        if (onBeforeSave) await onBeforeSave(request);
        else store(request);
      }
      if (!mounted.current) return;
      let receipt: GuidedReceipt;
      try {
        receipt = await saveGuided(kind, request);
      } catch (failure) {
        if (mounted.current && errorCode(failure) === 'validation') store({ ...request, requestId: null });
        throw failure;
      }
      if (!mounted.current) return;
      const nextDraft = newGuidedDraft(date, displayCurrency);
      try {
        await onSaved(receipt, nextDraft);
        if (mounted.current && !draft) setLocalDraft(nextDraft);
      } catch (failure) {
        if (mounted.current) setError(`« ${receipt.name} » a été enregistré dans le coffre. La confirmation locale reste à terminer. ${errorMessage(failure)}`);
      }
    } catch (failure) {
      if (mounted.current) setError(errorMessage(failure));
    } finally {
      inFlight.current = false;
      if (mounted.current) setPending(false);
    }
  }

  function text(key: ValueKey, label: string, { required = false, placeholder, hint, decimal = false, maxLength = 200 }: TextOptions = {}) {
    const id = `${prefix}-${key}`;
    return <Field id={id} label={label} hint={hint}><input id={id} required={required} maxLength={decimal ? 64 : maxLength}
      inputMode={decimal ? 'decimal' : undefined} placeholder={placeholder} aria-describedby={hint ? `${id}-help` : undefined}
      value={v[key]} onChange={event => change(key, event.currentTarget.value)} /></Field>;
  }

  function select(key: ValueKey, label: string, options: Record<string, string>, {
    required = false, empty = 'Choisir…', hint,
  }: { required?: boolean; empty?: string; hint?: string } = {}) {
    const id = `${prefix}-${key}`;
    const unavailableSelection = v[key] && !Object.hasOwn(options, v[key]);
    return <Field id={id} label={label} hint={hint}><select id={id} required={required} value={v[key]}
      aria-describedby={hint ? `${id}-help` : undefined} onChange={event => change(key, event.currentTarget.value)}>
      <option value="">{empty}</option>
      {unavailableSelection && <option value={v[key]} disabled>Choix devenu indisponible</option>}
      {Object.entries(options).map(([value, title]) => <option key={value} value={value}>{title}</option>)}
    </select></Field>;
  }

  function owner(required: boolean) {
    return select('ownerId', required ? 'Propriétaire unique (100 %)' : 'Propriétaire unique (facultatif)', ownerOptions, {
      required, empty: required ? 'Choisir le titulaire…' : 'À renseigner plus tard',
      hint: required
        ? 'Choisissez la personne ou la société qui détient seule les droits de propriété et les droits financiers. Pour une détention partagée, utilisez les réglages avancés.'
        : 'Indiquez le propriétaire seulement s’il détient seul les droits de propriété et les droits financiers. Une détention partagée peut être renseignée plus tard.',
    });
  }

  function companyOwners() {
    return <details className="guided-options" open={data.owners.length > 0 || undefined}>
      <summary>Associés (facultatif)</summary>
      <p className="field-hint">Ajoutez les personnes ou sociétés que vous connaissez. Laissez les parts inconnues vides pour les compléter plus tard.</p>
      {possibleOwners.length === 0 && <p>Aucune personne ou société n’est disponible. Vous pouvez enregistrer la société et ajouter ses associés plus tard.</p>}
      <div>
        {data.owners.map((row, index) => {
          const id = `${prefix}-${row.id}`;
          const unavailable = row.entityId && !Object.hasOwn(ownerOptions, row.entityId);
          return <fieldset key={row.id} className="guided-owner">
            <legend>Associé {index + 1}</legend>
            <Field id={`${id}-owner`} label="Personne ou société"><select id={`${id}-owner`} required value={row.entityId}
              onChange={event => updateOwner(row.id, 'entityId', event.currentTarget.value)}>
              <option value="">Choisir l’associé…</option>
              {unavailable && <option value={row.entityId} disabled>Choix devenu indisponible</option>}
              {possibleOwners.map(entity => <option disabled={data.owners.some(other => other.id !== row.id && other.entityId === entity.id)}
                value={entity.id} key={entity.id}>{entity.display_name}</option>)}
            </select></Field>
            <div className="guided-fields">
              <Field id={`${id}-legal`} label="Part du capital (%)" hint="Facultatif. Par exemple 25 pour un quart du capital.">
                <input id={`${id}-legal`} aria-describedby={`${id}-legal-help`} inputMode="decimal" maxLength={16}
                  value={row.legalPercent} onChange={event => updateOwner(row.id, 'legalPercent', event.currentTarget.value)} />
              </Field>
              <Field id={`${id}-economic`} label="Part des droits financiers (%)" hint="Facultatif. Part des bénéfices revenant à l’associé.">
                <input id={`${id}-economic`} aria-describedby={`${id}-economic-help`} inputMode="decimal" maxLength={16}
                  value={row.economicPercent} onChange={event => updateOwner(row.id, 'economicPercent', event.currentTarget.value)} />
              </Field>
            </div>
            <button type="button" className="text-button" aria-label={`Retirer l’associé ${index + 1}`}
              onClick={() => changeOwners(data.owners.filter(other => other.id !== row.id))}>Retirer cet associé</button>
          </fieldset>;
        })}
      </div>
      {possibleOwners.length > data.owners.length && <button type="button" className="button secondary small-button" onClick={addOwner}>Ajouter un associé</button>}
    </details>;
  }

  return <form className="guided-form" onSubmit={event => { void submit(event); }} aria-busy={pending} onInvalidCapture={event => {
    const disclosure = event.target instanceof HTMLElement && event.target.closest('details');
    if (disclosure) disclosure.open = true;
  }}>
    {error && <div className="notice notice-error" role="alert">{error}</div>}
    {data.requestId && !pending && <p role="status" className="notice notice-info">Une tentative d’enregistrement est en attente. Confirmez son résultat pour reprendre cette saisie.</p>}
    {needsOwner && possibleOwners.length === 0 && <div className="notice notice-info">
      Ajoutez d’abord un membre de la famille ou une société titulaire. Vous pouvez compléter cette saisie plus tard.
    </div>}
    <fieldset disabled={disabled || pending || data.requestId !== null}>
      <div className="guided-fields">
        {kind === 'family' ? <>
          {text('firstName', 'Prénom', { required: true, maxLength: 100 })}
          {text('lastName', 'Nom', { required: true, maxLength: 100 })}
          {select('country', 'Pays de résidence (facultatif)', countries, { empty: 'À renseigner plus tard' })}
        </> : null}
        {kind === 'property' ? <>
          {text('name', 'Nom du bien', { required: true, placeholder: 'Ex. Appartement des Tilleuls', maxLength: 160 })}
          {select('propertyType', 'Type de bien', propertyTypes, { required: true })}
          {select('country', 'Pays du bien', countries, { required: true })}
          {text('city', 'Ville (facultatif)', { maxLength: 160 })}
          {select('propertyStatus', 'État du bien', propertyStatuses, { required: true, hint: 'Par exemple, choisissez « Loué » si le bien a un locataire.' })}
          {owner(false)}
          {text('amount', 'Valeur estimée (facultatif)', { decimal: true, placeholder: 'Ex. 180000', hint: 'Indiquez une estimation du prix du bien entier. Les dettes et le partage de la valeur se suivent séparément.' })}
          {select('currency', 'Devise de l’estimation', currencies, { required: true })}
        </> : null}
        {kind === 'bank' || kind === 'cash' ? <>
          {text('name', kind === 'bank' ? 'Banque ou nom du compte' : 'Nom de la caisse', { required: true, placeholder: kind === 'bank' ? 'Ex. Banque du Centre' : 'Ex. Espèces de voyage', maxLength: 160 })}
          {select('country', kind === 'bank' ? 'Pays du compte' : 'Pays de conservation', countries, { required: true })}
          {kind === 'cash' && text('location', 'Lieu de conservation', { required: true, placeholder: 'Ex. Coffre à domicile', maxLength: 1000 })}
          {text('amount', 'Solde de départ', { required: true, decimal: true, placeholder: 'Ex. 1250,50', hint: 'Montant au premier jour du suivi. Saisissez 0 si le solde est nul.' })}
          {select('currency', 'Devise du solde', currencies, { required: true })}
          {owner(true)}
        </> : null}
        {kind === 'metals' ? <>
          {select('metal', 'Métal', { gold: 'Or', silver: 'Argent' }, { required: true })}
          {select('form', 'Forme', forms, { required: true })}
          {text('name', 'Nom du lot', { required: true, placeholder: 'Ex. Pièces d’or 20 francs', maxLength: 160 })}
          {text('quantity', 'Quantité', { required: true, decimal: true, placeholder: 'Ex. 3', hint: 'Nombre d’objets identiques dans ce lot. Séparez les formes ou les poids différents.' })}
          {text('unitWeight', 'Poids d’un objet', { required: true, decimal: true, placeholder: 'Ex. 6,45' })}
          {select('weightUnit', 'Unité de poids', weightUnits, { required: true })}
          {text('purity', 'Pureté', { required: true, decimal: true, placeholder: 'Ex. 0,9', hint: 'Part de métal fin dans le poids total. Saisissez 0,9 pour 900 millièmes ou 0,9999 pour 999,9 millièmes.' })}
          {owner(true)}
          {select('country', 'Pays de conservation', countries, { required: true })}
          {text('location', 'Lieu de conservation', { required: true, placeholder: 'Ex. Coffre à domicile', maxLength: 1000 })}
        </> : null}
        {kind === 'company' ? <>
          {text('name', 'Nom de la société', { required: true, placeholder: 'Ex. Ateliers du Canal', maxLength: 160 })}
          {select('country', 'Pays de la société', countries, { required: true })}
          {text('activity', 'Activité (facultatif)', { placeholder: 'Ex. Location de bureaux', maxLength: 1000 })}
        </> : null}
      </div>
      {kind === 'company' && companyOwners()}
      <details className="guided-options">
        <summary>Options avancées</summary>
        <div className="guided-fields">
          <Field id={`${prefix}-date`} label="Date de début de suivi"
            hint="Modifiez cette date si vous souhaitez que l’ajout prenne effet un autre jour.">
            <input id={`${prefix}-date`} type="date" aria-describedby={`${prefix}-date-help`} value={v.date}
              onChange={event => { if (event.currentTarget.value) change('date', event.currentTarget.value); }} />
          </Field>
          {kind === 'metals' && select('currency', 'Devise de suivi des coûts', currencies, {
            required: true, hint: 'Utilisée si vous ajoutez un prix d’achat plus tard. La saisie du lot ne crée aucun prix.',
          })}
        </div>
      </details>
    </fieldset>
    <div className="guided-actions"><button type="submit" className="button" disabled={disabled || pending || (!data.requestId && needsOwner && possibleOwners.length === 0)}>
      {pending ? 'Enregistrement…' : data.requestId ? 'Confirmer l’enregistrement' : submitLabels[kind]}
    </button></div>
  </form>;
}
