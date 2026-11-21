import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../services/api';
import { emptyRegistry } from '../services/registry-types';
import type { Entity, RegistrySnapshot } from '../services/registry-types';
import GuidedEntry, { newGuidedDraft } from './guided-entry';
import type { GuidedDraft, GuidedKind } from './guided-entry';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});

const date = '2026-11-21';
const owner: Entity = {
  id: 'owner', revision_id: 'owner-r1', kind: 'person', display_name: 'Camille Rivière',
  first_name: 'Camille', last_name: 'Rivière', birth_date: null, residence_country: null,
  preferred_currency: 'EUR', activity: '', notes: '', archived: false,
};
const registry: RegistrySnapshot = {
  ...emptyRegistry,
  entities: [owner, { ...owner, id: 'archived', revision_id: 'archived-r1', display_name: 'Ancien titulaire', archived: true }],
};

function open(kind: GuidedKind, source: RegistrySnapshot = emptyRegistry) {
  const user = userEvent.setup();
  const onSaved = vi.fn();
  return { user, onSaved, ...render(<GuidedEntry kind={kind} registry={source} date={date} displayCurrency="CHF" onSaved={onSaved} />) };
}

beforeEach(() => vi.resetAllMocks());

describe('Saisie guidée', () => {
  it('enregistre directement un bien, sans ajouter de propriétaire ni de valeur supposée', async () => {
    const { user, onSaved } = open('property');
    vi.mocked(api.createGuidedProperty).mockResolvedValue({ id: 'property', name: 'Les Tilleuls' });

    await user.type(screen.getByRole('textbox', { name: 'Nom du bien' }), 'Les Tilleuls');
    await user.selectOptions(screen.getByRole('combobox', { name: 'Type de bien' }), 'house');
    await user.selectOptions(screen.getByRole('combobox', { name: 'Pays du bien' }), 'FR');
    await user.selectOptions(screen.getByRole('combobox', { name: 'État du bien' }), 'owner_occupied');
    await user.click(screen.getByRole('button', { name: 'Enregistrer le bien' }));

    expect(api.createGuidedProperty).toHaveBeenCalledExactlyOnceWith({
      request_id: expect.stringMatching(/^[0-9a-f-]{36}$/),
      date, name: 'Les Tilleuls', property_type: 'house', country: 'FR', city: '', status: 'owner_occupied',
      estimated_value: null, currency: 'CHF', sole_owner_entity_id: null,
    });
    expect(api.saveAsset).not.toHaveBeenCalled();
    expect(onSaved).toHaveBeenCalledWith({ id: 'property', name: 'Les Tilleuls' }, newGuidedDraft(date, 'CHF'));
  });

  it('conserve les champs après un rejet métier et autorise leur correction', async () => {
    const { user, onSaved } = open('bank', registry);
    vi.mocked(api.createGuidedAccount).mockRejectedValueOnce({ code: 'validation', message: 'Solde invalide.' });
    vi.mocked(api.createGuidedAccount).mockResolvedValueOnce({ id: 'account', name: 'Compte courant' });

    await user.type(screen.getByRole('textbox', { name: 'Banque ou nom du compte' }), 'Compte courant');
    await user.selectOptions(screen.getByRole('combobox', { name: 'Pays du compte' }), 'CH');
    const ownerSelect = screen.getByRole('combobox', { name: 'Propriétaire unique (100 %)' });
    expect(screen.queryByRole('option', { name: 'Ancien titulaire' })).not.toBeInTheDocument();
    await user.selectOptions(ownerSelect, owner.id);
    const amount = screen.getByRole('textbox', { name: 'Solde de départ' });
    await user.type(amount, 'montant');
    await user.click(screen.getByRole('button', { name: 'Enregistrer le compte' }));

    expect(await screen.findByRole('alert')).toHaveTextContent('Solde invalide.');
    expect(amount).toHaveValue('montant');
    expect(amount).toBeEnabled();
    expect(onSaved).not.toHaveBeenCalled();

    await user.clear(amount);
    await user.type(amount, '0');
    await user.click(screen.getByRole('button', { name: 'Enregistrer le compte' }));

    const requests = vi.mocked(api.createGuidedAccount).mock.calls.map(([input]) => input);
    expect(requests).toHaveLength(2);
    expect(requests[1]).toEqual({
      ...requests[0], request_id: expect.not.stringMatching(requests[0].request_id), opening_balance: '0',
    });
    expect(onSaved).toHaveBeenCalledOnce();
  });

  it('réutilise la demande enregistrée si l’application ferme avant la confirmation', async () => {
    const user = userEvent.setup();
    let savedDraft: GuidedDraft = {
      ...newGuidedDraft(date, 'EUR'),
      values: { ...newGuidedDraft(date, 'EUR').values, firstName: 'Camille', lastName: 'Rivière' },
    };
    let settle = (_receipt: { id: string; name: string }) => {};
    vi.mocked(api.createGuidedFamily)
      .mockImplementationOnce(() => new Promise(resolve => { settle = resolve; }))
      .mockResolvedValueOnce({ id: 'member', name: 'Camille Rivière' });
    const resume = () => render(<GuidedEntry kind="family" registry={registry} date={date} displayCurrency="EUR"
      draft={savedDraft} onDraftChange={next => { savedDraft = next; }} onSaved={onSaved} />);
    const onSaved = vi.fn();

    const first = resume();
    await user.click(screen.getByRole('button', { name: 'Enregistrer le membre' }));
    expect(savedDraft.requestId).toEqual(expect.any(String));
    first.unmount();
    resume();
    expect(screen.getByRole('textbox', { name: 'Prénom' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Confirmer l’enregistrement' }));

    expect(api.createGuidedFamily).toHaveBeenCalledTimes(2);
    expect(vi.mocked(api.createGuidedFamily).mock.calls[0]).toEqual(vi.mocked(api.createGuidedFamily).mock.calls[1]);
    expect(onSaved).toHaveBeenCalledOnce();
    await act(async () => { settle({ id: 'member', name: 'Camille Rivière' }); });
    expect(onSaved).toHaveBeenCalledOnce();
  });

  it('demande un titulaire réel avant de pouvoir enregistrer un lot', () => {
    open('metals');
    expect(screen.getByRole('button', { name: 'Enregistrer le lot' })).toBeDisabled();
    expect(screen.getByText(/Ajoutez d’abord un membre de la famille/)).toBeVisible();
    expect(api.createGuidedMetal).not.toHaveBeenCalled();
  });

  it.each([
    { percent: '12,5', expected: { num: '125', den: '1000' } },
    { percent: '', expected: null },
  ])('enregistre un associé avec sa part déclarée ou inconnue ($percent)', async ({ percent, expected }) => {
    const { user, onSaved } = open('company', registry);
    vi.mocked(api.createGuidedCompany).mockResolvedValue({ id: 'company', name: 'Ateliers du Canal' });

    await user.type(screen.getByRole('textbox', { name: 'Nom de la société' }), 'Ateliers du Canal');
    await user.selectOptions(screen.getByRole('combobox', { name: 'Pays de la société' }), 'FR');
    await user.click(screen.getByText('Associés (facultatif)'));
    await user.click(screen.getByRole('button', { name: 'Ajouter un associé' }));
    const ownerSelect = screen.getByRole('combobox', { name: 'Personne ou société' });
    expect(ownerSelect).toHaveFocus();
    await user.selectOptions(ownerSelect, owner.id);
    if (percent) await user.type(screen.getByRole('textbox', { name: 'Part du capital (%)' }), percent);
    await user.click(screen.getByRole('button', { name: 'Enregistrer la société' }));

    await waitFor(() => expect(onSaved).toHaveBeenCalledOnce());
    expect(api.createGuidedCompany).toHaveBeenCalledWith({
      request_id: expect.any(String), date, name: 'Ateliers du Canal', country: 'FR', activity: '',
      owners: [{ owner_entity_id: owner.id, legal_share: expected, economic_share: null }],
    });
  });
});
