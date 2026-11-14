import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../services/api';
import type { CompanyDistribution, CompanyFinancialPeriod, CompanyReport, CompanySnapshot } from '../services/companies-types';
import { emptyRegistry } from '../services/registry-types';
import type { Entity, RegistrySnapshot } from '../services/registry-types';
import type { TreasurySnapshot } from '../services/treasury-types';
import CompaniesPage from './companies';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});
const company: Entity = { id: 'company', revision_id: 'company-r1', kind: 'company', display_name: 'Société documentée', first_name: null, last_name: null, birth_date: null, residence_country: 'FR', preferred_currency: 'EUR', activity: 'Activité', notes: '', archived: false };
const beneficiary: Entity = { ...company, id: 'person', revision_id: 'person-r1', kind: 'person', display_name: 'Membre bénéficiaire' };
const registry: RegistrySnapshot = { ...emptyRegistry, knowledge_sequence: 42, entities: [company, beneficiary],
  assets: [{ id: 'company-bank-asset', revision_id: 'cba-r', name: 'Compte de la société', country: 'FR', category: 'bank', notes: '', archived: false }, { id: 'personal-bank-asset', revision_id: 'pba-r', name: 'Compte personnel', country: 'FR', category: 'bank', notes: '', archived: false }],
};
const period: CompanyFinancialPeriod = { id: 'period', revision_id: 'period-r2', company_id: 'company', period_start: '2026-01-01', period_end: '2026-02-01', frequency: 'monthly', currency: 'EUR', revenue: '10000', operating_expenses: '7000', operating_result: '3000', net_result: null, indicative_cash: '999999', indicative_debt: null, important_receivables: null, dividends_declared: null, indicative_global_value: null, notes: 'Observation existante' };
const snapshot: CompanySnapshot = { financial_periods: [period], distributions: [], knowledge_sequence: 42 };
const account = { id: 'company-bank', revision_id: 'cb-r', asset_id: 'company-bank-asset', kind: 'bank', currency: 'EUR', tracking_start: '2020-01-01', legal_owner_entity_id: 'company', bank_name: 'Banque', reference: '', country: 'FR', storage_location: '', notes: '', archived: false };
const treasury: TreasurySnapshot = { accounts: [account, { ...account, id: 'personal-bank', revision_id: 'pb-r', asset_id: 'personal-bank-asset', legal_owner_entity_id: 'person' }],
  transactions: [{ id: 'journal', revision_id: 'journal-r3', kind: 'dividend', effective_on: '2026-03-01', label: 'Dividende déjà au journal', reference: 'REF', notes: 'Journal existant', entries: [{ line_id: 'payer-line', account_id: 'company-bank', amount: '-500', role: 'principal' }, { line_id: 'beneficiary-line', account_id: 'personal-bank', amount: '500', role: 'principal' }] }],
  reconciliations: [], cash_links: [], deposits: [], knowledge_sequence: 42,
};
const paid: CompanyDistribution = { id: 'paid', revision_id: 'paid-r4', company_id: 'company', beneficiary_entity_id: 'person', kind: 'paid', effective_on: '2026-03-01', amount: '500', currency: 'EUR', related_declaration_id: null, unlinked_reason: '', notes: '', cash_links: [{ side: 'payer', transaction_revision_id: 'journal-r3', line_id: 'payer-line', amount: '-500' }, { side: 'beneficiary', transaction_revision_id: 'journal-r3', line_id: 'beneficiary-line', amount: '500' }] };
const report: CompanyReport = { company_id: 'company', date: '2026-03-31', knowledge_sequence: 42, status: 'partial', company, policy: null, owners: [], subsidiaries: [], assets: [], accounts: [], debt_positions: [], financial_periods: [{ period, calculated_operating_result: '3000.00', operating_result_difference: '0.00', trends: [{ metric: 'revenue', previous_period_id: 'previous-period', difference_original: '10000.00', exact_difference: { num: '10000', den: '1' }, percentage: null, exact_percentage: null }] }], distributions: [{ distribution: { ...paid, cash_links: [] }, payer_cash_complete: false, beneficiary_cash_complete: false, paid_against_declaration: null, remaining_declared: null }], source_ids: ['period-r2'], warnings: [{ code: 'missing_dividend_cash', object_id: 'paid', message: 'Le paiement déclaré ne dispose pas de ses deux contreparties suivies.' }] };

async function open(knowledgeSequence: number | null = null) {
  const props = { registry, context: { personId: null, date: '2026-03-31', knowledgeSequence }, onChanged: vi.fn().mockResolvedValue(undefined), onBusyChange: vi.fn(), onNavigateTreasury: vi.fn() };
  const result = render(<CompaniesPage {...props} />);
  await screen.findByRole('button', { name: 'Ajouter une observation de société' });
  return { user: userEvent.setup(), props, ...result };
}
async function distributionForm(user: ReturnType<typeof userEvent.setup>, kind = 'declared') {
  await user.click(screen.getByRole('button', { name: 'Distributions réelles' }));
  await user.click(screen.getByRole('button', { name: 'Enregistrer une distribution de société' }));
  await user.selectOptions(screen.getByLabelText('Société de la source'), 'company');
  await user.selectOptions(screen.getByLabelText('Bénéficiaire du dividende'), 'person');
  await user.selectOptions(screen.getByLabelText('Nature de la distribution'), kind);
  await user.type(screen.getByLabelText('Montant positif de la distribution'), '500');
  await user.type(screen.getByLabelText('Motif de l’enregistrement de société'), 'Distribution documentée');
}
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.listCompaniesAt).mockResolvedValue(snapshot);
  vi.mocked(api.listTreasuryAt).mockResolvedValue(treasury);
  vi.mocked(api.saveCompanyFinancialPeriod).mockResolvedValue(period);
  vi.mocked(api.saveCompanyDistribution).mockResolvedValue(paid);
  vi.mocked(api.postCompanyDistribution).mockResolvedValue(paid);
  vi.mocked(api.getCompanyReport).mockResolvedValue(report);
});

describe('Observations et distributions de sociétés', () => {
  it('conserve les décimaux textuels et distingue un zéro déclaré d’un indicateur inconnu', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Ajouter une observation de société' }));
    await user.selectOptions(screen.getByLabelText('Société de la source'), 'company');
    await user.selectOptions(screen.getByLabelText('Fréquence de la période'), 'quarterly');
    fireEvent.change(screen.getByLabelText('Début de période'), { target: { value: '2026-01-01' } });
    fireEvent.change(screen.getByLabelText('Fin de période exclue'), { target: { value: '2026-04-01' } });
    await user.type(screen.getByLabelText('Chiffre d’affaires'), '123456789012345,6789');
    await user.type(screen.getByLabelText('Charges d’exploitation'), '0');
    await user.type(screen.getByLabelText('Résultat net'), '-12,34');
    await user.type(screen.getByLabelText('Motif de l’enregistrement de société'), 'Période vérifiée, autres données inconnues');
    await user.click(screen.getByRole('button', { name: 'Enregistrer l’observation de société' }));
    await waitFor(() => expect(api.saveCompanyFinancialPeriod).toHaveBeenCalledOnce());
    expect(api.saveCompanyFinancialPeriod).toHaveBeenCalledWith(expect.objectContaining({ company_id: 'company', frequency: 'quarterly', period_start: '2026-01-01', period_end: '2026-04-01', revenue: '123456789012345,6789', operating_expenses: '0', operating_result: null, net_result: '-12,34', indicative_cash: null }));
    expect(api.saveValuation).not.toHaveBeenCalled(); expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('enregistre une déclaration de dividende sans fabriquer un encaissement ni journal', async () => {
    const { user } = await open(); await distributionForm(user);
    expect(screen.queryByLabelText('Traitement de la trésorerie du dividende')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Enregistrer la distribution de société' }));
    await waitFor(() => expect(api.saveCompanyDistribution).toHaveBeenCalledOnce());
    expect(api.saveCompanyDistribution).toHaveBeenCalledWith(expect.objectContaining({ kind: 'declared', beneficiary_entity_id: 'person', amount: '500', cash_links: [], related_declaration_id: null }));
    expect(api.postCompanyDistribution).not.toHaveBeenCalled(); expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('crée le dividende payé et les deux lignes de trésorerie dans une seule commande', async () => {
    const { user } = await open(); await distributionForm(user, 'paid');
    await user.selectOptions(screen.getByLabelText('Traitement de la trésorerie du dividende'), 'new');
    await user.selectOptions(screen.getByLabelText('Compte débité de la société (si suivi)'), 'company-bank');
    await user.selectOptions(screen.getByLabelText('Compte crédité du bénéficiaire (si suivi)'), 'personal-bank');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la distribution de société' }));
    await waitFor(() => expect(api.postCompanyDistribution).toHaveBeenCalledOnce());
    const input = vi.mocked(api.postCompanyDistribution).mock.calls[0][0];
    expect(input.expected_sequence).toBe(42);
    expect(input.transaction).toMatchObject({ id: null, kind: 'dividend', entries: [{ line_id: 'dividend-payer', account_id: 'company-bank', amount: '-500', role: 'principal' }, { line_id: 'dividend-beneficiary', account_id: 'personal-bank', amount: '500', role: 'principal' }] });
    expect(input.links).toEqual([{ side: 'payer', line_id: 'dividend-payer', amount: '-500' }, { side: 'beneficiary', line_id: 'dividend-beneficiary', amount: '500' }]);
    expect(input.distribution.cash_links).toEqual([]);
    expect(api.saveCompanyDistribution).not.toHaveBeenCalled(); expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('révise le journal d’origine et les deux liens avec le paiement corrigé, sans seconde entrée', async () => {
    vi.mocked(api.listCompaniesAt).mockResolvedValue({ ...snapshot, distributions: [paid] });
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Distributions réelles' }));
    await user.click(screen.getByRole('button', { name: 'Modifier' }));
    expect(screen.getByLabelText('Société de la source')).toBeDisabled();
    await user.clear(screen.getByLabelText('Montant positif de la distribution'));
    await user.type(screen.getByLabelText('Montant positif de la distribution'), '600');
    await user.selectOptions(screen.getByLabelText('Traitement de la trésorerie du dividende'), 'correct');
    for (const [label, value] of [['Montant total de la ligne 1 (Compte de la société · EUR)', '-600'], ['Montant total de la ligne 2 (Compte personnel · EUR)', '600'], ['Montant bancaire signé du lien 1', '-600'], ['Montant bancaire signé du lien 2', '600']]) {
      await user.clear(screen.getByLabelText(label)); await user.type(screen.getByLabelText(label), value);
    }
    await user.type(screen.getByLabelText('Motif de l’enregistrement de société'), 'Montant corrigé avec la banque');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la distribution de société' }));
    await waitFor(() => expect(api.postCompanyDistribution).toHaveBeenCalledOnce());
    const input = vi.mocked(api.postCompanyDistribution).mock.calls[0][0];
    expect(input.distribution).toMatchObject({ id: 'paid', expected_revision_id: 'paid-r4', amount: '600', cash_links: [] });
    expect(input.transaction).toMatchObject({ id: 'journal', expected_revision_id: 'journal-r3', reference: 'REF' });
    expect(input.transaction?.entries.map(e => e.amount)).toEqual(['-600', '600']);
    expect(input.links.map(e => e.amount)).toEqual(['-600', '600']);
    expect(input.existing_transaction_revision_id).toBeNull();
  });

  it('exige un motif pour une source historique payée non rapprochée', async () => {
    const { user } = await open(); await distributionForm(user, 'paid');
    await user.selectOptions(screen.getByLabelText('Traitement de la trésorerie du dividende'), 'unlinked');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la distribution de société' }));
    expect(api.saveCompanyDistribution).not.toHaveBeenCalled();
    await user.type(screen.getByLabelText('Explication des côtés bancaires non rapprochés'), 'Banque historique non suivie, preuve conservée');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la distribution de société' }));
    await waitFor(() => expect(api.saveCompanyDistribution).toHaveBeenCalledOnce());
    expect(api.saveCompanyDistribution).toHaveBeenCalledWith(expect.objectContaining({ kind: 'paid', unlinked_reason: 'Banque historique non suivie, preuve conservée', cash_links: [] }));
  });

  it('présente les contrôles Rust et une tendance non calculable sans additionner les indicateurs', async () => {
    const { user } = await open();
    await user.selectOptions(screen.getByLabelText('Société consultée'), 'company');
    await user.click(screen.getByRole('button', { name: 'Consulter une société' }));
    await user.selectOptions(screen.getByLabelText('Fréquence des observations comparées'), 'monthly');
    await user.click(screen.getByRole('button', { name: 'Calculer le contrôle de société' }));
    await screen.findByText('Le paiement déclaré ne dispose pas de ses deux contreparties suivies.');
    expect(api.getCompanyReport).toHaveBeenCalledExactlyOnceWith({ company_id: 'company', date: '2026-03-31', frequency: 'monthly', knowledge_sequence: null });
    expect(screen.getByText(/variation en pourcentage non calculable/)).toBeVisible();
    expect(screen.getByText(/Trésorerie indicative : 999999 EUR/)).toBeVisible();
    const sources = screen.getByText(/Société : incomplète/).closest('tr')!;
    expect(within(sources).getByText(/Bénéficiaire : incomplet/)).toBeVisible();
    expect(api.getFinancialReport).not.toHaveBeenCalled(); expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('respecte K=0 en lecture seule et masque le rapport lorsque la date change', async () => {
    const { user, props, rerender } = await open(0);
    expect(api.listCompaniesAt).toHaveBeenCalledWith(0);
    expect(api.listTreasuryAt).toHaveBeenCalledWith(0);
    expect(screen.getByRole('button', { name: 'Ajouter une observation de société' })).toBeDisabled();
    await user.selectOptions(screen.getByLabelText('Société consultée'), 'company');
    await user.click(screen.getByRole('button', { name: 'Consulter une société' }));
    await user.click(screen.getByRole('button', { name: 'Calculer le contrôle de société' }));
    await screen.findByText('Le paiement déclaré ne dispose pas de ses deux contreparties suivies.');
    expect(api.getCompanyReport).toHaveBeenCalledWith({ company_id: 'company', date: '2026-03-31', frequency: null, knowledge_sequence: 0 });
    rerender(<CompaniesPage {...props} context={{ ...props.context, date: '2026-04-30' }} />);
    await waitFor(() => expect(screen.queryByText('Le paiement déclaré ne dispose pas de ses deux contreparties suivies.')).not.toBeInTheDocument());
    expect(api.saveCompanyFinancialPeriod).not.toHaveBeenCalled();
  });

  it('ne conserve pas les sources courantes après l’échec de chargement d’un ancien K', async () => {
    const { props, rerender } = await open();
    expect(screen.getByText('Observation existante')).toBeInTheDocument();
    vi.mocked(api.listCompaniesAt).mockRejectedValueOnce({ message: 'Cette connaissance est inaccessible.' });
    rerender(<CompaniesPage {...props} context={{ ...props.context, knowledgeSequence: 0 }} />);
    await screen.findByText('Cette connaissance est inaccessible.');
    expect(screen.queryByText('Observation existante')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Ajouter une observation de société' })).not.toBeInTheDocument();
  });

  it('garde la saisie quand la révision de la période a changé', async () => {
    vi.mocked(api.saveCompanyFinancialPeriod).mockRejectedValueOnce({ message: 'La révision de la période a changé.' });
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Modifier' }));
    await user.clear(screen.getByLabelText('Résultat net')); await user.type(screen.getByLabelText('Résultat net'), '-20,50');
    await user.type(screen.getByLabelText('Motif de l’enregistrement de société'), 'Source revue à conserver');
    await user.click(screen.getByRole('button', { name: 'Enregistrer l’observation de société' }));
    await screen.findByText('La révision de la période a changé.');
    expect(screen.getByLabelText('Résultat net')).toHaveValue('-20,50');
    expect(screen.getByLabelText('Motif de l’enregistrement de société')).toHaveValue('Source revue à conserver');
    expect(api.saveCompanyFinancialPeriod).toHaveBeenCalledWith(expect.objectContaining({ id: 'period', expected_revision_id: 'period-r2' }));
  });
});
