import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../services/api';
import { emptyRegistry } from '../services/registry-types';
import type { RegistrySnapshot } from '../services/registry-types';
import type { Property, PropertyEvent, PropertyReport, PropertySnapshot, RentalUnit } from '../services/property-types';
import { PropertyWorkspace } from './properties';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});

const property: Property = { id: 'details', revision_id: 'details-r1', asset_id: 'house', property_type: 'house', city: 'Tunis', address: 'Adresse de test', surface_m2: '90.125', status: 'vacant', valid_from: '2024-01-01', valid_to: null, notes: '' };
const unit: RentalUnit = { id: 'unit', revision_id: 'unit-r1', property_asset_id: 'house', reference: 'A', name: 'Logement', surface_m2: null, valid_from: '2024-01-01', valid_to: null, available_for_rent: true, notes: '' };
const registry: RegistrySnapshot = { ...emptyRegistry, knowledge_sequence: 24, assets: [{ id: 'house', revision_id: 'house-r1', name: 'Maison de test', country: 'TN', category: 'real_estate', notes: '', archived: false }] };
const snapshot: PropertySnapshot = { properties: [property], units: [unit], events: [], knowledge_sequence: 24 };
const disposal: PropertyEvent = { id: 'sale', revision_id: 'sale-r1', asset_id: 'house', kind: 'disposal', effective_on: '2026-10-01', price: '150000', fees: '1000', currency: 'EUR', notes: 'Acte initial' };
const report: PropertyReport = {
  asset_id: 'house', label: 'Maison de test', country: null, date: '2026-11-05', knowledge_sequence: 24,
  status: 'partial', property, units: [unit], acquisition: null, disposal: null,
  acquisition_cost: null, exact_acquisition_cost: null, acquisition_currency: null,
  acquisition_cost_eur: null, exact_acquisition_cost_eur: null, valuation: null, valuation_conversion: null,
  acquisition_conversions: [], estimated_value_eur: null, exact_estimated_value_eur: null,
  unrealized_gain_original: null, exact_unrealized_gain_original: null, unrealized_gain_currency: null,
  unrealized_gain_eur: null, exact_unrealized_gain_eur: null, debt_remaining_eur: null, net_before_disposal_eur: null,
  financing_conversions: [], exact_debt_remaining_eur: null, exact_net_before_disposal_eur: null,
  source_ids: ['details-r1'], warnings: [{ code: 'missing_valuation', message: 'Aucune estimation applicable ; le coût n’est pas une estimation.' }],
};

function open(knowledgeSequence: number | null = null, propertySnapshot = snapshot) {
  const onChanged = vi.fn().mockResolvedValue(undefined);
  const onNavigateRegistry = vi.fn();
  render(<PropertyWorkspace registry={registry} snapshot={propertySnapshot}
    context={{ personId: null, date: '2026-11-05', knowledgeSequence }}
    onChanged={onChanged} onBusyChange={vi.fn()} onNavigateRegistry={onNavigateRegistry} />);
  return { user: userEvent.setup(), onChanged, onNavigateRegistry };
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.saveProperty).mockResolvedValue(property);
  vi.mocked(api.saveRentalUnit).mockResolvedValue(unit);
  vi.mocked(api.savePropertyEvent).mockResolvedValue({ id: 'acquisition', revision_id: 'acquisition-r1', asset_id: 'house', kind: 'acquisition', effective_on: '2024-01-01', price: '100000', fees: '0', currency: 'TND', notes: '' });
  vi.mocked(api.disposeProperty).mockResolvedValue(snapshot);
  vi.mocked(api.getPropertyReport).mockResolvedValue(report);
});

describe('Fiches immobilières et unités techniques', () => {
  it('attache la fiche à l’actif existant et transmet la surface exacte sans créer un second actif', async () => {
    const { user, onChanged } = open(null, { ...snapshot, properties: [], units: [] });
    await user.click(screen.getByRole('button', { name: 'Ajouter une fiche immobilière' }));
    await user.selectOptions(screen.getByLabelText('Actif immobilier existant'), 'house');
    await user.selectOptions(screen.getByLabelText('Type de bien'), 'house');
    await user.type(screen.getByLabelText('Surface en m²'), '90,12500001');
    await user.type(screen.getByLabelText('Motif de l’enregistrement ou de la correction'), 'Première fiche documentée');
    expect(within(screen.getByLabelText('Situation du bien')).queryByRole('option', { name: 'Vendu' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Enregistrer la fiche immobilière' }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.saveProperty).toHaveBeenCalledWith(expect.objectContaining({ id: null, expected_revision_id: null, asset_id: 'house', surface_m2: '90,12500001', valid_from: '2026-11-05', valid_to: null }));
    expect(api.saveAsset).not.toHaveBeenCalled();
  });

  it('conserve une correction et sa révision attendue lorsqu’un conflit est refusé', async () => {
    vi.mocked(api.saveProperty).mockRejectedValue({ message: 'Une révision plus récente existe.' });
    const { user, onChanged } = open();
    await user.click(screen.getByRole('button', { name: 'Modifier la fiche de Maison de test' }));
    await user.clear(screen.getByLabelText('Ville'));
    await user.type(screen.getByLabelText('Ville'), 'Sfax');
    await user.type(screen.getByLabelText('Motif de l’enregistrement ou de la correction'), 'Adresse corrigée');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la correction immobilière' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Une révision plus récente existe.');
    expect(screen.getByLabelText('Ville')).toHaveValue('Sfax');
    expect(api.saveProperty).toHaveBeenCalledWith(expect.objectContaining({ id: 'details', expected_revision_id: 'details-r1', city: 'Sfax' }));
    expect(onChanged).not.toHaveBeenCalled();
    expect(screen.queryByText(/L’enregistrement immobilier a été conservé/)).not.toBeInTheDocument();
  });

  it('enregistre une unité indisponible avec une fin exclusive sans ajouter de valeur patrimoniale', async () => {
    const { user } = open();
    await user.click(screen.getByRole('button', { name: 'Ajouter une unité' }));
    await user.selectOptions(screen.getByLabelText('Bien de l’unité'), 'house');
    await user.type(screen.getByLabelText('Référence de l’unité'), 'LOT-B');
    await user.type(screen.getByLabelText('Nom de l’unité'), 'Unité en travaux');
    await user.click(screen.getByLabelText('Unité disponible à la location dans cette période'));
    fireEvent.change(screen.getByLabelText('Fin de validité (exclue)'), { target: { value: '2027-01-01' } });
    await user.type(screen.getByLabelText('Motif de l’enregistrement ou de la correction'), 'Période d’indisponibilité');
    await user.click(screen.getByRole('button', { name: 'Enregistrer l’unité' }));
    expect(api.saveRentalUnit).toHaveBeenCalledWith(expect.objectContaining({ property_asset_id: 'house', reference: 'LOT-B', surface_m2: null, available_for_rent: false, valid_to: '2027-01-01' }));
    expect(api.saveAsset).not.toHaveBeenCalled();
    expect(api.saveValuation).not.toHaveBeenCalled();
  });

  it('distingue prix inconnu et frais explicitement nuls, puis le début de suivi sans coût supposé', async () => {
    const { user } = open();
    await user.click(screen.getByRole('button', { name: 'Enregistrer une acquisition' }));
    await user.selectOptions(screen.getByLabelText('Bien concerné'), 'house');
    await user.type(screen.getByLabelText('Frais d’acquisition'), '0');
    await user.selectOptions(screen.getByLabelText('Devise des prix et frais'), 'TND');
    await user.type(screen.getByLabelText('Motif de l’enregistrement ou de la correction'), 'Frais connus, prix absent');
    await user.click(screen.getByRole('button', { name: 'Enregistrer l’événement immobilier' }));
    expect(api.savePropertyEvent).toHaveBeenCalledWith(expect.objectContaining({ price: null, fees: '0', currency: 'TND' }));
    await waitFor(() => expect(screen.queryByLabelText('Prix d’achat')).not.toBeInTheDocument());
    await user.click(screen.getByRole('button', { name: 'Enregistrer une acquisition' }));
    await user.selectOptions(screen.getByLabelText('Bien concerné'), 'house');
    await user.type(screen.getByLabelText('Prix d’achat'), '100000,00000001');
    await user.selectOptions(screen.getByLabelText('Nature de l’événement'), 'opening_tracking');
    expect(screen.queryByLabelText('Prix d’achat')).not.toBeInTheDocument();
    await user.type(screen.getByLabelText('Motif de l’enregistrement ou de la correction'), 'Entrée dans le suivi');
    await user.click(screen.getByRole('button', { name: 'Enregistrer l’événement immobilier' }));
    expect(api.savePropertyEvent).toHaveBeenLastCalledWith(expect.objectContaining({ kind: 'opening_tracking', price: null, fees: null }));
  });

  it('exige une confirmation de cession et transmet la séquence de toute l’opération atomique', async () => {
    const { user, onChanged } = open();
    await user.click(screen.getByRole('button', { name: 'Enregistrer une cession' }));
    await user.selectOptions(screen.getByLabelText('Bien concerné'), 'house');
    fireEvent.change(screen.getByLabelText('Date de cession'), { target: { value: '2026-12-01' } });
    await user.type(screen.getByLabelText('Prix de cession'), '150000,00000001');
    await user.type(screen.getByLabelText('Motif de l’enregistrement ou de la correction'), 'Vente documentée');
    expect(screen.getByRole('button', { name: 'Confirmer la cession complète' })).toBeDisabled();
    expect(api.disposeProperty).not.toHaveBeenCalled();
    await user.click(screen.getByLabelText('Je confirme la cession complète et la clôture des droits suivis.'));
    await user.click(screen.getByRole('button', { name: 'Confirmer la cession complète' }));
    expect(api.disposeProperty).toHaveBeenCalledExactlyOnceWith({ asset_id: 'house', effective_on: '2026-12-01', price: '150000,00000001', fees: null, currency: 'EUR', reason: 'Vente documentée', expected_sequence: 24 });
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.savePropertyEvent).not.toHaveBeenCalled();
  });

  it('protège K=0 tout en demandant le rapport à cette même borne', async () => {
    const { user } = open(0);
    for (const name of ['Ajouter une fiche immobilière', 'Ajouter une unité', 'Enregistrer une acquisition', 'Enregistrer une cession']) expect(screen.getByRole('button', { name })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Modifier la fiche de Maison de test' })).toBeDisabled();
    await user.selectOptions(screen.getByLabelText('Bien à contrôler'), 'house');
    await user.click(screen.getByRole('button', { name: 'Contrôler la valeur du bien' }));
    await screen.findByText('Aucune estimation applicable ; le coût n’est pas une estimation.');
    expect(api.getPropertyReport).toHaveBeenCalledExactlyOnceWith({ asset_id: 'house', date: '2026-11-05', knowledge_sequence: 0 });
    expect(api.saveProperty).not.toHaveBeenCalled();
  });

  it('corrige date, prix, devise et notes de cession uniquement après confirmation du recalcul des droits', async () => {
    const { user, onChanged } = open(null, { ...snapshot, events: [disposal] });
    await user.click(screen.getByRole('button', { name: 'Corriger la cession' }));
    expect(screen.getByLabelText('Bien concerné')).toBeDisabled();
    expect(screen.getByLabelText('Bien concerné')).toHaveValue('house');
    expect(screen.getByLabelText('Nature de l’événement')).toBeDisabled();
    expect(screen.getByLabelText('Nature de l’événement')).toHaveValue('disposal');
    fireEvent.change(screen.getByLabelText('Date de cession'), { target: { value: '2026-10-15' } });
    await user.clear(screen.getByLabelText('Prix de cession'));
    await user.type(screen.getByLabelText('Prix de cession'), '151000,00000001');
    await user.selectOptions(screen.getByLabelText('Devise des prix et frais'), 'CHF');
    await user.clear(screen.getByLabelText('Notes'));
    await user.type(screen.getByLabelText('Notes'), 'Acte rectificatif du notaire');
    await user.type(screen.getByLabelText('Motif de l’enregistrement ou de la correction'), 'Date et montant rectifiés');
    const confirm = screen.getByRole('button', { name: 'Confirmer la correction de cession' });
    expect(confirm).toBeDisabled();
    fireEvent.submit(confirm.closest('form')!);
    expect(api.savePropertyEvent).not.toHaveBeenCalled();
    expect(api.disposeProperty).not.toHaveBeenCalled();
    await user.click(screen.getByLabelText('Je confirme la correction de la cession et le recalcul des droits suivis.'));
    await user.click(confirm);
    expect(api.savePropertyEvent).toHaveBeenCalledExactlyOnceWith({ id: 'sale', expected_revision_id: 'sale-r1', asset_id: 'house', kind: 'disposal', effective_on: '2026-10-15', price: '151000,00000001', fees: '1000', currency: 'CHF', notes: 'Acte rectificatif du notaire', reason: 'Date et montant rectifiés' });
    expect(api.disposeProperty).not.toHaveBeenCalled();
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(await screen.findByText(/La correction de cession et le recalcul des droits ont été enregistrés ensemble/)).toBeVisible();
  });

  it('conserve la correction motivée de cession après un conflit et redemande confirmation si la date change', async () => {
    vi.mocked(api.savePropertyEvent).mockRejectedValue({ code: 'conflict', message: 'La cession a déjà été corrigée dans une autre révision.' });
    const { user, onChanged } = open(null, { ...snapshot, events: [disposal] });
    await user.click(screen.getByRole('button', { name: 'Corriger la cession' }));
    await user.clear(screen.getByLabelText('Notes'));
    await user.type(screen.getByLabelText('Notes'), 'Pièce nouvelle conservée');
    await user.type(screen.getByLabelText('Motif de l’enregistrement ou de la correction'), 'Correction motivée');
    await user.click(screen.getByLabelText('Je confirme la correction de la cession et le recalcul des droits suivis.'));
    fireEvent.change(screen.getByLabelText('Date de cession'), { target: { value: '2026-10-20' } });
    expect(screen.getByRole('button', { name: 'Confirmer la correction de cession' })).toBeDisabled();
    await user.click(screen.getByLabelText('Je confirme la correction de la cession et le recalcul des droits suivis.'));
    await user.click(screen.getByRole('button', { name: 'Confirmer la correction de cession' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('La cession a déjà été corrigée dans une autre révision.');
    expect(screen.getByLabelText('Date de cession')).toHaveValue('2026-10-20');
    expect(screen.getByLabelText('Notes')).toHaveValue('Pièce nouvelle conservée');
    expect(screen.getByLabelText('Motif de l’enregistrement ou de la correction')).toHaveValue('Correction motivée');
    expect(api.savePropertyEvent).toHaveBeenCalledWith(expect.objectContaining({ id: 'sale', expected_revision_id: 'sale-r1', kind: 'disposal', effective_on: '2026-10-20' }));
    expect(onChanged).not.toHaveBeenCalled();
    expect(screen.queryByText(/La correction de cession et le recalcul des droits ont été enregistrés ensemble/)).not.toBeInTheDocument();
  });

  it('rend les montants inconnus et financements absents explicitement non calculables', async () => {
    const { user } = open();
    await user.selectOptions(screen.getByLabelText('Bien à contrôler'), 'house');
    await user.click(screen.getByRole('button', { name: 'Contrôler la valeur du bien' }));
    await screen.findByRole('heading', { name: 'Maison de test · 2026-11-05' });
    expect(screen.getByText('Non calculable · couverture des financements insuffisante')).toBeVisible();
    expect(screen.getAllByText('Non calculable').length).toBeGreaterThan(3);
    expect(screen.queryByText('0.00 EUR')).not.toBeInTheDocument();
    expect(screen.getByText(/Le coût ne remplace jamais une estimation manquante/)).toBeVisible();
  });
  it('REG-03 conserve la confirmation de cession si le parent refuse son actualisation', async () => {
    const { user, onChanged } = open();
    onChanged.mockRejectedValueOnce(new Error('Relecture immobilière indisponible'));
    await user.click(screen.getByRole('button', { name: 'Enregistrer une cession' }));
    await user.selectOptions(screen.getByLabelText('Bien concerné'), 'house');
    await user.type(screen.getByLabelText('Motif de l’enregistrement ou de la correction'), 'Acte enregistré');
    await user.click(screen.getByLabelText('Je confirme la cession complète et la clôture des droits suivis.'));
    await user.click(screen.getByRole('button', { name: 'Confirmer la cession complète' }));
    expect(await screen.findByText(/La cession et la clôture des droits ont été enregistrées ensemble.*L’affichage n’a pas pu être actualisé/)).toHaveTextContent('Relecture immobilière indisponible');
    expect(api.disposeProperty).toHaveBeenCalledOnce();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Confirmer la cession complète' })).not.toBeInTheDocument();
  });

});
