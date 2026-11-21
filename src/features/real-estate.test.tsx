import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../services/api';
import { emptyRegistry } from '../services/registry-types';
import type { Entity, RegistrySnapshot } from '../services/registry-types';
import type { PropertySnapshot } from '../services/property-types';
import type { Lease, LeaseTerms, RentDue, RentalReport, RentalSnapshot, RentPayment } from '../services/rental-types';
import RealEstatePage from './real-estate';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});

const entity = (id: string, name: string): Entity => ({ id, revision_id: `${id}-r1`, kind: 'person', display_name: name, first_name: null, last_name: null, birth_date: null, residence_country: 'FR', preferred_currency: 'EUR', activity: '', notes: '', archived: false });
const registry: RegistrySnapshot = { ...emptyRegistry, knowledge_sequence: 27, entities: [entity('tenant', 'Locataire identifié'), entity('owner', 'Propriétaire'), entity('recipient', 'Bénéficiaire distinct'), { ...entity('company', 'Société immobilière'), kind: 'company' }], assets: [{ id: 'house', revision_id: 'house-r1', name: 'Maison de test', country: 'FR', category: 'real_estate', notes: '', archived: false }] };
const properties: PropertySnapshot = {
  properties: [{ id: 'property', revision_id: 'property-r1', asset_id: 'house', property_type: 'house', city: '', address: '', surface_m2: null, status: 'rented', valid_from: '2024-01-01', valid_to: null, notes: '' }],
  units: [{ id: 'unit', revision_id: 'unit-r1', property_asset_id: 'house', reference: 'A', name: 'Logement A', surface_m2: '50', valid_from: '2024-01-01', valid_to: null, available_for_rent: true, notes: '' }], events: [], knowledge_sequence: 27,
};
const lease: Lease = { id: 'lease', revision_id: 'lease-r1', unit_id: 'unit', tenant_entity_id: 'tenant', valid_from: '2026-01-01', valid_to: null, notes: '' };
const terms: LeaseTerms = { id: 'terms', revision_id: 'terms-r1', lease_id: 'lease', rent: '1000', charges: '50', currency: 'EUR', frequency: 'monthly', due_day: 5, deposit_required: '2000', valid_from: '2026-01-01', valid_to: null, notes: '' };
const due: RentDue = { id: 'due', revision_id: 'due-r1', lease_id: 'lease', terms_revision_id: 'terms-r1', period_start: '2026-03-01', period_end: '2026-04-01', due_on: '2026-03-05', rent: '1000', charges: '50', currency: 'EUR', generation_key: 'stable-march', notes: '' };
const payment: RentPayment = { id: 'payment', revision_id: 'payment-r1', lease_id: 'lease', kind: 'receipt', amount: '600', currency: 'EUR', effective_on: '2026-03-05', original_payment_id: null, recipient_entity_id: 'owner', reference: 'VIREMENT-MARS', unlinked_reason: 'Relevé conservé ; banque non intégrée' };
const rentals: RentalSnapshot = { leases: [lease], lease_terms: [terms], lease_term_revisions: [terms], dues: [due], payments: [payment], allocations: [], expenses: [], knowledge_sequence: 27 };
const report: RentalReport = {
  from_date: '2026-03-01', to_date: '2026-04-01', date: '2026-03-31', knowledge_sequence: 27, status: 'partial',
  contractual_annual_rent_eur: '12000.00', rent_due_eur: '1000.00', charges_due_eur: '50.00',
  rent_received_eur: '600.00', charges_received_eur: '0.00', outstanding_rent_eur: '400.00',
  outstanding_charges_eur: '50.00', unallocated_payments_eur: '0.00', economic_rent_eur: '1000.00',
  operating_expenses_eur: '0.00', noi_eur: '1000.00', gross_yield_percent: null, net_yield_percent: null,
  cash_flow_after_financing_eur: null, exact_totals: { rent_due: { num: '1000', den: '1' } },
  occupation: { occupied_unit_days: 31, available_unit_days: 31, percent: '100.00', basis: 'Occupation contractuelle, pas un constat physique.' },
  dues: [{ due_id: 'due', lease_id: 'lease', asset_id: 'house', due_on: '2026-03-05', period_start: '2026-03-01', period_end: '2026-04-01', currency: 'EUR', rent: '1000.00', charges: '50.00', allocated_rent: '600.00', allocated_charges: '0.00', outstanding_rent: '400.00', outstanding_charges: '50.00', overdue_days: 26, settled_on: null, historical_late: true, source_ids: ['due-r1', 'payment-r1'] }],
  source_ids: ['terms-r1', 'due-r1', 'payment-r1'], conversions: [{
    amount_original: '1', from_currency: 'TND', to_currency: 'EUR', date: '2026-03-05', knowledge_sequence: 27,
    status: 'complete', method: 'direct', exact_rate: { num: '3', den: '10' }, exact_converted: { num: '3', den: '10' },
    amount_converted: '0.30', legs: [{ rate_id: 'manual-rate', revision_id: 'manual-rate-r2', base_currency: 'TND', quote_currency: 'EUR', rate: '0.3', from_currency: 'TND', to_currency: 'EUR', effective_on: '2026-03-01', source: 'Cours manuel du relevé', inverted: false, exact_rate: { num: '3', den: '10' } }],
    oldest_rate_date: '2026-03-01', age_days: 4, warnings: [],
  }],
  warnings: [{ code: 'financing_missing', object_id: 'scope', message: 'Les financements ne sont pas inventoriés.' }],
};

async function open(knowledgeSequence: number | null = null, registrySnapshot = registry) {
  const onChanged = vi.fn().mockResolvedValue(undefined);
  const props = { registry: registrySnapshot, context: { personId: null as string | null, date: '2026-03-31', knowledgeSequence }, onChanged, onBusyChange: vi.fn(), onNavigateRegistry: vi.fn() };
  const view = render(<RealEstatePage {...props} />);
  await screen.findByRole('button', { name: 'Ajouter une fiche immobilière' });
  return { user: userEvent.setup(), onChanged, props, ...view };
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.listPropertiesAt).mockResolvedValue(properties);
  vi.mocked(api.listRentalsAt).mockResolvedValue(rentals);
  vi.mocked(api.saveLease).mockResolvedValue(lease);
  vi.mocked(api.saveLeaseTerms).mockResolvedValue(terms);
  vi.mocked(api.saveRentPayment).mockResolvedValue(payment);
  vi.mocked(api.refundRentPayment).mockResolvedValue({ ...payment, id: 'refund', revision_id: 'refund-r1', amount: '-100', kind: 'refund', original_payment_id: 'payment' });
  vi.mocked(api.saveRentAllocation).mockResolvedValue({ id: 'allocation', revision_id: 'allocation-r1', payment_id: 'payment', due_id: 'due', component: 'rent', amount: '600', currency: 'EUR', effective_on: '2026-03-05' });
  vi.mocked(api.generateRentDues).mockResolvedValue({ created: [due], existing_count: 2, knowledge_sequence: 28 });
  vi.mocked(api.getRentalReport).mockResolvedValue(report);
});

describe('Locations, encaissements et indicateurs', () => {
  it('IMMO-02 garde la confirmation immobilière après les relectures du parent et du registre', async () => {
    const savedProperty = { ...properties.properties[0], revision_id: 'property-r2', city: 'Lyon' };
    vi.mocked(api.saveProperty).mockResolvedValue(savedProperty);
    const { user, onChanged, props, rerender } = await open();
    const refreshedProperties = { ...properties, properties: [savedProperty], knowledge_sequence: 28 };
    vi.mocked(api.listPropertiesAt).mockResolvedValue(refreshedProperties);
    vi.mocked(api.listRentalsAt).mockResolvedValue({ ...rentals, knowledge_sequence: 28 });
    onChanged.mockImplementationOnce(async () => {
      rerender(<RealEstatePage {...props} registry={{ ...registry, knowledge_sequence: 28 }} />);
    });
    await user.click(screen.getByRole('button', { name: 'Modifier la fiche de Maison de test' }));
    await user.type(screen.getByLabelText('Ville'), 'Lyon');
    await user.type(screen.getByLabelText('Motif de l’enregistrement ou de la correction'), 'Adresse vérifiée');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la correction immobilière' }));
    expect(await screen.findByText('L’enregistrement immobilier a été conservé avec son historique.')).toBeVisible();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Baux et échéances' })).toBeEnabled());
    expect(screen.getByText('Lyon · Adresse non renseignée')).toBeVisible();
    expect(api.listPropertiesAt).toHaveBeenCalledTimes(3);
    expect(api.listRentalsAt).toHaveBeenCalledTimes(3);
    expect(onChanged).toHaveBeenCalledOnce();
    expect(api.saveProperty).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({ id: 'property', expected_revision_id: 'property-r1', city: 'Lyon' }));
    expect(screen.getByText('L’enregistrement immobilier a été conservé avec son historique.')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Enregistrer la correction immobilière' })).not.toBeInTheDocument();
  });

  it('IMMO-03 bloque tous les onglets pendant la sauvegarde de la fiche enfant', async () => {
    let finishSave!: (value: PropertySnapshot['properties'][number]) => void;
    vi.mocked(api.saveProperty).mockReturnValue(new Promise(resolve => { finishSave = resolve; }));
    const { user, props, onChanged } = await open();
    await user.click(screen.getByRole('button', { name: 'Modifier la fiche de Maison de test' }));
    await user.type(screen.getByLabelText('Ville'), 'Lille');
    await user.type(screen.getByLabelText('Motif de l’enregistrement ou de la correction'), 'Adresse documentée');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la correction immobilière' }));
    const tabs = within(screen.getByRole('group', { name: 'Sections immobilières' })).getAllByRole('button');
    for (const tab of tabs) expect(tab).toBeDisabled();
    expect(props.onBusyChange).toHaveBeenLastCalledWith(true);
    await user.click(screen.getByRole('button', { name: 'Baux et échéances' }));
    expect(screen.getByLabelText('Ville')).toHaveValue('Lille');
    expect(screen.queryByRole('button', { name: 'Ajouter un bail' })).not.toBeInTheDocument();
    expect(onChanged).not.toHaveBeenCalled();
    await act(async () => { finishSave({ ...properties.properties[0], city: 'Lille' }); });
    await screen.findByText('L’enregistrement immobilier a été conservé avec son historique.');
    await waitFor(() => { for (const tab of tabs) expect(tab).toBeEnabled(); });
    expect(onChanged).toHaveBeenCalledOnce();
    expect(props.onBusyChange).toHaveBeenLastCalledWith(false);
    expect(api.saveProperty).toHaveBeenCalledOnce();
    expect(api.saveLease).not.toHaveBeenCalled();
  });

  it('IMMO-04 distingue les unités et baux homonymes et transmet les identifiants choisis', async () => {
    const otherHouse = { ...registry.assets[0], id: 'house-b', revision_id: 'house-b-r1' };
    const otherUnit = { ...properties.units[0], id: 'unit-b', revision_id: 'unit-b-r1', property_asset_id: 'house-b' };
    const otherLease = { ...lease, id: 'lease-b', revision_id: 'lease-b-r1', unit_id: 'unit-b', valid_from: '2026-02-01', valid_to: '2026-12-01' };
    vi.mocked(api.listPropertiesAt).mockResolvedValue({ ...properties, properties: [...properties.properties, { ...properties.properties[0], id: 'property-b', asset_id: 'house-b' }], units: [...properties.units, otherUnit] });
    vi.mocked(api.listRentalsAt).mockResolvedValue({ ...rentals, leases: [lease, otherLease] });
    const { user } = await open(null, { ...registry, assets: [...registry.assets, otherHouse] });
    await user.click(screen.getByRole('button', { name: 'Baux et échéances' }));
    await user.click(screen.getByRole('button', { name: 'Ajouter un bail' }));
    const unitSelect = screen.getByLabelText('Unité louée');
    expect(within(unitSelect).getByRole('option', { name: 'Maison de test · house · A · Logement A · unit' })).toBeVisible();
    await user.selectOptions(unitSelect, within(unitSelect).getByRole('option', { name: 'Maison de test · house-b · A · Logement A · unit-b' }));
    await user.selectOptions(screen.getByLabelText('Locataire identifié'), 'tenant');
    fireEvent.change(screen.getByLabelText('Début de validité'), { target: { value: '2027-01-01' } });
    await user.type(screen.getByLabelText('Motif de l’enregistrement locatif'), 'Nouveau bail du second bien');
    await user.click(screen.getByRole('button', { name: 'Créer l’enregistrement locatif' }));
    await waitFor(() => expect(api.saveLease).toHaveBeenCalledOnce());
    expect(api.saveLease).toHaveBeenCalledWith(expect.objectContaining({ unit_id: 'unit-b', valid_from: '2027-01-01' }));
    await screen.findByText('L’enregistrement locatif a été conservé avec son historique.');
    await user.click(screen.getByRole('button', { name: 'Ajouter des conditions de bail' }));
    const leaseSelect = screen.getByLabelText('Bail concerné');
    expect(within(leaseSelect).getByRole('option', { name: 'Maison de test · house · A · Logement A · Locataire identifié · 2026-01-01 → sans fin · lease' })).toBeVisible();
    await user.selectOptions(leaseSelect, within(leaseSelect).getByRole('option', { name: 'Maison de test · house-b · A · Logement A · Locataire identifié · 2026-02-01 → 2026-12-01 · lease-b' }));
    await user.type(screen.getByLabelText('Loyer contractuel hors charges'), '750');
    await user.type(screen.getByLabelText('Motif de l’enregistrement locatif'), 'Conditions du second bail');
    await user.click(screen.getByRole('button', { name: 'Créer l’enregistrement locatif' }));
    await waitFor(() => expect(api.saveLeaseTerms).toHaveBeenCalledOnce());
    expect(api.saveLeaseTerms).toHaveBeenCalledWith(expect.objectContaining({ lease_id: 'lease-b', rent: '750' }));
  });

  it('crée un bail rattaché à une unité et un locataire existants avec fin exclusive', async () => {
    const { user, onChanged } = await open();
    await user.click(screen.getByRole('button', { name: 'Baux et échéances' }));
    await user.click(screen.getByRole('button', { name: 'Ajouter un bail' }));
    await user.selectOptions(screen.getByLabelText('Unité louée'), 'unit');
    await user.selectOptions(screen.getByLabelText('Locataire identifié'), 'tenant');
    fireEvent.change(screen.getByLabelText('Début de validité'), { target: { value: '2027-01-01' } });
    fireEvent.change(screen.getByLabelText('Fin de validité (exclue)'), { target: { value: '2028-01-01' } });
    await user.type(screen.getByLabelText('Motif de l’enregistrement locatif'), 'Nouveau contrat signé');
    await user.click(screen.getByRole('button', { name: 'Créer l’enregistrement locatif' }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.saveLease).toHaveBeenCalledExactlyOnceWith({ id: null, expected_revision_id: null, reason: 'Nouveau contrat signé', unit_id: 'unit', tenant_entity_id: 'tenant', valid_from: '2027-01-01', valid_to: '2028-01-01', notes: '' });
    expect(api.saveAsset).not.toHaveBeenCalled();
  });

  it('transmet les loyers et le dépôt TND comme textes précis, sans créer de paiement de dépôt', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Baux et échéances' }));
    await user.click(screen.getByRole('button', { name: 'Ajouter des conditions de bail' }));
    await user.selectOptions(screen.getByLabelText('Bail concerné'), 'lease');
    await user.type(screen.getByLabelText('Loyer contractuel hors charges'), '1000,12345678');
    await user.selectOptions(screen.getByLabelText('Devise de l’enregistrement'), 'TND');
    await user.selectOptions(screen.getByLabelText('Fréquence contractuelle'), 'quarterly');
    await user.selectOptions(screen.getByLabelText('Jour d’exigibilité'), '31');
    const deposit = screen.getByLabelText('Dépôt de garantie requis');
    await user.clear(deposit); await user.type(deposit, '2000,001');
    await user.type(screen.getByLabelText('Motif de l’enregistrement locatif'), 'Conditions contractuelles');
    await user.click(screen.getByRole('button', { name: 'Créer l’enregistrement locatif' }));
    await waitFor(() => expect(api.saveLeaseTerms).toHaveBeenCalledOnce());
    expect(api.saveLeaseTerms).toHaveBeenCalledWith(expect.objectContaining({ rent: '1000,12345678', charges: '0', currency: 'TND', frequency: 'quarterly', due_day: 31, deposit_required: '2000,001' }));
    expect(api.saveRentPayment).not.toHaveBeenCalled();
  });

  it('génère par période explicite et borne optimiste, puis annonce créations et échéances existantes', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Baux et échéances' }));
    await user.click(screen.getByRole('button', { name: 'Générer les échéances' }));
    await user.selectOptions(screen.getByLabelText('Bail à générer'), 'lease');
    fireEvent.change(screen.getByLabelText('Début de génération'), { target: { value: '2026-01-01' } });
    fireEvent.change(screen.getByLabelText('Fin de génération (exclue)'), { target: { value: '2027-01-01' } });
    await user.type(screen.getByLabelText('Motif de la génération'), 'Échéancier annuel');
    await user.click(screen.getByRole('button', { name: 'Confirmer la génération' }));
    expect(await screen.findByText('1 échéance(s) créée(s), 2 déjà présente(s).')).toBeVisible();
    expect(api.generateRentDues).toHaveBeenCalledExactlyOnceWith({ lease_id: 'lease', from_date: '2026-01-01', to_date: '2027-01-01', expected_sequence: 27, reason: 'Échéancier annuel' });
  });

  it('déclare un encaissement et son bénéficiaire sans crédit bancaire implicite', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Paiements et dépenses' }));
    await user.click(screen.getByRole('button', { name: 'Enregistrer un paiement' }));
    await user.selectOptions(screen.getByLabelText('Bail concerné'), 'lease');
    await user.type(screen.getByLabelText('Montant du paiement'), '400,01');
    await user.selectOptions(screen.getByLabelText('Bénéficiaire de l’encaissement déclaré'), 'recipient');
    await user.type(screen.getByLabelText('Référence du paiement'), 'REÇU-2');
    await user.type(screen.getByLabelText('Motif de non-rapprochement bancaire'), 'Relevé à rapprocher en phase 6');
    await user.type(screen.getByLabelText('Motif de l’enregistrement locatif'), 'Paiement reçu');
    await user.click(screen.getByRole('button', { name: 'Créer l’enregistrement locatif' }));
    await waitFor(() => expect(api.saveRentPayment).toHaveBeenCalledOnce());
    expect(api.saveRentPayment).toHaveBeenCalledWith(expect.objectContaining({ amount: '400,01', kind: 'receipt', original_payment_id: null, recipient_entity_id: 'recipient', reference: 'REÇU-2', unlinked_reason: 'Relevé à rapprocher en phase 6' }));
    expect(api.saveRentAllocation).not.toHaveBeenCalled();
  });

  it('rattache un remboursement signé au paiement d’origine', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Paiements et dépenses' }));
    await user.click(screen.getByRole('button', { name: 'Enregistrer un paiement' }));
    await user.selectOptions(screen.getByLabelText('Bail concerné'), 'lease');
    await user.selectOptions(screen.getByLabelText('Nature du paiement'), 'refund');
    await user.selectOptions(screen.getByLabelText('Paiement d’origine remboursé'), 'payment');
    await user.type(screen.getByLabelText('Montant du paiement'), '-100');
    await user.selectOptions(screen.getByLabelText('Bénéficiaire de l’encaissement déclaré'), 'owner');
    await user.type(screen.getByLabelText('Motif de non-rapprochement bancaire'), 'Remboursement à rapprocher');
    await user.type(screen.getByLabelText('Motif de l’enregistrement locatif'), 'Trop-perçu remboursé');
    await user.click(screen.getByRole('button', { name: 'Créer l’enregistrement locatif' }));
    await waitFor(() => expect(api.refundRentPayment).toHaveBeenCalledOnce());
    expect(api.refundRentPayment).toHaveBeenCalledWith({ payment: expect.objectContaining({ kind: 'refund', amount: '-100', original_payment_id: 'payment' }), allocations: [], expected_sequence: 27 });
    expect(api.saveRentPayment).not.toHaveBeenCalled();
  });

  it('conserve le formulaire et n’annonce aucune réussite pour une suraffectation refusée par Rust', async () => {
    vi.mocked(api.saveRentAllocation).mockRejectedValue({ message: 'Le montant dépasse le disponible du paiement.' });
    const { user, onChanged } = await open();
    await user.click(screen.getByRole('button', { name: 'Paiements et dépenses' }));
    await user.click(screen.getByRole('button', { name: 'Affecter un paiement' }));
    await user.selectOptions(screen.getByLabelText('Paiement à affecter'), 'payment');
    await user.selectOptions(screen.getByLabelText('Échéance réglée'), 'due');
    await user.type(screen.getByLabelText('Montant affecté'), '700');
    await user.type(screen.getByLabelText('Motif de l’enregistrement locatif'), 'Affectation contrôlée');
    await user.click(screen.getByRole('button', { name: 'Créer l’enregistrement locatif' }));
    expect(await screen.findByText('Le montant dépasse le disponible du paiement.')).toBeVisible();
    expect(screen.getByLabelText('Montant affecté')).toHaveValue('700');
    expect(onChanged).not.toHaveBeenCalled();
    expect(api.saveRentAllocation).toHaveBeenCalledWith(expect.objectContaining({ payment_id: 'payment', due_id: 'due', component: 'rent', amount: '700', currency: 'EUR' }));
  });

  it('affiche les montants du moteur, les inconnus et le retard historique sans calcul de remplacement', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Indicateurs locatifs' }));
    fireEvent.change(screen.getByLabelText('Début de la période locative'), { target: { value: '2026-03-01' } });
    fireEvent.change(screen.getByLabelText('Fin de la période locative (exclue)'), { target: { value: '2026-04-01' } });
    await user.click(screen.getByRole('button', { name: 'Calculer les indicateurs locatifs' }));
    expect(await screen.findByRole('heading', { name: 'Résultats locatifs à la séquence 27' })).toBeVisible();
    const result = screen.getByText('Cash-flow après financement', { selector: 'dt' }).parentElement!;
    expect(within(result).getByText('Non calculable')).toBeVisible();
    expect(screen.getByText('Loyers encaissés sur la période', { selector: 'dt' }).parentElement).toHaveTextContent('600.00 EUR');
    expect(screen.getByText('Loyers impayés à la date de référence', { selector: 'dt' }).parentElement).toHaveTextContent('400.00 EUR');
    expect(screen.getByText('Retard historique conservé')).toBeVisible();
    await user.click(screen.getByText('Sources et taux de conversion'));
    expect(screen.getByText(/Source : Cours manuel du relevé · révision manual-rate-r2/)).toBeVisible();
    expect(screen.getByText(/date de conversion : 2026-03-05/)).toBeVisible();
    expect(screen.getByText('terms-r1, due-r1, payment-r1')).toBeVisible();
    expect(api.getRentalReport).toHaveBeenCalledExactlyOnceWith({ context: 'family', person_id: null, asset_id: null, country: null, from_date: '2026-03-01', to_date: '2026-04-01', date: '2026-03-31', knowledge_sequence: null });
  });

  it('verrouille chaque écran de saisie à K=0 mais autorise une consultation calculée à K=0', async () => {
    const { user } = await open(0);
    expect(screen.getByRole('button', { name: 'Ajouter une fiche immobilière' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Baux et échéances' }));
    for (const name of ['Ajouter un bail', 'Ajouter des conditions de bail', 'Générer les échéances', 'Ajouter une échéance rectificative']) expect(screen.getByRole('button', { name })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Paiements et dépenses' }));
    for (const name of ['Enregistrer un paiement', 'Affecter un paiement', 'Ajouter une dépense']) expect(screen.getByRole('button', { name })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Indicateurs locatifs' }));
    await user.click(screen.getByRole('button', { name: 'Calculer les indicateurs locatifs' }));
    await waitFor(() => expect(api.getRentalReport).toHaveBeenCalledOnce());
    expect(api.listPropertiesAt).toHaveBeenCalledWith(0);
    expect(api.listRentalsAt).toHaveBeenCalledWith(0);
    expect(api.getRentalReport).toHaveBeenCalledWith(expect.objectContaining({ knowledge_sequence: 0 }));
    expect(api.saveLease).not.toHaveBeenCalled();
  });

  it('rembourse et reprend les affectations dans une seule commande atomique', async () => {
    vi.mocked(api.listRentalsAt).mockResolvedValue({ ...rentals, allocations: [{ id: 'allocated', revision_id: 'allocated-r1', payment_id: 'payment', due_id: 'due', component: 'rent', amount: '600', currency: 'EUR', effective_on: '2026-03-05' }] });
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Paiements et dépenses' }));
    await user.click(screen.getByRole('button', { name: 'Enregistrer un paiement' }));
    await user.selectOptions(screen.getByLabelText('Bail concerné'), 'lease');
    await user.selectOptions(screen.getByLabelText('Nature du paiement'), 'refund');
    await user.selectOptions(screen.getByLabelText('Paiement d’origine remboursé'), 'payment');
    await user.type(screen.getByLabelText('Montant du paiement'), '-100,125');
    await user.selectOptions(screen.getByLabelText('Bénéficiaire de l’encaissement déclaré'), 'owner');
    await user.type(screen.getByLabelText('Motif de non-rapprochement bancaire'), 'Remboursement à rapprocher');
    await user.click(screen.getByRole('button', { name: 'Ajouter une reprise d’affectation' }));
    await user.selectOptions(screen.getByLabelText('Échéance reprise 1'), 'due');
    await user.type(screen.getByLabelText('Montant repris 1'), '-100,125');
    await user.type(screen.getByLabelText('Motif de l’enregistrement locatif'), 'Reprise motivée du règlement');
    await user.click(screen.getByRole('button', { name: 'Créer l’enregistrement locatif' }));
    await waitFor(() => expect(api.refundRentPayment).toHaveBeenCalledOnce());
    expect(api.refundRentPayment).toHaveBeenCalledExactlyOnceWith({ payment: expect.objectContaining({ kind: 'refund', amount: '-100,125', original_payment_id: 'payment' }), allocations: [{ due_id: 'due', component: 'rent', amount: '-100,125' }], expected_sequence: 27 });
    expect(api.saveRentPayment).not.toHaveBeenCalled();
    expect(api.saveRentAllocation).not.toHaveBeenCalled();
  });

  it('corrige un remboursement et remplace toutes ses reprises avec leurs bornes de révision et de séquence', async () => {
    const refund = { ...payment, id: 'refund', revision_id: 'refund-r2', kind: 'refund', amount: '-200', original_payment_id: 'payment' };
    vi.mocked(api.listRentalsAt).mockResolvedValue({ ...rentals, payments: [payment, refund], allocations: [
      { id: 'positive', revision_id: 'positive-r1', payment_id: 'payment', due_id: 'due', component: 'rent', amount: '600', currency: 'EUR', effective_on: '2026-03-05' },
      { id: 'negative', revision_id: 'negative-r1', payment_id: 'refund', due_id: 'due', component: 'rent', amount: '-200', currency: 'EUR', effective_on: '2026-03-31' },
    ] });
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Paiements et dépenses' }));
    const row = screen.getByText('-200 EUR').closest('tr')!;
    await user.click(within(row).getByRole('button', { name: 'Modifier' }));
    expect(screen.getByLabelText('Montant repris 1')).toHaveValue('-200');
    const amount = screen.getByLabelText('Montant du paiement');
    await user.clear(amount); await user.type(amount, '-100');
    const reversed = screen.getByLabelText('Montant repris 1');
    await user.clear(reversed); await user.type(reversed, '-100');
    await user.type(screen.getByLabelText('Motif de l’enregistrement locatif'), 'Rectifier le trop-perçu remboursé');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la correction locative' }));
    await waitFor(() => expect(api.refundRentPayment).toHaveBeenCalledOnce());
    expect(api.refundRentPayment).toHaveBeenCalledExactlyOnceWith({ payment: expect.objectContaining({ id: 'refund', expected_revision_id: 'refund-r2', amount: '-100', kind: 'refund' }), allocations: [{ due_id: 'due', component: 'rent', amount: '-100' }], expected_sequence: 27 });
    expect(api.saveRentPayment).not.toHaveBeenCalled();
    expect(api.saveRentAllocation).not.toHaveBeenCalled();
  });

  it('ne conserve pas les anciennes fiches lorsque le chargement d’une autre connaissance échoue', async () => {
    const { props, rerender } = await open();
    expect(screen.getByRole('button', { name: 'Ajouter une fiche immobilière' })).toBeVisible();
    vi.mocked(api.listPropertiesAt).mockRejectedValue({ message: 'La borne historique est indisponible.' });
    vi.mocked(api.listRentalsAt).mockRejectedValue({ message: 'La borne historique est indisponible.' });
    rerender(<RealEstatePage {...props} context={{ ...props.context, knowledgeSequence: 0 }} />);
    expect(await screen.findByText('La borne historique est indisponible.')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Ajouter une fiche immobilière' })).not.toBeInTheDocument();
    expect(screen.queryByText('A · Logement A')).not.toBeInTheDocument();
    expect(api.listPropertiesAt).toHaveBeenLastCalledWith(0);
  });

  it('conserve le périmètre familial avec un filtre de bien, puis exige un choix explicite pour le bien entier', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Indicateurs locatifs' }));
    await user.selectOptions(screen.getByLabelText('Bien à consulter'), 'house');
    expect(screen.getByLabelText('Analyser le bien entier')).not.toBeChecked();
    await user.click(screen.getByRole('button', { name: 'Calculer les indicateurs locatifs' }));
    await waitFor(() => expect(api.getRentalReport).toHaveBeenCalledTimes(1));
    expect(api.getRentalReport).toHaveBeenLastCalledWith(expect.objectContaining({ context: 'family', asset_id: 'house', person_id: null }));
    await user.click(screen.getByLabelText('Analyser le bien entier'));
    await user.click(screen.getByRole('button', { name: 'Calculer les indicateurs locatifs' }));
    await waitFor(() => expect(api.getRentalReport).toHaveBeenCalledTimes(2));
    expect(api.getRentalReport).toHaveBeenLastCalledWith(expect.objectContaining({ context: 'property', asset_id: 'house', person_id: null }));
  });

  it('demande une attribution société et un pays daté au moteur sans réaffectation locale', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Indicateurs locatifs' }));
    await user.selectOptions(screen.getByLabelText('Attribution des indicateurs'), 'company');
    await user.selectOptions(screen.getByLabelText('Pays des biens à la date de référence'), 'TN');
    await user.click(screen.getByRole('button', { name: 'Calculer les indicateurs locatifs' }));
    await waitFor(() => expect(api.getRentalReport).toHaveBeenCalledOnce());
    expect(api.getRentalReport).toHaveBeenCalledWith(expect.objectContaining({ context: 'company', person_id: 'company', country: 'TN', asset_id: null, date: '2026-03-31' }));
  });

  it('corrige une version précise et conserve la saisie après un conflit concurrent', async () => {
    vi.mocked(api.saveLeaseTerms).mockRejectedValue({ message: 'La révision attendue est périmée.' });
    const { user, onChanged } = await open();
    await user.click(screen.getByRole('button', { name: 'Baux et échéances' }));
    const row = screen.getByText('Dépôt contractuel').closest('table')!;
    await user.click(within(row).getByRole('button', { name: 'Modifier' }));
    const rent = screen.getByLabelText('Loyer contractuel hors charges');
    await user.clear(rent); await user.type(rent, '1100');
    await user.type(screen.getByLabelText('Motif de l’enregistrement locatif'), 'Correction contractuelle');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la correction locative' }));
    expect(await screen.findByText('La révision attendue est périmée.')).toBeVisible();
    expect(rent).toHaveValue('1100');
    expect(onChanged).not.toHaveBeenCalled();
    expect(api.saveLeaseTerms).toHaveBeenCalledWith(expect.objectContaining({ id: 'terms', expected_revision_id: 'terms-r1', rent: '1100' }));
  });
});
