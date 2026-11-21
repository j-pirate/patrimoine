import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../services/api';
import { emptyRegistry } from '../services/registry-types';
import type { RegistrySnapshot } from '../services/registry-types';
import type { TreasurySnapshot } from '../services/treasury-types';
import type { RentalSnapshot } from '../services/rental-types';
import CashPostings from './treasury-postings';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});
const registry: RegistrySnapshot = { ...emptyRegistry, knowledge_sequence: 42, entities: [{ id: 'owner', revision_id: 'owner-r', kind: 'person', display_name: 'Bénéficiaire documenté', first_name: null, last_name: null, birth_date: null, residence_country: 'FR', preferred_currency: 'EUR', activity: '', notes: '', archived: false }], assets: [{ id: 'house', revision_id: 'house-r', name: 'Maison', country: 'FR', category: 'real_estate', notes: '', archived: false }, { id: 'bank-asset', revision_id: 'bank-asset-r', name: 'Banque de réception', country: 'FR', category: 'other', notes: '', archived: false }] };
const payment = { id: 'payment', revision_id: 'payment-r3', lease_id: 'lease', kind: 'receipt', amount: '400', currency: 'EUR', effective_on: '2026-03-05', original_payment_id: null, recipient_entity_id: 'owner', reference: 'Réception déjà déclarée', unlinked_reason: 'Pas encore rapprochée' };
const rentals: RentalSnapshot = { leases: [{ id: 'lease', revision_id: 'lease-r', unit_id: 'unit', tenant_entity_id: 'owner', valid_from: '2026-01-01', valid_to: null, notes: '' }], lease_terms: [], lease_term_revisions: [], dues: [{ id: 'due', revision_id: 'due-r', lease_id: 'lease', terms_revision_id: 'terms-r', period_start: '2026-03-01', period_end: '2026-04-01', due_on: '2026-03-05', rent: '1000', charges: '0', currency: 'EUR', generation_key: 'due-key', notes: '' }], payments: [payment], allocations: [{ id: 'allocation', revision_id: 'allocation-r', payment_id: 'payment', due_id: 'due', component: 'rent', amount: '400', currency: 'EUR', effective_on: '2026-03-05' }], expenses: [], knowledge_sequence: 42 };
const treasury: TreasurySnapshot = { accounts: [{ id: 'bank', revision_id: 'bank-r', asset_id: 'bank-asset', kind: 'bank', currency: 'EUR', tracking_start: '2026-01-01', legal_owner_entity_id: 'owner', bank_name: 'Banque', reference: '', country: 'FR', storage_location: '', notes: '', archived: false }], transactions: [{ id: 'journal', revision_id: 'journal-r2', kind: 'receipt', effective_on: '2026-03-05', label: 'Entrée bancaire déjà comptée', reference: 'REF', notes: '', entries: [{ line_id: 'line', account_id: 'bank', amount: '400', role: 'principal' }] }], reconciliations: [], cash_links: [], deposits: [], knowledge_sequence: 42 };
const properties = { properties: [{ id: 'property', revision_id: 'property-r', asset_id: 'house', property_type: 'house', city: '', address: '', surface_m2: null, status: 'rented', valid_from: '2026-01-01', valid_to: null, notes: '' }], units: [{ id: 'unit', revision_id: 'unit-r', property_asset_id: 'house', reference: 'LOGEMENT', name: 'Logement', surface_m2: null, valid_from: '2026-01-01', valid_to: null, available_for_rent: true, notes: '' }], events: [], knowledge_sequence: 42 };
const debts = { contracts: [{ id: 'loan', revision_id: 'loan-r', name: 'Prêt documenté', currency: 'EUR', tracking_start: '2026-01-01', notes: '', archived: false }], terms: [], movements: [], receivable_adjustments: [], schedules: [], payment_allocations: [], asset_links: [], guarantees: [], coverages: [], knowledge_sequence: 42 };
function open(k: number | null = null) {
  const props = { registry, context: { personId: null, date: '2026-03-31', knowledgeSequence: k }, treasury, rentals, properties, debts, onChanged: vi.fn().mockResolvedValue(undefined), onBusyChange: vi.fn() };
  return { user: userEvent.setup(), props, ...render(<CashPostings {...props} />) };
}
async function commonRent(user: ReturnType<typeof userEvent.setup>) {
  await user.selectOptions(screen.getByLabelText('Bail concerné'), 'lease');
  await user.selectOptions(screen.getByLabelText('Bénéficiaire déclaré'), 'owner');
}
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.postRentPayment).mockResolvedValue(payment);
  vi.mocked(api.postRentalExpense).mockResolvedValue({ id: 'expense', revision_id: 'expense-r', asset_id: 'house', kind: 'operating', amount: '2000', currency: 'EUR', effective_on: '2026-03-01', paid_on: '2026-03-10', recoverable: false, notes: '' });
  vi.mocked(api.postDeposit).mockResolvedValue({ id: 'deposit', revision_id: 'deposit-r', lease_id: 'lease', kind: 'received', amount: '2000', currency: 'EUR', effective_on: '2026-03-31', recipient_entity_id: 'owner', due_id: null, component: '', notes: '' });
  vi.mocked(api.postDebtMovement).mockResolvedValue({ id: 'repayment', revision_id: 'repayment-r', debt_id: 'loan', kind: 'principal_repayment', effective_on: '2026-03-31', principal_change: '-1000', interest_paid: '200', fees_paid: '10', unlinked_reason: '', notes: '' });
});

describe('Sources et trésorerie dans une même opération', () => {
  it('IMMO-04 distingue les baux homonymes dans le poste bancaire et limite leurs échéances', async () => {
    const otherHouse = { ...registry.assets[0], id: 'house-b', revision_id: 'house-b-r' };
    const otherUnit = { ...properties.units[0], id: 'unit-b', revision_id: 'unit-b-r', property_asset_id: 'house-b' };
    const otherLease = { ...rentals.leases[0], id: 'lease-b', revision_id: 'lease-b-r', unit_id: 'unit-b', valid_from: '2026-02-01', valid_to: '2026-12-01' };
    const otherDue = { ...rentals.dues[0], id: 'due-b', revision_id: 'due-b-r', lease_id: 'lease-b' };
    const { user, props, rerender } = open();
    rerender(<CashPostings {...props}
      registry={{ ...registry, assets: [...registry.assets, otherHouse] }}
      properties={{ ...properties, properties: [...properties.properties, { ...properties.properties[0], id: 'property-b', asset_id: 'house-b' }], units: [...properties.units, otherUnit] }}
      rentals={{ ...rentals, leases: [...rentals.leases, otherLease], dues: [...rentals.dues, otherDue] }} />);
    await user.click(screen.getByRole('button', { name: 'Enregistrer un loyer en trésorerie' }));
    const leaseSelect = screen.getByLabelText('Bail concerné');
    expect(within(leaseSelect).getByRole('option', { name: 'Maison · house · LOGEMENT · Logement · Bénéficiaire documenté · 2026-01-01 → sans fin · lease' })).toBeVisible();
    await user.selectOptions(leaseSelect, within(leaseSelect).getByRole('option', { name: 'Maison · house-b · LOGEMENT · Logement · Bénéficiaire documenté · 2026-02-01 → 2026-12-01 · lease-b' }));
    await user.selectOptions(screen.getByLabelText('Bénéficiaire déclaré'), 'owner');
    await user.type(screen.getByLabelText('Montant signé de la source'), '400');
    await user.selectOptions(screen.getByLabelText('Compte du mouvement bancaire'), 'bank');
    await user.click(screen.getByRole('button', { name: 'Ajouter une affectation au loyer' }));
    const dueSelect = screen.getByLabelText('Échéance affectée 1');
    expect(within(dueSelect).getAllByRole('option').map(option => option.getAttribute('value'))).toEqual(['', 'due-b']);
    await user.selectOptions(dueSelect, 'due-b');
    await user.type(screen.getByLabelText('Montant affecté 1'), '400');
    await user.type(screen.getByLabelText('Motif de l’opération liée'), 'Loyer du second bien identifié');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source et la trésorerie ensemble' }));
    await waitFor(() => expect(api.postRentPayment).toHaveBeenCalledOnce());
    expect(api.postRentPayment).toHaveBeenCalledWith(expect.objectContaining({
      payment: expect.objectContaining({ lease_id: 'lease-b' }),
      allocations: [{ due_id: 'due-b', component: 'rent', amount: '400' }],
      cash: expect.objectContaining({ transaction: expect.objectContaining({ label: expect.stringContaining('house-b · LOGEMENT · Logement') }) }),
    }));
    expect(api.saveRentPayment).not.toHaveBeenCalled();
    expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('poste le paiement, ses affectations et sa ligne de banque via une seule commande', async () => {
    const { user, props } = open();
    await user.click(screen.getByRole('button', { name: 'Enregistrer un loyer en trésorerie' }));
    await commonRent(user);
    await user.type(screen.getByLabelText('Montant signé de la source'), '400,00');
    await user.selectOptions(screen.getByLabelText('Compte du mouvement bancaire'), 'bank');
    await user.click(screen.getByRole('button', { name: 'Ajouter une affectation au loyer' }));
    await user.selectOptions(screen.getByLabelText('Échéance affectée 1'), 'due');
    await user.type(screen.getByLabelText('Montant affecté 1'), '400,00');
    await user.type(screen.getByLabelText('Motif de l’opération liée'), 'Réception et affectation documentées');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source et la trésorerie ensemble' }));
    await waitFor(() => expect(props.onChanged).toHaveBeenCalledOnce());
    const input = vi.mocked(api.postRentPayment).mock.calls[0][0];
    expect(input.expected_sequence).toBe(42);
    expect(input.payment).toMatchObject({ id: null, lease_id: 'lease', amount: '400,00', recipient_entity_id: 'owner' });
    expect(input.allocations).toEqual([{ due_id: 'due', component: 'rent', amount: '400,00' }]);
    expect(input.cash.transaction?.entries).toEqual([{ line_id: input.cash.line_id, account_id: 'bank', amount: '400,00', role: 'principal' }]);
    expect(api.saveRentPayment).not.toHaveBeenCalled(); expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('rapproche un paiement existant avec le journal sans nouveau crédit', async () => {
    const { user } = open();
    await user.click(screen.getByRole('button', { name: 'Enregistrer un loyer en trésorerie' }));
    await user.selectOptions(screen.getByLabelText('Source existante à rapprocher ou corriger (facultatif)'), 'payment');
    await user.selectOptions(screen.getByLabelText('Traitement du journal'), 'existing');
    await user.selectOptions(screen.getByLabelText('Opération du journal'), 'journal-r2');
    await user.selectOptions(screen.getByLabelText('Ligne de l’opération'), 'line');
    await user.type(screen.getByLabelText('Motif de l’opération liée'), 'Un seul encaissement bancaire');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source et la trésorerie ensemble' }));
    await waitFor(() => expect(api.postRentPayment).toHaveBeenCalledOnce());
    expect(api.postRentPayment).toHaveBeenCalledWith(expect.objectContaining({ payment: expect.objectContaining({ id: 'payment', expected_revision_id: 'payment-r3', amount: '400' }), allocations: [{ due_id: 'due', component: 'rent', amount: '400' }], cash: { transaction: null, existing_transaction_revision_id: 'journal-r2', line_id: 'line' } }));
    expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('ne réduit pas une ligne bancaire partiellement rapprochée au seul montant du lien', async () => {
    const { user, props, rerender } = open();
    const linked: TreasurySnapshot = { ...treasury,
      transactions: [{ ...treasury.transactions[0], entries: [{ ...treasury.transactions[0].entries[0], amount: '1000' }] }],
      cash_links: [{ id: 'link', revision_id: 'link-r1', source_kind: 'rent_payment', source_id: 'payment', source_revision_id: 'payment-r3', transaction_revision_id: 'journal-r2', line_id: 'line', amount: '400', component: 'full', side: '', notes: '' }],
    };
    rerender(<CashPostings {...props} treasury={linked} />);
    await user.click(screen.getByRole('button', { name: 'Enregistrer un loyer en trésorerie' }));
    await user.selectOptions(screen.getByLabelText('Source existante à rapprocher ou corriger (facultatif)'), 'payment');
    await user.selectOptions(screen.getByLabelText('Traitement du journal'), 'correct');
    expect(screen.getByLabelText('Montant bancaire signé (vide : conserver le journal)')).toHaveValue('1000');
    await user.clear(screen.getByLabelText('Montant bancaire signé (vide : conserver le journal)'));
    await user.type(screen.getByLabelText('Motif de l’opération liée'), 'Correction des notes, banque conservée');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source et la trésorerie ensemble' }));
    await waitFor(() => expect(api.postRentPayment).toHaveBeenCalledOnce());
    const input = vi.mocked(api.postRentPayment).mock.calls[0][0];
    expect(input.payment.amount).toBe('400');
    expect(input.cash.transaction).toMatchObject({ id: 'journal', expected_revision_id: 'journal-r2', entries: [{ line_id: 'line', account_id: 'bank', amount: '1000', role: 'principal' }] });
  });

  it('applique un dépôt à une échéance sans trésorerie ni faux paiement de loyer', async () => {
    const { user } = open();
    await user.click(screen.getByRole('button', { name: 'Enregistrer un mouvement de dépôt' }));
    await commonRent(user);
    await user.selectOptions(screen.getByLabelText('Nature du mouvement de dépôt'), 'applied_to_due');
    await user.selectOptions(screen.getByLabelText('Échéance compensée'), 'due');
    await user.type(screen.getByLabelText('Montant signé de la source'), '-300');
    expect(screen.queryByLabelText('Compte du mouvement bancaire')).not.toBeInTheDocument();
    await user.type(screen.getByLabelText('Notes de la source'), 'Compensation autorisée du dépôt déjà reçu');
    await user.type(screen.getByLabelText('Motif de l’opération liée'), 'Compensation autorisée, banque inchangée');
    await user.click(screen.getByRole('button', { name: 'Appliquer le dépôt sans mouvement bancaire' }));
    await waitFor(() => expect(api.postDeposit).toHaveBeenCalledOnce());
    expect(api.postDeposit).toHaveBeenCalledWith(expect.objectContaining({ expected_sequence: 42, movement: expect.objectContaining({ kind: 'applied_to_due', amount: '-300', due_id: 'due', component: 'rent' }), cash: null }));
    expect(api.postRentPayment).not.toHaveBeenCalled(); expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('enregistre un dépôt reçu comme source distincte, sans le classer en revenu locatif', async () => {
    const { user } = open();
    await user.click(screen.getByRole('button', { name: 'Enregistrer un mouvement de dépôt' }));
    await commonRent(user);
    await user.type(screen.getByLabelText('Montant signé de la source'), '2000');
    await user.selectOptions(screen.getByLabelText('Compte du mouvement bancaire'), 'bank');
    await user.type(screen.getByLabelText('Notes de la source'), 'Dépôt effectivement reçu pour le contrat');
    await user.type(screen.getByLabelText('Motif de l’opération liée'), 'Dépôt reçu et obligation de restitution');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source et la trésorerie ensemble' }));
    await waitFor(() => expect(api.postDeposit).toHaveBeenCalledOnce());
    const input = vi.mocked(api.postDeposit).mock.calls[0][0];
    expect(input.movement).toMatchObject({ kind: 'received', amount: '2000', due_id: null, component: '' });
    expect(input.cash?.transaction).toMatchObject({ kind: 'receipt', entries: [{ line_id: expect.any(String), account_id: 'bank', amount: '2000', role: 'principal' }] });
    expect(api.postRentPayment).not.toHaveBeenCalled();
  });

  it('enregistre une régularisation justifiée de dépôt sans fabriquer de banque', async () => {
    const { user } = open();
    await user.click(screen.getByRole('button', { name: 'Enregistrer un mouvement de dépôt' }));
    await commonRent(user);
    await user.selectOptions(screen.getByLabelText('Nature du mouvement de dépôt'), 'adjustment');
    await user.type(screen.getByLabelText('Montant signé de la source'), '100');
    await user.type(screen.getByLabelText('Notes de la source'), 'Régularisation documentée de l’obligation');
    await user.type(screen.getByLabelText('Motif de l’opération liée'), 'Ajustement distinct d’un encaissement');
    expect(screen.queryByLabelText('Compte du mouvement bancaire')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Ajuster le dépôt sans mouvement bancaire' }));
    await waitFor(() => expect(api.postDeposit).toHaveBeenCalledOnce());
    expect(api.postDeposit).toHaveBeenCalledWith(expect.objectContaining({ movement: expect.objectContaining({ kind: 'adjustment', amount: '100', component: '', notes: 'Régularisation documentée de l’obligation' }), cash: null }));
  });

  it('sépare date économique et décaissement d’une dépense, et garde le débit signé', async () => {
    const { user } = open();
    await user.click(screen.getByRole('button', { name: 'Enregistrer une dépense payée' }));
    await user.selectOptions(screen.getByLabelText('Bien de la dépense'), 'house');
    await user.type(screen.getByLabelText('Montant positif de la dépense'), '2000');
    fireEvent.change(screen.getByLabelText('Date économique de la source'), { target: { value: '2026-03-01' } });
    fireEvent.change(screen.getByLabelText('Date du décaissement'), { target: { value: '2026-03-10' } });
    await user.selectOptions(screen.getByLabelText('Compte du mouvement bancaire'), 'bank');
    await user.type(screen.getByLabelText('Motif de l’opération liée'), 'Dépense courue puis réellement payée');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source et la trésorerie ensemble' }));
    await waitFor(() => expect(api.postRentalExpense).toHaveBeenCalledOnce());
    const input = vi.mocked(api.postRentalExpense).mock.calls[0][0];
    expect(input.expense).toMatchObject({ amount: '2000', effective_on: '2026-03-01', paid_on: '2026-03-10' });
    expect(input.cash.transaction).toMatchObject({ effective_on: '2026-03-10', entries: [{ line_id: expect.any(String), account_id: 'bank', amount: '-2000', role: 'principal' }] });
  });

  it('transmet principal, intérêts et frais du remboursement sans les soustraire tous du principal', async () => {
    const { user } = open();
    await user.click(screen.getByRole('button', { name: 'Enregistrer un financement en trésorerie' }));
    await user.selectOptions(screen.getByLabelText('Contrat de financement'), 'loan');
    await user.type(screen.getByLabelText('Variation signée du principal'), '-1000');
    await user.clear(screen.getByLabelText('Intérêts réellement payés')); await user.type(screen.getByLabelText('Intérêts réellement payés'), '200');
    await user.clear(screen.getByLabelText('Frais réellement payés')); await user.type(screen.getByLabelText('Frais réellement payés'), '10');
    await user.selectOptions(screen.getByLabelText('Compte du mouvement bancaire'), 'bank');
    await user.type(screen.getByLabelText('Montant bancaire total signé'), '-1210');
    for (const [index, component, amount] of [[0, 'principal', '-1000'], [1, 'interest', '-200'], [2, 'fee', '-10']] as const) {
      await user.click(screen.getByRole('button', { name: 'Ajouter une composante rapprochée' }));
      await user.selectOptions(screen.getByLabelText(`Composante du prêt ${index + 1}`), component);
      await user.type(screen.getByLabelText(`Montant bancaire rapproché ${index + 1}`), amount);
    }
    await user.type(screen.getByLabelText('Motif de l’opération liée'), 'Remboursement réel avec ses composantes');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source et la trésorerie ensemble' }));
    await waitFor(() => expect(api.postDebtMovement).toHaveBeenCalledOnce());
    const input = vi.mocked(api.postDebtMovement).mock.calls[0][0];
    expect(input.movement).toMatchObject({ principal_change: '-1000', interest_paid: '200', fees_paid: '10' });
    expect(input.transaction?.entries).toEqual([{ line_id: expect.any(String), account_id: 'bank', amount: '-1210', role: 'principal' }]);
    expect(input.links.map(l => [l.side, l.component, l.amount])).toEqual([['debtor', 'principal', '-1000'], ['debtor', 'interest', '-200'], ['debtor', 'fee', '-10']]);
    expect(input.links.every(l => l.line_id === input.transaction?.entries[0].line_id)).toBe(true);
    expect(api.saveDebtMovement).not.toHaveBeenCalled(); expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('reste en lecture seule à K=0 et conserve les champs d’une source en conflit', async () => {
    const { user, props, rerender } = open();
    await user.click(screen.getByRole('button', { name: 'Enregistrer un loyer en trésorerie' }));
    await commonRent(user); await user.type(screen.getByLabelText('Montant signé de la source'), '400');
    await user.selectOptions(screen.getByLabelText('Compte du mouvement bancaire'), 'bank');
    await user.type(screen.getByLabelText('Motif de l’opération liée'), 'Saisie à conserver');
    vi.mocked(api.postRentPayment).mockRejectedValueOnce({ message: 'Le journal a changé avant validation.' });
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source et la trésorerie ensemble' }));
    await screen.findByText('Le journal a changé avant validation.');
    expect(screen.getByLabelText('Motif de l’opération liée')).toHaveValue('Saisie à conserver');
    expect(props.onChanged).not.toHaveBeenCalled();
    rerender(<CashPostings {...props} context={{ ...props.context, knowledgeSequence: 0 }} />);
    expect(screen.getByRole('button', { name: 'Enregistrer un loyer en trésorerie' })).toBeDisabled();
    expect(screen.queryByLabelText('Motif de l’opération liée')).not.toBeInTheDocument();
  });
});
