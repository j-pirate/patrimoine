import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../services/api';
import { emptyRegistry } from '../services/registry-types';
import type { RegistrySnapshot } from '../services/registry-types';
import type { FinancialReport, FinanceSnapshot } from '../services/finance-types';
import FinancialPanel from './financial-report';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});
const registry: RegistrySnapshot = { ...emptyRegistry, knowledge_sequence: 42, entities: [{ id: 'owner', revision_id: 'owner-r1', kind: 'person', display_name: 'Créancier identifié', first_name: null, last_name: null, birth_date: null, residence_country: 'FR', preferred_currency: 'EUR', activity: '', notes: '', archived: false }] };
const finance: FinanceSnapshot = { coverages: [], rent_claims: [], rent_claim_adjustments: [], knowledge_sequence: 42 };
const due = { id: 'due', revision_id: 'due-r1', lease_id: 'lease', terms_revision_id: 'terms-r1', period_start: '2026-03-01', period_end: '2026-04-01', due_on: '2026-03-05', rent: '1000', charges: '0', currency: 'EUR', generation_key: 'due-key', notes: '' };
const report: FinancialReport = { context: 'family', person_id: null, date: '2026-03-31', knowledge_sequence: 42, status: 'partial', coverage_confirmed: false, assets_eur: null, liabilities_eur: null, net_eur: null, exact_assets: null, exact_liabilities: null, exact_net: null, known_assets_eur: '1000.00', known_liabilities_eur: '0.00', known_net_eur: '1000.00', positions: [{ object_id: 'bank', label: 'Banque réelle', kind: 'bank', entity_id: 'owner', country: 'FR', currency: 'EUR', amount_original: '400', exact_original: { num: '400', den: '1' }, weight: { num: '1', den: '1' }, amount_eur: '400.00', exact_eur: { num: '400', den: '1' }, source_ids: ['cash-r2'], conversion: null }, { object_id: 'due', label: 'Reliquat locatif', kind: 'rent_receivable', entity_id: 'owner', country: 'FR', currency: 'EUR', amount_original: '600', exact_original: { num: '600', den: '1' }, weight: { num: '1', den: '1' }, amount_eur: '600.00', exact_eur: { num: '600', den: '1' }, source_ids: ['due-r1', 'claim-r1'], conversion: null }], eliminations: [], source_ids: ['cash-r2', 'due-r1', 'claim-r1'], warnings: [{ code: 'missing_coverage', object_id: 'scope', message: 'La couverture complète du périmètre n’est pas déclarée.' }] };
async function open(k: number | null = null) {
  const props = { registry, context: { personId: null as string | null, date: '2026-03-31', knowledgeSequence: k }, onChanged: vi.fn().mockResolvedValue(undefined), onBusyChange: vi.fn() };
  const view = render(<FinancialPanel {...props} />);
  await waitFor(() => expect(screen.getByRole('button', { name: 'Calculer le bilan financier' })).toBeEnabled());
  return { user: userEvent.setup(), props, ...view };
}
async function declarations(user: ReturnType<typeof userEvent.setup>) { await user.click(screen.getByText('Déclarations de couverture et droits sur les loyers')); }
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.listFinanceAt).mockResolvedValue(finance);
  vi.mocked(api.listRentalsAt).mockResolvedValue({ leases: [], lease_terms: [], lease_term_revisions: [], dues: [due], payments: [], allocations: [], expenses: [], knowledge_sequence: 42 });
  vi.mocked(api.listDebtsAt).mockResolvedValue({ contracts: [], terms: [], movements: [], receivable_adjustments: [], schedules: [], payment_allocations: [], asset_links: [], guarantees: [], coverages: [], knowledge_sequence: 42 });
  vi.mocked(api.getFinancialReport).mockResolvedValue(report);
  vi.mocked(api.saveFinanceCoverage).mockResolvedValue({ id: 'coverage', revision_id: 'coverage-r1', valid_from: '2026-01-01', valid_to: null, assets_complete: true, debts_complete: true, rentals_complete: true, notes: '' });
  vi.mocked(api.saveRentClaim).mockResolvedValue({ id: 'claim', revision_id: 'claim-r1', due_id: 'due', parties: [{ entity_id: 'owner', economic_share: null }], valid_from: '2026-03-31', valid_to: null, represented_by_debt_id: null, notes: '' });
  vi.mocked(api.saveRentClaimAdjustment).mockResolvedValue({ id: 'adjustment', revision_id: 'adjustment-r1', due_id: 'due', creditor_entity_id: 'owner', kind: 'impairment', amount: '-100', effective_on: '2026-03-31', source: 'Justificatif', notes: '' });
});

describe('Bilan et couverture explicite', () => {
  it('garde un net incomplet distinct du sous-total connu et n’ajoute pas deux fois le loyer en banque', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Calculer le bilan financier' }));
    await screen.findByText('La couverture complète du périmètre n’est pas déclarée.');
    const totals = document.querySelector('.financial-result .rental-indicators')!;
    expect(within(totals as HTMLElement).getAllByText('Non calculable')).toHaveLength(3);
    expect(screen.getByText(/Sous-totaux connus : actifs 1000.00 EUR/)).toBeVisible();
    expect(screen.getByText('400.00 EUR')).toBeVisible();
    expect(screen.getByText('600.00 EUR')).toBeVisible();
    expect(screen.queryByText('1400.00 EUR')).not.toBeInTheDocument();
    expect(api.saveFinanceCoverage).not.toHaveBeenCalled();
    expect(api.getFinancialReport).toHaveBeenCalledExactlyOnceWith({ context: 'family', person_id: null, date: '2026-03-31', knowledge_sequence: null });
  });

  it('présente les deux faces et l’élimination interne calculées sans masquer les sources', async () => {
    vi.mocked(api.getFinancialReport).mockResolvedValue({ ...report, status: 'complete', coverage_confirmed: true, assets_eur: '0.00', liabilities_eur: '0.00', net_eur: '0.00', positions: [], eliminations: [{ object_id: 'loan', label: 'Prêt familial unique', creditor_before_eur: '10000.00', debtor_before_eur: '10000.00', eliminated_eur: '10000.00', creditor_after_eur: '0.00', debtor_after_eur: '0.00', exact_eliminated: { num: '10000', den: '1' }, source_ids: ['loan-r1', 'creditor-r1', 'debtor-r1'] }], warnings: [] });
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Calculer le bilan financier' }));
    await screen.findByText('Prêt familial unique');
    expect(screen.getByText('Couverture déclarée et sources cohérentes')).toBeVisible();
    const row = screen.getByText('Prêt familial unique').closest('tr')!;
    expect(within(row).getAllByText('10000.00 EUR')).toHaveLength(3);
    expect(within(row).getByText(/Créance 0.00 EUR ; dette 0.00 EUR/)).toBeVisible();
    expect(screen.queryByText(/Cette différence n’est pas un patrimoine net complet/)).not.toBeInTheDocument();
  });

  it('exige une déclaration délibérée de chaque inventaire, sans cocher des tables vides', async () => {
    const { user } = await open(); await declarations(user);
    await user.click(screen.getByRole('button', { name: 'Déclarer la couverture du bilan' }));
    const asset = screen.getByLabelText('Tous les actifs de l’espace familial sont inventoriés');
    const debt = screen.getByLabelText('Tous les financements et engagements certains de l’espace familial sont inventoriés');
    const rental = screen.getByLabelText('Les créances, avances et dépôts locatifs de l’espace familial sont inventoriés');
    expect(asset).not.toBeChecked(); expect(debt).not.toBeChecked(); expect(rental).not.toBeChecked();
    await user.click(asset); await user.click(debt); await user.click(rental);
    fireEvent.change(screen.getByLabelText('Début d’application de la déclaration'), { target: { value: '2026-01-01' } });
    await user.type(screen.getByLabelText('Motif de la déclaration financière'), 'Inventaires vérifiés sur la période');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la déclaration financière' }));
    await waitFor(() => expect(api.saveFinanceCoverage).toHaveBeenCalledOnce());
    expect(api.saveFinanceCoverage).toHaveBeenCalledExactlyOnceWith({ id: null, expected_revision_id: null, reason: 'Inventaires vérifiés sur la période', valid_from: '2026-01-01', valid_to: null, assets_complete: true, debts_complete: true, rentals_complete: true, notes: '' });
    expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('préserve un créancier inconnu sans déduire ses droits de l’immeuble actuel', async () => {
    const { user } = await open(); await declarations(user);
    await user.click(screen.getByRole('button', { name: 'Définir les créanciers d’une échéance' }));
    await user.selectOptions(screen.getByLabelText('Échéance porteuse de la créance'), 'due');
    await user.selectOptions(screen.getByLabelText('Créancier 1'), 'owner');
    expect(screen.getByLabelText('Quote-part économique connue du créancier 1')).not.toBeChecked();
    await user.type(screen.getByLabelText('Motif de la déclaration financière'), 'Droit documenté, quotité à compléter');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la déclaration financière' }));
    await waitFor(() => expect(api.saveRentClaim).toHaveBeenCalledOnce());
    expect(api.saveRentClaim).toHaveBeenCalledWith(expect.objectContaining({ due_id: 'due', parties: [{ entity_id: 'owner', economic_share: null }], represented_by_debt_id: null }));
    expect(api.saveOwnership).not.toHaveBeenCalled();
  });

  it('transmet des quotes-parts exactes et une dépréciation sans remise de dette', async () => {
    vi.mocked(api.listFinanceAt).mockResolvedValue({ ...finance, rent_claims: [{ id: 'claim', revision_id: 'claim-r1', due_id: 'due', parties: [{ entity_id: 'owner', economic_share: { num: '1', den: '3' } }], valid_from: '2026-03-01', valid_to: null, represented_by_debt_id: null, notes: '' }] });
    const { user } = await open(); await declarations(user);
    await user.click(screen.getByRole('button', { name: 'Définir les créanciers d’une échéance' }));
    await user.selectOptions(screen.getByLabelText('Échéance porteuse de la créance'), 'due');
    await user.selectOptions(screen.getByLabelText('Créancier 1'), 'owner');
    await user.click(screen.getByLabelText('Quote-part économique connue du créancier 1'));
    await user.type(screen.getByLabelText('Numérateur du créancier 1'), '1');
    await user.type(screen.getByLabelText('Dénominateur du créancier 1'), '3');
    await user.type(screen.getByLabelText('Motif de la déclaration financière'), 'Un tiers exact du droit');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la déclaration financière' }));
    await waitFor(() => expect(api.saveRentClaim).toHaveBeenCalledWith(expect.objectContaining({ parties: [{ entity_id: 'owner', economic_share: { num: '1', den: '3' } }] })));
    await user.click(screen.getByRole('button', { name: 'Ajuster la valeur d’une créance locative' }));
    await user.selectOptions(screen.getByLabelText('Échéance dépréciée ou reprise'), 'due');
    await user.selectOptions(screen.getByLabelText('Créancier concerné'), 'owner');
    await user.type(screen.getByLabelText('Variation signée de valeur'), '-100,125');
    await user.type(screen.getByLabelText('Justificatif ou méthode'), 'Dossier de recouvrement');
    await user.type(screen.getByLabelText('Motif de la déclaration financière'), 'Perte estimée sans remise juridique');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la déclaration financière' }));
    await waitFor(() => expect(api.saveRentClaimAdjustment).toHaveBeenCalledWith(expect.objectContaining({ kind: 'impairment', amount: '-100,125', creditor_entity_id: 'owner' })));
    expect(api.saveDebtMovement).not.toHaveBeenCalled();
  });

  it('respecte le contexte individuel et K=0 tout en interdisant les déclarations', async () => {
    const { user, rerender, props } = await open(0);
    await declarations(user);
    expect(screen.getByRole('button', { name: 'Déclarer la couverture du bilan' })).toBeDisabled();
    rerender(<FinancialPanel {...props} context={{ ...props.context, personId: 'owner' }} />);
    await user.click(screen.getByRole('button', { name: 'Calculer le bilan financier' }));
    await waitFor(() => expect(api.getFinancialReport).toHaveBeenCalledExactlyOnceWith({ context: 'person', person_id: 'owner', date: '2026-03-31', knowledge_sequence: 0 }));
    expect(api.saveFinanceCoverage).not.toHaveBeenCalled();
  });

  it('conserve le motif et les choix quand une déclaration périmée est refusée', async () => {
    vi.mocked(api.saveFinanceCoverage).mockRejectedValueOnce({ message: 'La révision de couverture a changé.' });
    const { user } = await open(); await declarations(user);
    await user.click(screen.getByRole('button', { name: 'Déclarer la couverture du bilan' }));
    await user.click(screen.getByLabelText('Tous les actifs de l’espace familial sont inventoriés'));
    await user.type(screen.getByLabelText('Motif de la déclaration financière'), 'Couverture à revérifier');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la déclaration financière' }));
    await screen.findByText('La révision de couverture a changé.');
    expect(screen.getByLabelText('Motif de la déclaration financière')).toHaveValue('Couverture à revérifier');
    expect(screen.getByLabelText('Tous les actifs de l’espace familial sont inventoriés')).toBeChecked();
    expect(screen.queryByText(/La déclaration et ses anciennes révisions sont conservées/)).not.toBeInTheDocument();
  });
});
