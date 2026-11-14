import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../services/api';
import { emptyRegistry } from '../services/registry-types';
import type { RegistrySnapshot } from '../services/registry-types';
import type { TreasuryAccount, TreasuryReport, TreasurySnapshot } from '../services/treasury-types';
import type { RentalSnapshot } from '../services/rental-types';
import TreasuryPage from './treasury';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});
const registry: RegistrySnapshot = { ...emptyRegistry, knowledge_sequence: 42,
  entities: [{ id: 'owner', revision_id: 'owner-r', kind: 'person', display_name: 'Titulaire identifié', first_name: null, last_name: null, birth_date: null, residence_country: 'FR', preferred_currency: 'EUR', activity: '', notes: '', archived: false }],
  assets: [{ id: 'bank-asset', revision_id: 'bank-asset-r', name: 'Compte courant familial', country: 'FR', category: 'other', notes: '', archived: false }, { id: 'cash-asset', revision_id: 'cash-asset-r', name: 'Caisse tunisienne', country: 'TN', category: 'other', notes: '', archived: false }],
};
const account: TreasuryAccount = { id: 'bank', revision_id: 'bank-r1', asset_id: 'bank-asset', kind: 'bank', currency: 'EUR', tracking_start: '2026-01-01', legal_owner_entity_id: 'owner', bank_name: 'Banque de test', reference: 'REF-BANQUE', country: 'FR', storage_location: '', notes: '', archived: false };
const snapshot: TreasurySnapshot = { accounts: [account, { ...account, id: 'cash', revision_id: 'cash-r1', asset_id: 'cash-asset', kind: 'cash', currency: 'TND', country: 'TN', bank_name: '', storage_location: 'Lieu identifié' }],
  transactions: [{ id: 'receipt', revision_id: 'receipt-r2', kind: 'receipt', effective_on: '2026-03-05', label: 'Loyer reçu au journal', reference: '', notes: '', entries: [{ line_id: 'cash-line', account_id: 'bank', amount: '400', role: 'principal' }] }],
  reconciliations: [], cash_links: [], deposits: [], knowledge_sequence: 42,
};
const rentals: RentalSnapshot = { leases: [], lease_terms: [], lease_term_revisions: [], dues: [], payments: [{ id: 'rent-payment', revision_id: 'rent-payment-r3', lease_id: 'lease', kind: 'receipt', amount: '400', currency: 'EUR', effective_on: '2026-03-05', original_payment_id: null, recipient_entity_id: 'owner', reference: 'Loyer mars déclaré', unlinked_reason: 'À rapprocher' }], allocations: [], expenses: [], knowledge_sequence: 42 };
const report: TreasuryReport = { account_id: 'bank', asset_id: 'bank-asset', date: '2026-03-31', knowledge_sequence: 42, currency: 'EUR', status: 'complete', balance_original: '110.00', exact_balance: { num: '110', den: '1' }, balance_eur: '110.00', exact_balance_eur: { num: '110', den: '1' }, known_movement_total: { num: '10', den: '1' }, overdraft: false, opening_transaction_id: 'opening', executed_fx: [], reconciliations: [{ reconciliation_id: 'statement', revision_id: 'statement-r1', effective_on: '2026-03-31', statement_balance: '100', calculated_balance: '110.00', difference: '-10.00', exact_difference: { num: '-10', den: '1' } }], conversions: [], source_ids: ['opening-r1', 'receipt-r2'], warnings: [] };
const emptyDebts = { contracts: [], terms: [], movements: [], receivable_adjustments: [], schedules: [], payment_allocations: [], asset_links: [], guarantees: [], coverages: [], knowledge_sequence: 42 };
async function open(k: number | null = null) {
  const props = { mode: 'bank' as const, registry, context: { personId: null, date: '2026-03-31', knowledgeSequence: k }, onChanged: vi.fn().mockResolvedValue(undefined), onBusyChange: vi.fn(), onNavigateRegistry: vi.fn() };
  const view = render(<TreasuryPage {...props} />);
  await screen.findByRole('button', { name: 'Ajouter un compte bancaire' });
  return { user: userEvent.setup(), props, ...view };
}
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.listTreasuryAt).mockResolvedValue(snapshot);
  vi.mocked(api.listRentalsAt).mockResolvedValue(rentals);
  vi.mocked(api.listPropertiesAt).mockResolvedValue({ properties: [], units: [], events: [], knowledge_sequence: 42 });
  vi.mocked(api.listDebtsAt).mockResolvedValue(emptyDebts);
  vi.mocked(api.listDocuments).mockResolvedValue([]);
  vi.mocked(api.saveTreasuryAccount).mockResolvedValue(account);
  vi.mocked(api.saveTreasuryTransaction).mockResolvedValue(snapshot.transactions[0]);
  vi.mocked(api.saveAccountReconciliation).mockResolvedValue({ id: 'statement', revision_id: 'statement-r1', account_id: 'bank', effective_on: '2026-03-31', statement_balance: '100', statement_reference: 'Mars', document_id: null, notes: '' });
  vi.mocked(api.saveCashLink).mockResolvedValue({ id: 'link', revision_id: 'link-r1', source_kind: 'rent_payment', source_id: 'rent-payment', source_revision_id: 'rent-payment-r3', transaction_revision_id: 'receipt-r2', line_id: 'cash-line', amount: '400', component: 'full', side: '', notes: '' });
  vi.mocked(api.getTreasuryReport).mockResolvedValue(report);
});

describe('Trésorerie locale', () => {
  it('rattache un compte à son actif et son titulaire, sans inventer d’ouverture', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Ajouter un compte bancaire' }));
    await user.selectOptions(screen.getByLabelText('Actif du compte'), 'bank-asset');
    await user.selectOptions(screen.getByLabelText('Titulaire juridique'), 'owner');
    await user.type(screen.getByLabelText('Banque'), 'Banque déclarée');
    await user.type(screen.getByLabelText('Motif de l’enregistrement de trésorerie'), 'Ouverture du suivi documentaire');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la trésorerie' }));
    await waitFor(() => expect(api.saveTreasuryAccount).toHaveBeenCalledOnce());
    expect(api.saveTreasuryAccount).toHaveBeenCalledWith(expect.objectContaining({ id: null, asset_id: 'bank-asset', legal_owner_entity_id: 'owner', bank_name: 'Banque déclarée', currency: 'EUR' }));
    expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
    expect(api.saveAsset).not.toHaveBeenCalled();
  });

  it('enregistre une ouverture explicitement nulle à la date de début du suivi', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Enregistrer l’ouverture' }));
    expect(screen.getByLabelText('Date de l’opération')).toHaveValue('2026-01-01');
    await user.type(screen.getByLabelText('Libellé de l’opération'), 'Ouverture confirmée');
    await user.type(screen.getByLabelText('Montant signé de la ligne 1'), '0');
    await user.type(screen.getByLabelText('Motif de l’enregistrement de trésorerie'), 'Aucun solde antérieur repris');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la trésorerie' }));
    await waitFor(() => expect(api.saveTreasuryTransaction).toHaveBeenCalledOnce());
    expect(api.saveTreasuryTransaction).toHaveBeenCalledWith(expect.objectContaining({ kind: 'opening', effective_on: '2026-01-01', entries: [{ line_id: expect.any(String), account_id: 'bank', amount: '0', role: 'opening' }] }));
  });

  it('conserve débit, crédit et frais de change en trois lignes décimales distinctes', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Journal et transferts' }));
    await user.click(screen.getByRole('button', { name: 'Ajouter une opération' }));
    await user.selectOptions(screen.getByLabelText('Nature de l’opération'), 'fx');
    await user.type(screen.getByLabelText('Libellé de l’opération'), 'Change réel du relevé');
    await user.selectOptions(screen.getByLabelText('Compte de la ligne 1'), 'bank');
    await user.type(screen.getByLabelText('Montant signé de la ligne 1'), '-100');
    await user.click(screen.getByRole('button', { name: 'Ajouter une ligne au journal' }));
    await user.selectOptions(screen.getByLabelText('Compte de la ligne 2'), 'cash');
    await user.type(screen.getByLabelText('Montant signé de la ligne 2'), '330,000');
    await user.click(screen.getByRole('button', { name: 'Ajouter une ligne au journal' }));
    await user.selectOptions(screen.getByLabelText('Compte de la ligne 3'), 'bank');
    await user.type(screen.getByLabelText('Montant signé de la ligne 3'), '-2');
    await user.selectOptions(screen.getByLabelText('Rôle de la ligne 3'), 'fee');
    await user.type(screen.getByLabelText('Motif de l’enregistrement de trésorerie'), 'Montants exécutés, frais séparés');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la trésorerie' }));
    await waitFor(() => expect(api.saveTreasuryTransaction).toHaveBeenCalledOnce());
    const input = vi.mocked(api.saveTreasuryTransaction).mock.calls[0][0];
    expect(input.entries.map(e => [e.account_id, e.amount, e.role])).toEqual([['bank', '-100', 'principal'], ['cash', '330,000', 'principal'], ['bank', '-2', 'fee']]);
    expect(api.convertCurrency).not.toHaveBeenCalled();
  });

  it('présente un écart de relevé retourné par Rust sans créer d’ajustement', async () => {
    const { user } = await open();
    await user.selectOptions(screen.getByLabelText('Compte du contrôle'), 'bank');
    await user.click(screen.getByRole('button', { name: 'Calculer le solde' }));
    await screen.findByText('-10.00');
    expect(api.getTreasuryReport).toHaveBeenCalledExactlyOnceWith({ account_id: 'bank', date: '2026-03-31', knowledge_sequence: null });
    expect(screen.getByText('110.00 EUR', { selector: 'strong' })).toBeVisible();
    expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('garde un solde sans ouverture inconnu malgré des mouvements connus', async () => {
    vi.mocked(api.getTreasuryReport).mockResolvedValue({ ...report, status: 'partial', balance_original: null, balance_eur: null, exact_balance: null, exact_balance_eur: null, opening_transaction_id: null, reconciliations: [], warnings: [{ code: 'missing_opening', object_id: 'bank', message: 'L’ouverture du suivi n’est pas connue.' }] });
    const { user } = await open();
    await user.selectOptions(screen.getByLabelText('Compte du contrôle'), 'bank');
    await user.click(screen.getByRole('button', { name: 'Calculer le solde' }));
    await screen.findByText('Solde non calculable');
    expect(screen.getByText('L’ouverture du suivi n’est pas connue.')).toBeVisible();
    expect(screen.queryByText('0.00 EUR')).not.toBeInTheDocument();
  });

  it('rapproche les révisions exactes d’un loyer et du journal sans dupliquer le cash', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Rapprocher une source' }));
    await user.click(screen.getByRole('button', { name: 'Rapprocher une source financière' }));
    await user.selectOptions(screen.getByLabelText('Source à rapprocher'), 'rent-payment');
    await user.selectOptions(screen.getByLabelText('Opération déjà enregistrée'), 'receipt-r2');
    await user.selectOptions(screen.getByLabelText('Ligne de trésorerie'), 'cash-line');
    await user.type(screen.getByLabelText('Motif de l’enregistrement de trésorerie'), 'Même paiement, même crédit bancaire');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la trésorerie' }));
    await waitFor(() => expect(api.saveCashLink).toHaveBeenCalledOnce());
    expect(api.saveCashLink).toHaveBeenCalledWith(expect.objectContaining({ source_id: 'rent-payment', source_revision_id: 'rent-payment-r3', transaction_revision_id: 'receipt-r2', line_id: 'cash-line', amount: '400', component: 'full', side: '' }));
    expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
    expect(api.postRentPayment).not.toHaveBeenCalled();
  });

  it('bloque les écritures à K=0 mais permet de consulter un solde historique', async () => {
    const { user } = await open(0);
    expect(screen.getByRole('button', { name: 'Ajouter un compte bancaire' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Enregistrer l’ouverture' })).toBeDisabled();
    await user.selectOptions(screen.getByLabelText('Compte du contrôle'), 'bank');
    await user.click(screen.getByRole('button', { name: 'Calculer le solde' }));
    await waitFor(() => expect(api.getTreasuryReport).toHaveBeenCalledWith(expect.objectContaining({ knowledge_sequence: 0 })));
    expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('ne laisse pas apparaître le profil courant après échec du chargement d’un autre K', async () => {
    const { rerender, props } = await open();
    expect(screen.getByText(/Banque de test/)).toBeVisible();
    vi.mocked(api.listTreasuryAt).mockRejectedValueOnce({ message: 'La séquence historique ne peut pas être chargée.' });
    rerender(<TreasuryPage {...props} context={{ ...props.context, knowledgeSequence: 3 }} />);
    await screen.findByText('La séquence historique ne peut pas être chargée.');
    expect(screen.queryByText(/Banque de test/)).not.toBeInTheDocument();
  });

  it('garde les champs d’une correction refusée et affiche le taux exécuté indépendamment des cours manuels', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Journal et transferts' }));
    await user.click(within(screen.getByText('Loyer reçu au journal').closest('tr')!).getByRole('button', { name: 'Modifier' }));
    await user.type(screen.getByLabelText('Motif de l’enregistrement de trésorerie'), 'Correction en conflit');
    vi.mocked(api.saveTreasuryTransaction).mockRejectedValueOnce({ message: 'La source liée a changé.' });
    await user.click(screen.getByRole('button', { name: 'Enregistrer la correction de trésorerie' }));
    await screen.findByText('La source liée a changé.');
    expect(screen.getByLabelText('Motif de l’enregistrement de trésorerie')).toHaveValue('Correction en conflit');
    fireEvent.change(screen.getByLabelText('Compte du contrôle'), { target: { value: 'bank' } });
    vi.mocked(api.getTreasuryReport).mockResolvedValue({ ...report, executed_fx: [{ transaction_id: 'fx', revision_id: 'fx-r', effective_on: '2026-03-01', from_currency: 'EUR', to_currency: 'TND', debited_amount: '100', credited_amount: '330', exact_rate: { num: '33', den: '10' }, fees: [{ currency: 'EUR', amount: { num: '2', den: '1' } }] }] });
    await user.click(screen.getByRole('button', { name: 'Calculer le solde' }));
    await screen.findByText(/Taux exécuté exact : 33\/10 TND/);
    expect(screen.getByText('Frais séparés : 2/1 EUR.')).toBeVisible();
  });
});
