import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import RegistryPage from './registry';
import { api } from '../services/api';
import { emptyRegistry } from '../services/registry-types';
import type { Asset, ConsolidationReport, Entity, RegistrySnapshot } from '../services/registry-types';
import type { ProfilePeriod } from '../services/phase4-types';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});

const person: Entity = {
  id: 'alice', revision_id: 'alice-r3', kind: 'person', display_name: 'Alice actuelle', first_name: 'Alice', last_name: 'Actuelle',
  birth_date: null, residence_country: 'FR', preferred_currency: 'EUR', activity: '', notes: 'Profil courant', archived: false,
};
const oldPerson: Entity = { ...person, revision_id: 'alice-r1', display_name: 'Alice ancienne', last_name: 'Ancienne', residence_country: 'TN', notes: 'Ancien profil' };
const middlePerson: Entity = { ...person, revision_id: 'alice-r2', display_name: 'Alice intermédiaire', residence_country: 'CH' };
const asset: Asset = { id: 'home', revision_id: 'home-r3', name: 'Maison actuelle', country: 'FR', category: 'immobilier', notes: '', archived: false };
const oldAsset: Asset = { ...asset, revision_id: 'home-r1', name: 'Maison ancienne', country: 'TN' };
const middleAsset: Asset = { ...asset, revision_id: 'home-r2', name: 'Maison intermédiaire', country: 'CH' };
const profilePeriod: ProfilePeriod = {
  id: 'person-profile-period', revision_id: 'period-r1', target_kind: 'entity', target_id: person.id,
  profile_revision_id: oldPerson.revision_id, valid_from: '2020-01-01', valid_to: '2025-01-01',
};
const registry: RegistrySnapshot = {
  ...emptyRegistry, entities: [person], assets: [asset],
  profile_revisions: [oldPerson, middlePerson, person], asset_profile_revisions: [oldAsset, middleAsset, asset],
  profile_periods: [profilePeriod], knowledge_sequence: 12,
  memberships: [{ id: 'membership', revision_id: 'member-r1', person_id: person.id, relationship: 'Parent', valid_from: '2020-01-01', valid_to: null }],
  ownerships: [{ id: 'ownership', revision_id: 'ownership-r1', asset_id: asset.id, owner_entity_id: person.id,
    legal_share: { num: '1', den: '1' }, economic_share: { num: '1', den: '1' }, valid_from: '2020-01-01', valid_to: null }],
};

function openRegistry(page: 'family' | 'assets' = 'family', knowledgeSequence: number | null = null) {
  const onChanged = vi.fn().mockResolvedValue(undefined);
  const props = { registry, onChanged, onBusyChange: vi.fn(), context: { personId: null, date: '2024-06-30', knowledgeSequence } };
  const view = render(<RegistryPage {...props} page={page} />);
  return { user: userEvent.setup(), onChanged, view, props };
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.saveEntity).mockResolvedValue(person);
  vi.mocked(api.saveProfilePeriod).mockResolvedValue(profilePeriod);
  vi.mocked(api.saveValuation).mockResolvedValue({ id: 'valuation', revision_id: 'valuation-r1', target_kind: 'asset', target_id: asset.id,
    amount: '123.456', currency: 'TND', effective_on: '2024-06-30', source: 'Expertise originale' });
});

describe('Périodes et révisions exactes des profils descriptifs', () => {
  it('affiche la révision désignée à D parmi toutes les versions du même objet et respecte la fin exclue', () => {
    const { view, props } = openRegistry();
    const table = screen.getByRole('table', { name: 'Personnes, membres et tiers' });
    expect(within(table).getByText('Alice ancienne')).toBeVisible();
    expect(within(table).getByText('TN')).toBeVisible();
    expect(within(table).queryByText('Alice actuelle')).not.toBeInTheDocument();
    expect(within(table).queryByText('Alice intermédiaire')).not.toBeInTheDocument();
    view.rerender(<RegistryPage {...props} page="family" context={{ ...props.context, date: '2025-01-01' }} />);
    const current = screen.getByRole('table', { name: 'Personnes, membres et tiers' });
    expect(within(current).getByText('Alice actuelle')).toBeVisible();
    expect(within(current).getByText('FR')).toBeVisible();
    expect(within(current).queryByText('Alice ancienne')).not.toBeInTheDocument();
  });

  it('ouvre le profil de base courant pour le modifier même si la liste montre un ancien profil à D', async () => {
    const { user, onChanged } = openRegistry();
    const row = within(screen.getByRole('table', { name: 'Personnes, membres et tiers' })).getByText('Alice ancienne').closest('tr')!;
    await user.click(within(row).getByRole('button', { name: 'Modifier' }));
    expect(screen.getByText('Modifier le profil de base des connaissances actuelles.')).toBeVisible();
    expect(screen.getByLabelText('Nom affiché de la personne')).toHaveValue('Alice actuelle');
    expect(screen.getByLabelText('Pays de résidence')).toHaveValue('FR');
    expect(screen.getByLabelText('Notes')).toHaveValue('Profil courant');
    await user.clear(screen.getByLabelText('Nom affiché de la personne'));
    await user.type(screen.getByLabelText('Nom affiché de la personne'), 'Alice corrigée');
    await user.type(screen.getByLabelText('Motif de la correction'), 'Correction du profil courant');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la correction' }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.saveEntity).toHaveBeenCalledWith(expect.objectContaining({
      id: person.id, expected_revision_id: person.revision_id, display_name: 'Alice corrigée', residence_country: 'FR', notes: 'Profil courant',
    }));
    expect(api.saveProfilePeriod).not.toHaveBeenCalled();
  });

  it.each([
    { targetKind: 'entity', targetId: person.id, revisionId: oldPerson.revision_id, versions: ['alice-r1', 'alice-r2', 'alice-r3'] },
    { targetKind: 'asset', targetId: asset.id, revisionId: oldAsset.revision_id, versions: ['home-r1', 'home-r2', 'home-r3'] },
  ] as const)('crée une période $targetKind liée à la révision exacte choisie sans perdre les anciennes versions', async ({ targetKind, targetId, revisionId, versions }) => {
    const { user, onChanged } = openRegistry();
    await user.click(screen.getByText('Profils datés et historique des fiches', { selector: 'summary' }));
    await user.click(screen.getByRole('button', { name: 'Définir une période de profil' }));
    await user.selectOptions(screen.getByLabelText('Type d’objet du profil'), targetKind);
    await user.selectOptions(screen.getByLabelText('Objet du profil'), targetId);
    const revisions = screen.getByLabelText('Révision exacte du profil');
    for (const version of versions) expect(within(revisions).getByRole('option', { name: new RegExp(`révision ${version}$`) })).toHaveValue(version);
    await user.selectOptions(revisions, revisionId);
    fireEvent.change(screen.getByLabelText('Date de début'), { target: { value: '2010-01-01' } });
    fireEvent.change(screen.getByLabelText('Date de fin (exclue)'), { target: { value: '2020-01-01' } });
    await user.type(screen.getByLabelText('Motif de l’enregistrement'), 'Document ancien vérifié');
    await user.click(screen.getByRole('button', { name: 'Créer l’enregistrement' }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.saveProfilePeriod).toHaveBeenCalledExactlyOnceWith({
      id: null, expected_revision_id: null, reason: 'Document ancien vérifié', target_kind: targetKind,
      target_id: targetId, profile_revision_id: revisionId, valid_from: '2010-01-01', valid_to: '2020-01-01',
    });
    expect(api.saveEntity).not.toHaveBeenCalled();
    expect(api.saveAsset).not.toHaveBeenCalled();
  });

  it('corrige une période avec la révision attendue de la période et la révision exacte du profil', async () => {
    const { user } = openRegistry();
    await user.click(screen.getByText('Profils datés et historique des fiches', { selector: 'summary' }));
    const table = screen.getByRole('table', { name: 'Périodes des profils descriptifs' });
    await user.click(within(table).getByRole('button', { name: 'Modifier' }));
    expect(screen.getByLabelText('Révision exacte du profil')).toHaveValue(oldPerson.revision_id);
    await user.selectOptions(screen.getByLabelText('Révision exacte du profil'), middlePerson.revision_id);
    await user.type(screen.getByLabelText('Motif de la correction'), 'Référence documentaire rectifiée');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la correction' }));
    await waitFor(() => expect(api.saveProfilePeriod).toHaveBeenCalledExactlyOnceWith({
      id: profilePeriod.id, expected_revision_id: profilePeriod.revision_id, reason: 'Référence documentaire rectifiée',
      target_kind: 'entity', target_id: person.id, profile_revision_id: middlePerson.revision_id,
      valid_from: profilePeriod.valid_from, valid_to: profilePeriod.valid_to,
    }));
  });

  it('bloque créations, corrections, archivages et annulations à K=0 et transmet K au calcul', async () => {
    const report: ConsolidationReport = {
      context: 'family', person_id: null, date: '2024-06-30', knowledge_sequence: 0, engine_version: 'test', status: 'non_calculable',
      gross_eur: '0.00', debt_eur: '0.00', net_eur: '0.00', exact_gross: { num: '0', den: '1' },
      exact_debt: { num: '0', den: '1' }, exact_net: { num: '0', den: '1' }, global_equity_eur: '0.00',
      gross_is_underlying_complete: false, contributions: [], warnings: [],
    };
    vi.mocked(api.getConsolidation).mockResolvedValue(report);
    const { user, view, props } = openRegistry('family', 0);
    await user.click(screen.getByText('Périodes d’appartenance à la famille', { selector: 'summary' }));
    await user.click(screen.getByText('Profils datés et historique des fiches', { selector: 'summary' }));
    expect(screen.getByRole('button', { name: 'Créer une personne' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Ajouter une période familiale' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Archiver' })).toBeDisabled();
    for (const button of screen.getAllByRole('button', { name: 'Modifier' })) expect(button).toBeDisabled();
    for (const button of screen.getAllByRole('button', { name: 'Annuler l’enregistrement' })) expect(button).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Définir une période de profil' })).toBeDisabled();
    view.rerender(<RegistryPage {...props} page="assets" />);
    expect(screen.getByRole('button', { name: 'Créer un actif' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Ajouter une estimation' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Archiver' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Annuler l’enregistrement' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Contrôler les attributions' }));
    expect(await screen.findByText('Non calculable')).toBeVisible();
    expect(api.getConsolidation).toHaveBeenCalledExactlyOnceWith({ context: 'family', person_id: null, date: '2024-06-30', knowledge_sequence: 0 });
    expect(api.saveEntity).not.toHaveBeenCalled();
    expect(api.saveAsset).not.toHaveBeenCalled();
    expect(api.saveValuation).not.toHaveBeenCalled();
    expect(api.saveProfilePeriod).not.toHaveBeenCalled();
    expect(api.voidRegistryRecord).not.toHaveBeenCalled();
  });

  it('enregistre une estimation TND dans sa devise et garde la précision de la chaîne saisie', async () => {
    const { user, onChanged } = openRegistry('assets');
    await user.click(screen.getByRole('button', { name: 'Ajouter une estimation' }));
    await user.selectOptions(screen.getByLabelText('Actif à valoriser'), asset.id);
    await user.selectOptions(screen.getByLabelText('Devise de l’estimation'), 'TND');
    await user.type(screen.getByLabelText('Montant de l’estimation (TND)'), '12345678901234567890,123456789');
    await user.type(screen.getByLabelText('Source ou méthode de l’estimation'), 'Expertise originale');
    await user.type(screen.getByLabelText('Motif de l’enregistrement'), 'Première estimation en dinars');
    await user.click(screen.getByRole('button', { name: 'Créer l’enregistrement' }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.saveValuation).toHaveBeenCalledExactlyOnceWith({
      id: null, expected_revision_id: null, reason: 'Première estimation en dinars', target_kind: 'asset', target_id: asset.id,
      amount: '12345678901234567890,123456789', currency: 'TND', effective_on: '2024-06-30', source: 'Expertise originale',
    });
    expect(api.convertCurrency).not.toHaveBeenCalled();
  });
});
