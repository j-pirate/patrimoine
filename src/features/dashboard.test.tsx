import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../services/api';
import type { VaultStatus } from '../services/api';
import type { DashboardReport, DashboardRequest, DashboardTotals } from '../services/dashboard-types';
import type { CompanyReport } from '../services/companies-types';
import type { FinancialReport } from '../services/finance-types';
import { emptyRegistry } from '../services/registry-types';
import type { Entity, RegistrySnapshot } from '../services/registry-types';
import DashboardPage from './dashboard';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});
vi.mock('./dashboard-charts', () => ({
  DashboardHistoryChart: () => <section aria-label="Graphique historique" />,
  DashboardAllocationChart: ({ title }: { title: string }) => <section aria-label={title} />,
  DashboardMembersChart: () => <section aria-label="Graphique des membres" />,
  DashboardCompanyChart: () => <section aria-label="Graphique des observations" />,
}));

const person: Entity = { id: 'alice', revision_id: 'alice-r1', kind: 'person', display_name: 'Alice', first_name: 'Alice', last_name: null, birth_date: null, residence_country: 'FR', preferred_currency: 'EUR', activity: '', notes: '', archived: false };
const company: Entity = { ...person, id: 'company', revision_id: 'company-r1', kind: 'company', display_name: 'Société suivie', first_name: null };
const registry: RegistrySnapshot = { ...emptyRegistry, knowledge_sequence: 42, entities: [person, company] };
const status: VaultStatus = { generation_id: 'generation-a', data_dir: '/test/vault', schema_version: 6, app_version: '0.1.0', mutation_sequence: 42, document_count: 0, country_count: 4, currency_count: 5, settings: { workspace_name: 'Famille test', display_currency: 'CHF' } };
const totals: DashboardTotals = { known_assets_eur: '1000.00', known_liabilities_eur: '0.00', known_net_eur: '1000.00', exact_known_assets: { num: '1000', den: '1' }, exact_known_liabilities: { num: '0', den: '1' }, exact_known_net: { num: '1000', den: '1' }, assets_eur: null, liabilities_eur: null, net_eur: null, position_count: 1, unknown_count: 0 };
const financial: FinancialReport = { context: 'family', person_id: null, date: '2026-03-31', knowledge_sequence: 42, status: 'partial', coverage_confirmed: false, assets_eur: null, liabilities_eur: null, net_eur: null, exact_assets: null, exact_liabilities: null, exact_net: null, known_assets_eur: '1000.00', known_liabilities_eur: '0.00', known_net_eur: '1000.00', positions: [], eliminations: [], source_ids: ['bank-r1'], warnings: [] };
const request: DashboardRequest = { context: 'family', entity_id: null, date: '2026-03-31', from_date: '2026-01-01', interval: 'monthly', knowledge_sequence: 42, filters: { country: null, currency: null, category: null, owner_entity_id: null, query: '' } };
const position = { position: { object_id: 'bank', label: 'Banque réelle', kind: 'treasury', entity_id: 'alice', country: 'FR', currency: 'EUR', amount_original: '1000', exact_original: { num: '1000', den: '1' }, weight: { num: '1', den: '1' }, amount_eur: '1200.00', exact_eur: { num: '1200', den: '1' }, source_ids: ['bank-r1'], conversion: null }, side: 'asset' as const, category: 'banque', country: 'FR', owner_entity_ids: ['alice'], amount_eur: '1000.00', exact_eur: { num: '1000', den: '1' }, eliminated_eur: '200.00', exact_eliminated: { num: '200', den: '1' } };
const base: DashboardReport = { request, knowledge_sequence: 42, engine_version: 'dashboard-test-1', financial, company: null, rental: null, totals, positions: [position], by_category: [], by_country: [], by_currency: [], members: [], history: [], period_change: { from_date: request.from_date, to_date: request.date, known_net_change_eur: '200.00', exact_known_net_change: { num: '200', den: '1' }, net_change_eur: null }, variation: null, warnings: [{ code: 'coverage_unconfirmed', object_id: 'scope', message: 'L’inventaire complet n’est pas confirmé.' }], source_ids: ['bank-r1', 'scope-r1'], filters_applied: false };
const props = () => ({ registry, context: { personId: null as string | null, date: '2026-03-31', knowledgeSequence: null as number | null }, status, onNavigate: vi.fn() });
function answer(input: DashboardRequest, changes: Partial<DashboardReport> = {}): DashboardReport {
  return { ...base, request: input, knowledge_sequence: input.knowledge_sequence ?? 42, ...changes };
}
async function open() {
  const p = props();
  const view = render(<DashboardPage {...p} />);
  await screen.findByRole('heading', { name: 'Votre situation patrimoniale' });
  return { ...view, p, user: userEvent.setup() };
}
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(done => { resolve = done; });
  return { promise, resolve };
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.getDashboard).mockImplementation(async input => answer(input));
  vi.mocked(api.chooseDashboardReportDestination).mockResolvedValue(null);
  vi.mocked(api.exportDashboardReport).mockResolvedValue(undefined);
});

describe('Tableau de bord sourcé', () => {
  it('conserve un net inconnu et les montants après élimination, sans les recalculer depuis les lignes', async () => {
    const { user } = await open();
    const net = screen.getByRole('article', { name: 'Patrimoine net' });
    expect(within(net).getByText('Non calculable')).toBeVisible();
    expect(within(net).getByText(/1\s000,00 EUR/)).toBeVisible();
    expect(net.querySelector('strong')).toHaveAttribute('data-value', 'unknown');
    expect(screen.getByText('Banque réelle')).toBeVisible();
    expect(screen.getByText(/Ce sous-total n’est pas un net complet/)).toBeVisible();
    expect(screen.getByText(/Cet écart ne mesure pas une performance/)).toBeVisible();
    await user.click(screen.getByText('Voir les références'));
    expect(screen.getByText(/Avant élimination : 1\s200,00 EUR/)).toBeVisible();
    expect(screen.getByText(/Réduction interne : 200,00 EUR/)).toBeVisible();
    await user.click(screen.getByText('Afficher toutes les alertes et leurs références'));
    expect(screen.getByText('L’inventaire complet n’est pas confirmé.')).toBeVisible();
    expect(api.getDashboard).toHaveBeenCalledExactlyOnceWith(request);
  });

  it('affiche un zéro complet comme zéro et conserve les engagements conditionnels hors total', async () => {
    vi.mocked(api.getDashboard).mockImplementation(async input => answer(input, {
      totals: { ...totals, assets_eur: '0.00', liabilities_eur: '0.00', net_eur: '0.00', known_assets_eur: '0.00', known_net_eur: '0.00' },
      positions: [{ ...position, side: 'contingent', amount_eur: '999999.00', position: { ...position.position, label: 'Garantie éventuelle', kind: 'guarantee' } }], warnings: [],
    }));
    await open();
    const net = screen.getByRole('article', { name: 'Patrimoine net' });
    expect(within(net).getByText('0,00 EUR')).toBeVisible();
    expect(within(net).queryByText('Non calculable')).not.toBeInTheDocument();
    expect(screen.getByText('Garantie éventuelle')).toBeVisible();
    expect(screen.getByText('Non ajouté aux totaux')).toBeVisible();
    expect(net).not.toHaveTextContent('999');
  });

  it('écarte une ancienne réponse lors d’un changement de membre et conserve K=0', async () => {
    const old = deferred<DashboardReport>();
    vi.mocked(api.getDashboard).mockReturnValueOnce(old.promise).mockImplementation(async input => answer(input, { positions: [{ ...position, position: { ...position.position, label: 'Source du nouveau contexte' } }] }));
    const p = props();
    const view = render(<DashboardPage {...p} />);
    await waitFor(() => expect(api.getDashboard).toHaveBeenCalledOnce());
    view.rerender(<DashboardPage {...p} context={{ ...p.context, personId: 'alice', knowledgeSequence: 0 }} />);
    await screen.findByText('Source du nouveau contexte');
    expect(api.getDashboard).toHaveBeenLastCalledWith({ ...request, context: 'person', entity_id: 'alice', knowledge_sequence: 0 });
    await act(async () => old.resolve({ ...base, positions: [{ ...position, position: { ...position.position, label: 'Ancienne source interdite' } }] }));
    expect(screen.queryByText('Ancienne source interdite')).not.toBeInTheDocument();
    expect(screen.getByText('Source du nouveau contexte')).toBeVisible();
  });

  it('masque les résultats dès modification des filtres et transmet pays, propriétaire et recherche', async () => {
    const { user } = await open();
    await user.selectOptions(screen.getByLabelText('Pays'), 'CH');
    expect(screen.queryByText('Banque réelle')).not.toBeInTheDocument();
    expect(api.getDashboard).toHaveBeenCalledOnce();
    await user.selectOptions(screen.getByLabelText('Propriétaire associé (positions entières)'), 'alice');
    await user.type(screen.getByLabelText('Rechercher une position'), 'référence exacte');
    await user.selectOptions(screen.getByLabelText('Type de position'), 'banque');
    await user.click(screen.getByRole('button', { name: 'Actualiser le tableau de bord' }));
    await screen.findByText('Banque réelle');
    expect(api.getDashboard).toHaveBeenLastCalledWith({ ...request, filters: { country: 'CH', currency: null, category: 'banque', owner_entity_id: 'alice', query: 'référence exacte' } });
    expect(screen.getByText(/Le propriétaire associé sélectionne des positions entières/)).toBeVisible();
  });

  it('recalcule la période et le pas avec la date économique courante sans courbe fictive', async () => {
    await open();
    fireEvent.change(screen.getByLabelText('Début de la période'), { target: { value: '2024-02-29' } });
    await waitFor(() => expect(api.getDashboard).toHaveBeenLastCalledWith({ ...request, from_date: '2024-02-29' }));
    fireEvent.change(screen.getByLabelText('Pas de l’historique'), { target: { value: 'annual' } });
    await waitFor(() => expect(api.getDashboard).toHaveBeenLastCalledWith({ ...request, from_date: '2024-02-29', interval: 'annual' }));
    expect(screen.getByText(/Du 2024-02-29 au 2026-03-31/)).toBeVisible();
  });

  it('présente la société sans fabriquer un net et désactive les filtres de positions', async () => {
    const companyReport: CompanyReport = { company_id: 'company', date: request.date, knowledge_sequence: 42, status: 'partial', company, policy: null, owners: [], subsidiaries: [], assets: [], accounts: [], debt_positions: [], financial_periods: [], distributions: [], source_ids: ['company-r1'], warnings: [] };
    vi.mocked(api.getDashboard).mockImplementation(async input => input.context === 'company' ? answer(input, { financial: null, totals: null, company: companyReport, positions: [], warnings: [] }) : answer(input));
    const { user, p, rerender } = await open();
    await user.selectOptions(screen.getByLabelText('Pays'), 'CH');
    await user.selectOptions(screen.getByLabelText('Vue de société'), 'company');
    await screen.findByRole('heading', { name: 'Pilotage de la société' });
    expect(api.getDashboard).toHaveBeenLastCalledWith({ ...request, context: 'company', entity_id: 'company' });
    expect(screen.getByLabelText('Pays')).toBeDisabled();
    expect(screen.getByLabelText('Pays')).toHaveValue('');
    expect(screen.queryByRole('article', { name: 'Patrimoine net' })).not.toBeInTheDocument();
    expect(screen.getByText(/aucun patrimoine net de société n’est déduit/)).toBeVisible();
    expect(screen.getByText(/Cette absence ne confirme pas un inventaire complet/)).toBeVisible();
    rerender(<DashboardPage {...p} context={{ ...p.context, personId: 'alice' }} />);
    await screen.findByRole('heading', { name: 'Votre situation patrimoniale' });
    expect(screen.getByLabelText('Vue de société')).toHaveValue('');
    expect(screen.getByLabelText('Pays')).toBeEnabled();
  });

  it('annule le dialogue sans exporter puis exporte le contexte exact connu du rapport', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Exporter le rapport filtré' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Exporter le rapport filtré' })).toBeEnabled());
    expect(api.exportDashboardReport).not.toHaveBeenCalled();
    expect(screen.queryByText(/Le rapport filtré a été exporté/)).not.toBeInTheDocument();
    vi.mocked(api.chooseDashboardReportDestination).mockResolvedValue('/test/rapport.json');
    await user.click(screen.getByRole('button', { name: 'Exporter le rapport filtré' }));
    await screen.findByText(/Le rapport filtré a été exporté en JSON/);
    expect(api.exportDashboardReport).toHaveBeenCalledExactlyOnceWith(request, '/test/rapport.json');
    expect(screen.getByText(/Pour récupérer l’intégralité du coffre et ses originaux/)).toBeVisible();
  });

  it('refuse d’exporter un rapport devenu périmé pendant le choix du fichier', async () => {
    const destination = deferred<string | null>();
    vi.mocked(api.chooseDashboardReportDestination).mockReturnValue(destination.promise);
    const { user, p, rerender } = await open();
    await user.click(screen.getByRole('button', { name: 'Exporter le rapport filtré' }));
    rerender(<DashboardPage {...p} context={{ ...p.context, personId: 'alice' }} />);
    await screen.findByRole('heading', { name: 'Votre situation patrimoniale' });
    await act(async () => destination.resolve('/test/ancien-rapport.json'));
    expect(api.exportDashboardReport).not.toHaveBeenCalled();
    expect(screen.queryByText(/Le rapport filtré a été exporté/)).not.toBeInTheDocument();
  });

  it('affiche les erreurs de calcul et d’export sans conserver une ancienne réussite', async () => {
    const { user } = await open();
    vi.mocked(api.chooseDashboardReportDestination).mockResolvedValue('/test/rapport.json');
    vi.mocked(api.exportDashboardReport).mockRejectedValue({ code: 'exists', message: 'Le fichier existe déjà.' });
    await user.click(screen.getByRole('button', { name: 'Exporter le rapport filtré' }));
    await screen.findByRole('alert');
    expect(screen.getByText('Le fichier existe déjà.')).toBeVisible();
    expect(screen.queryByText(/Le rapport filtré a été exporté/)).not.toBeInTheDocument();
    vi.mocked(api.getDashboard).mockRejectedValue({ code: 'validation', message: 'La période dépasse la capacité autorisée.' });
    await user.click(screen.getByRole('button', { name: 'Actualiser le tableau de bord' }));
    await screen.findByText('La période dépasse la capacité autorisée.');
    expect(screen.queryByText('Banque réelle')).not.toBeInTheDocument();
    expect(screen.queryByText('Le fichier existe déjà.')).not.toBeInTheDocument();
  });

  it('distingue un coffre neuf d’une couverture financière confirmée et fournit les premiers accès', async () => {
    const p = props();
    render(<DashboardPage {...p} registry={emptyRegistry} />);
    await screen.findByRole('heading', { name: 'Construire une vue à partir de vos données' });
    expect(screen.getByText(/aucun montant de démonstration/)).toBeVisible();
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: 'Renseigner la famille' }));
    expect(p.onNavigate).toHaveBeenCalledExactlyOnceWith('family');
  });
});
