import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import RegistryPage from './registry';
import { api } from '../services/api';
import { emptyRegistry } from '../services/registry-types';
import type { ConsolidationReport, Entity, RegistrySnapshot } from '../services/registry-types';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});

const person: Entity = {
  id: 'alice', revision_id: 'alice-r1', kind: 'person', display_name: 'Alice Martin', first_name: 'Alice', last_name: 'Martin',
  birth_date: null, residence_country: 'FR', preferred_currency: 'EUR', activity: '', notes: '', archived: false,
};
const registry: RegistrySnapshot = {
  ...emptyRegistry, entities: [person],
  memberships: [{ id: 'membership', revision_id: 'member-r1', person_id: person.id, relationship: 'Parent', valid_from: '2020-01-01', valid_to: null }],
  assets: [{ id: 'property', revision_id: 'property-r1', name: 'Bien commun', country: 'FR', category: 'immobilier', notes: '', archived: false }],
  knowledge_sequence: 8,
};
const report: ConsolidationReport = {
  context: 'person', person_id: person.id, date: '2026-11-04', knowledge_sequence: 8, engine_version: 'test', status: 'partial',
  gross_eur: '200000.00', debt_eur: '0.00', net_eur: '200000.00', exact_gross: { num: '200000', den: '1' },
  exact_debt: { num: '0', den: '1' }, exact_net: { num: '200000', den: '1' }, global_equity_eur: '0.00',
  gross_is_underlying_complete: false, contributions: [],
  warnings: [{ code: 'missing_debt_scope', object_id: '', message: 'Les passifs ne sont pas encore inventoriés.' }],
};

function openRegistry(page: 'family' | 'companies' | 'assets', data = registry, personId: string | null = null) {
  const onChanged = vi.fn().mockResolvedValue(undefined);
  render(<RegistryPage page={page} registry={data} context={{ personId, date: '2026-11-04' }} onChanged={onChanged} onBusyChange={vi.fn()} />);
  return { user: userEvent.setup(), onChanged };
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.saveEntity).mockResolvedValue(person);
  vi.mocked(api.saveOwnership).mockResolvedValue({ id: 'ownership', revision_id: 'r', asset_id: 'property', owner_entity_id: person.id, legal_share: null, economic_share: null, valid_from: '2026-11-04', valid_to: null });
  vi.mocked(api.saveValuation).mockResolvedValue({ id: 'valuation', revision_id: 'r', target_kind: 'asset', target_id: 'property', amount: '1.25', currency: 'EUR', effective_on: '2026-11-04', source: '' });
});

describe('Registre familial et droits datés', () => {
  it('ne présente pas zéro comme une valeur calculée lorsque toutes les contributions sont inconnues', async () => {
    vi.mocked(api.getConsolidation).mockResolvedValue({ ...report, status: 'non_calculable', gross_eur: '0.00', contributions: [], warnings: [{ code: 'missing_valuation', object_id: 'property', message: 'La valorisation est absente.' }] });
    const { user } = openRegistry('assets');
    await user.click(screen.getByRole('button', { name: 'Contrôler les attributions' }));
    expect(await screen.findByText('Non calculable')).toBeVisible();
    expect(screen.getByText('La valorisation est absente.')).toBeVisible();
    expect(screen.queryByText('0.00 EUR')).not.toBeInTheDocument();
  });

  it('crée une personne sans inventer une appartenance familiale et conserve plusieurs champs saisis ensemble', async () => {
    const { user, onChanged } = openRegistry('family', emptyRegistry);
    await user.click(screen.getByRole('button', { name: 'Créer une personne' }));
    act(() => {
      fireEvent.change(screen.getByRole('textbox', { name: 'Nom affiché de la personne' }), { target: { value: 'Nadia Martin' } });
      fireEvent.change(screen.getByRole('textbox', { name: 'Prénom' }), { target: { value: 'Nadia' } });
      fireEvent.change(screen.getByRole('textbox', { name: 'Nom de famille' }), { target: { value: 'Martin' } });
    });
    await user.type(screen.getByRole('textbox', { name: 'Motif de l’enregistrement' }), 'Créer un tiers copropriétaire');
    await user.click(screen.getByRole('button', { name: 'Créer l’enregistrement' }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.saveEntity).toHaveBeenCalledWith(expect.objectContaining({ id: null, expected_revision_id: null, display_name: 'Nadia Martin', first_name: 'Nadia', last_name: 'Martin', kind: 'person', archived: false }));
    expect(api.saveMembership).not.toHaveBeenCalled();
  });

  it('transmet explicitement la période familiale avec fin exclusive', async () => {
    vi.mocked(api.saveMembership).mockResolvedValue(registry.memberships[0]);
    const { user } = openRegistry('family');
    await user.click(screen.getByRole('button', { name: 'Ajouter une période familiale' }));
    await user.selectOptions(screen.getByLabelText('Personne à inclure dans la famille'), person.id);
    fireEvent.change(screen.getByLabelText('Date de début'), { target: { value: '2027-01-01' } });
    fireEvent.change(screen.getByLabelText('Date de fin (exclue)'), { target: { value: '2028-01-01' } });
    await user.type(screen.getByRole('textbox', { name: 'Motif de l’enregistrement' }), 'Nouvelle période future');
    await user.click(screen.getByRole('button', { name: 'Créer l’enregistrement' }));
    await waitFor(() => expect(api.saveMembership).toHaveBeenCalledWith(expect.objectContaining({ person_id: person.id, valid_from: '2027-01-01', valid_to: '2028-01-01' })));
  });

  it('distingue un droit juridique nul connu et un droit économique inconnu', async () => {
    const { user } = openRegistry('assets');
    await user.click(screen.getByRole('button', { name: 'Attribuer une propriété' }));
    await user.selectOptions(screen.getByLabelText('Actif concerné'), 'property');
    await user.selectOptions(screen.getByLabelText('Propriétaire identifié'), person.id);
    await user.click(screen.getByRole('checkbox', { name: 'Droit juridique connu' }));
    await user.type(screen.getByLabelText('Droit juridique · numérateur'), '0');
    await user.type(screen.getByLabelText('Droit juridique · dénominateur'), '1');
    expect(screen.getByRole('checkbox', { name: 'Droit économique connu' })).not.toBeChecked();
    await user.type(screen.getByRole('textbox', { name: 'Motif de l’enregistrement' }), 'Droits économiques à qualifier');
    await user.click(screen.getByRole('button', { name: 'Créer l’enregistrement' }));
    await waitFor(() => expect(api.saveOwnership).toHaveBeenCalledWith(expect.objectContaining({ legal_share: { num: '0', den: '1' }, economic_share: null })));
  });

  it('conserve la saisie lorsqu’une correction rencontre un conflit de révision', async () => {
    vi.mocked(api.saveEntity).mockRejectedValue({ code: 'conflict', message: 'Cette fiche a été modifiée. Rechargez sa version courante.' });
    const { user, onChanged } = openRegistry('family');
    const row = within(screen.getByRole('table', { name: 'Personnes, membres et tiers' })).getByText('Alice Martin').closest('tr')!;
    await user.click(within(row).getByRole('button', { name: 'Modifier' }));
    await user.clear(screen.getByRole('textbox', { name: 'Nom affiché de la personne' }));
    await user.type(screen.getByRole('textbox', { name: 'Nom affiché de la personne' }), 'Alice corrigée');
    await user.type(screen.getByRole('textbox', { name: 'Motif de la correction' }), 'Corriger le nom');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la correction' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Cette fiche a été modifiée');
    expect(screen.getByRole('textbox', { name: 'Nom affiché de la personne' })).toHaveValue('Alice corrigée');
    expect(api.saveEntity).toHaveBeenCalledWith(expect.objectContaining({ id: 'alice', expected_revision_id: 'alice-r1' }));
    expect(onChanged).not.toHaveBeenCalled();
  });

  it('demande confirmation et motif avant archivage sans annuler les liens', async () => {
    const { user } = openRegistry('family');
    await user.click(within(within(screen.getByRole('table', { name: 'Personnes, membres et tiers' })).getByText('Alice Martin').closest('tr')!).getByRole('button', { name: 'Archiver' }));
    expect(api.saveEntity).not.toHaveBeenCalled();
    expect(screen.getByText(/Archiver une fiche ne cède pas ses actifs/)).toBeVisible();
    await user.type(screen.getByLabelText('Motif de la modification'), 'Masquer la fiche dans les nouvelles saisies');
    await user.click(screen.getByRole('button', { name: 'Confirmer l’archivage' }));
    await waitFor(() => expect(api.saveEntity).toHaveBeenCalledWith(expect.objectContaining({ id: person.id, expected_revision_id: person.revision_id, archived: true })));
    expect(api.voidRegistryRecord).not.toHaveBeenCalled();
  });

  it.each(['archivage', 'réactivation', 'annulation'])('REG-05 focalise la confirmation de %s puis restaure son déclencheur', async action => {
    const { user } = openRegistry('family', action === 'réactivation' ? { ...registry, entities: [{ ...person, archived: true }] } : registry);
    if (action === 'réactivation') await user.click(screen.getByRole('checkbox', { name: 'Afficher les fiches archivées' }));
    const table = screen.getByRole('table', { name: action === 'annulation' ? 'Périodes du périmètre familial' : 'Personnes, membres et tiers' });
    const trigger = within(table).getByRole('button', { name: action === 'annulation' ? 'Annuler l’enregistrement' : action === 'réactivation' ? 'Réactiver' : 'Archiver' });
    await user.click(trigger);
    expect(screen.getByRole('region', { name: 'Confirmer la modification du registre' })).toHaveFocus();
    await user.click(screen.getByRole('button', { name: 'Conserver l’état actuel' }));
    await waitFor(() => expect(trigger).toHaveFocus());
    expect(api.saveEntity).not.toHaveBeenCalled();
    expect(api.voidRegistryRecord).not.toHaveBeenCalled();
  });

  it('REG-03 ferme la correction confirmée et garde son succès si la relecture échoue', async () => {
    const { user, onChanged } = openRegistry('family');
    onChanged.mockRejectedValue(new Error('Registre momentanément illisible'));
    const row = within(screen.getByRole('table', { name: 'Personnes, membres et tiers' })).getByText('Alice Martin').closest('tr')!;
    await user.click(within(row).getByRole('button', { name: 'Modifier' }));
    await user.type(screen.getByLabelText('Motif de la correction'), 'Correction confirmée');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la correction' }));
    expect(await screen.findByText(/L’affichage n’a pas pu être actualisé/)).toHaveTextContent('Registre momentanément illisible');
    expect(api.saveEntity).toHaveBeenCalledOnce();
    expect(screen.queryByRole('button', { name: 'Enregistrer la correction' })).not.toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('envoie le montant français comme texte au backend sans calcul JavaScript', async () => {
    const { user } = openRegistry('assets');
    await user.click(screen.getByRole('button', { name: 'Ajouter une estimation' }));
    await user.selectOptions(screen.getByLabelText('Actif à valoriser'), 'property');
    await user.type(screen.getByLabelText('Montant de l’estimation (EUR)'), '1,25');
    await user.type(screen.getByLabelText('Source ou méthode de l’estimation'), 'Estimation personnelle');
    await user.type(screen.getByRole('textbox', { name: 'Motif de l’enregistrement' }), 'Estimation manuelle');
    await user.click(screen.getByRole('button', { name: 'Créer l’enregistrement' }));
    await waitFor(() => expect(api.saveValuation).toHaveBeenCalledWith(expect.objectContaining({ amount: '1,25', currency: 'EUR' })));
  });

  it('utilise le contexte individuel et la date sans présenter un net exhaustif', async () => {
    vi.mocked(api.getConsolidation).mockResolvedValue(report);
    const { user } = openRegistry('assets', registry, person.id);
    await user.click(screen.getByRole('button', { name: 'Contrôler les attributions' }));
    await screen.findByText('200000.00 EUR');
    expect(api.getConsolidation).toHaveBeenCalledWith({ context: 'person', person_id: person.id, date: '2026-11-04', knowledge_sequence: null });
    expect(screen.getByText(/ne présente pas un patrimoine net exhaustif/)).toBeVisible();
    expect(screen.getByText('Les passifs ne sont pas encore inventoriés.')).toBeVisible();
  });
});
