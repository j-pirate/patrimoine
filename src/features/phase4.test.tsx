import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ExchangeRatesPage from './exchange-rates';
import HistoryPage from './history';
import { api } from '../services/api';
import { emptyRegistry } from '../services/registry-types';
import type { ConsolidationReport } from '../services/registry-types';
import type { ExchangeRate, FxResult, RegistryHistory, VariationReport } from '../services/phase4-types';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});

const rate: ExchangeRate = {
  id: 'rate-tnd', revision_id: 'rate-r1', base_currency: 'TND', quote_currency: 'EUR', rate: '0.30',
  effective_on: '2026-10-01', source: 'Relevé local', notes: '',
};
const converted: FxResult = {
  amount_original: '10', from_currency: 'TND', to_currency: 'EUR', date: '2026-11-04', knowledge_sequence: 8,
  status: 'complete', method: 'direct', exact_rate: { num: '3', den: '10' },
  exact_converted: { num: '3', den: '1' }, amount_converted: '3.00',
  legs: [{ rate_id: rate.id, revision_id: rate.revision_id, base_currency: 'TND', quote_currency: 'EUR', rate: '0.30',
    from_currency: 'TND', to_currency: 'EUR', effective_on: rate.effective_on, source: rate.source,
    inverted: false, exact_rate: { num: '3', den: '10' } }],
  oldest_rate_date: '2026-10-01', age_days: 34,
  warnings: [{ code: 'old_rate', message: 'Le cours retenu a plus de 30 jours.' }],
};
const history: RegistryHistory = {
  latest_sequence: 8,
  events: [{ sequence: 8, recorded_at: '2026-11-04T12:00:00Z', object_id: rate.id,
    revision_id: rate.revision_id, kind: 'exchange_rate', label: 'TND/EUR', reason: 'Cours corrigé', is_void: false }],
};

function openRates(knowledgeSequence: number | null = null, rates: ExchangeRate[] = []) {
  const onChanged = vi.fn().mockResolvedValue(undefined);
  render(<ExchangeRatesPage registry={{ ...emptyRegistry, exchange_rates: rates, knowledge_sequence: knowledgeSequence ?? 8 }}
    context={{ personId: null, date: '2026-11-04', knowledgeSequence }} onChanged={onChanged} onBusyChange={vi.fn()} />);
  return { user: userEvent.setup(), onChanged };
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.saveExchangeRate).mockResolvedValue(rate);
  vi.mocked(api.convertCurrency).mockResolvedValue(converted);
});

describe('Taux manuels et conversions locales', () => {
  it('transmet le taux décimal saisi et sa source sans calcul dans l’interface', async () => {
    const { user, onChanged } = openRates();
    await user.click(screen.getByRole('button', { name: 'Ajouter un taux' }));
    await user.type(screen.getByLabelText('Taux manuel'), '0,1234567890123456789');
    await user.selectOptions(screen.getByLabelText('Devise de départ'), 'CHF');
    await user.selectOptions(screen.getByLabelText('Devise d’arrivée'), 'AED');
    fireEvent.change(screen.getByLabelText('Date d’application'), { target: { value: '2026-09-30' } });
    await user.type(screen.getByLabelText('Source (facultative)'), '  Cours de mon établissement  ');
    await user.type(screen.getByLabelText('Motif de l’enregistrement ou de la correction'), 'Observation originale');
    await user.click(screen.getByRole('button', { name: 'Enregistrer le taux' }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.saveExchangeRate).toHaveBeenCalledExactlyOnceWith({
      id: null, expected_revision_id: null, reason: 'Observation originale', base_currency: 'CHF', quote_currency: 'AED',
      rate: '0,1234567890123456789', effective_on: '2026-09-30', source: 'Cours de mon établissement', notes: '',
    });
    expect(await screen.findByText(/Le taux a été enregistré/)).toBeVisible();
  });

  it('conserve une saisie rejetée et n’annonce aucune réussite', async () => {
    vi.mocked(api.saveExchangeRate).mockRejectedValue({ code: 'invalid_rate', message: 'Le taux doit être strictement positif.' });
    const { user, onChanged } = openRates();
    await user.click(screen.getByRole('button', { name: 'Ajouter un taux' }));
    await user.type(screen.getByLabelText('Taux manuel'), '-0,5');
    await user.type(screen.getByLabelText('Motif de l’enregistrement ou de la correction'), 'Erreur à vérifier');
    await user.click(screen.getByRole('button', { name: 'Enregistrer le taux' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Le taux doit être strictement positif.');
    expect(screen.getByLabelText('Taux manuel')).toHaveValue('-0,5');
    expect(screen.queryByText(/Le taux a été enregistré/)).not.toBeInTheDocument();
    expect(onChanged).not.toHaveBeenCalled();
  });

  it('protège une correction par sa révision et conserve le formulaire en cas de conflit', async () => {
    vi.mocked(api.saveExchangeRate).mockRejectedValue({ code: 'conflict', message: 'Une révision plus récente existe.' });
    const { user, onChanged } = openRates(null, [rate]);
    await user.click(screen.getByRole('button', { name: 'Modifier le taux TND/EUR du 2026-10-01' }));
    await user.clear(screen.getByLabelText('Taux manuel'));
    await user.type(screen.getByLabelText('Taux manuel'), '0.31');
    await user.type(screen.getByLabelText('Motif de l’enregistrement ou de la correction'), 'Correction du relevé');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la correction' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Une révision plus récente existe.');
    expect(api.saveExchangeRate).toHaveBeenCalledWith(expect.objectContaining({
      id: rate.id, expected_revision_id: rate.revision_id, rate: '0.31', reason: 'Correction du relevé',
    }));
    expect(screen.getByLabelText('Taux manuel')).toHaveValue('0.31');
    expect(screen.getByRole('heading', { name: 'Corriger un taux manuel' })).toBeVisible();
    expect(onChanged).not.toHaveBeenCalled();
  });

  it('affiche le résultat Rust avec la date, la source, la révision et l’ancienneté des cours', async () => {
    const { user } = openRates();
    await user.type(screen.getByLabelText('Montant à convertir'), '10');
    await user.click(screen.getByRole('button', { name: 'Vérifier la conversion' }));
    expect(await screen.findByText('3.00 EUR')).toBeVisible();
    expect(api.convertCurrency).toHaveBeenCalledExactlyOnceWith({
      amount: '10', from_currency: 'TND', to_currency: 'EUR', date: '2026-11-04', knowledge_sequence: null,
    });
    expect(screen.getByText(/ancienneté 34 jour/)).toBeVisible();
    expect(screen.getByText(/Source : Relevé local · révision rate-r1/)).toBeVisible();
    expect(screen.getByText('Le cours retenu a plus de 30 jours.')).toBeVisible();
  });

  it.each([
    { amount: '0.00', label: '0.00 EUR', absent: 'Conversion impossible' },
    { amount: null, label: 'Conversion impossible', absent: '0.00 EUR' },
  ])('distingue le résultat de conversion $label d’une valeur inconnue', async ({ amount, label, absent }) => {
    vi.mocked(api.convertCurrency).mockResolvedValue({ ...converted, amount_original: '0', amount_converted: amount,
      status: amount === null ? 'non_calculable' : 'complete', exact_converted: amount === null ? null : { num: '0', den: '1' } });
    const { user } = openRates();
    await user.type(screen.getByLabelText('Montant à convertir'), '0');
    await user.click(screen.getByRole('button', { name: 'Vérifier la conversion' }));
    expect(await screen.findByText(label)).toBeVisible();
    expect(screen.queryByText(absent)).not.toBeInTheDocument();
  });

  it('interdit les écritures à K=0 et permet une conversion à cette même séquence', async () => {
    const { user } = openRates(0, [rate]);
    expect(screen.getByText('Lecture seule historique.')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Ajouter un taux' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Modifier le taux TND/EUR du 2026-10-01' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Annuler le taux' })).toBeDisabled();
    await user.type(screen.getByLabelText('Montant à convertir'), '1,00000000000000001');
    await user.click(screen.getByRole('button', { name: 'Vérifier la conversion' }));
    await screen.findByText('3.00 EUR');
    expect(api.convertCurrency).toHaveBeenCalledWith(expect.objectContaining({ amount: '1,00000000000000001', knowledge_sequence: 0 }));
    expect(api.saveExchangeRate).not.toHaveBeenCalled();
    expect(api.voidRegistryRecord).not.toHaveBeenCalled();
  });
});

describe('Journal et explication des variations', () => {
  it('permet de consulter un commit puis de revenir aux connaissances actuelles sans restauration', async () => {
    const user = userEvent.setup();
    const onChooseSequence = vi.fn();
    const props = { history, historyError: '', onChooseSequence, onReload: vi.fn().mockResolvedValue(undefined), onBusyChange: vi.fn() };
    const view = render(<HistoryPage {...props} context={{ personId: null, date: '2026-11-04', knowledgeSequence: null }} />);
    expect(screen.getByText('Cours corrigé')).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Consulter la séquence 8' }));
    expect(onChooseSequence).toHaveBeenLastCalledWith(8);
    view.rerender(<HistoryPage {...props} context={{ personId: null, date: '2026-11-04', knowledgeSequence: 8 }} />);
    await user.click(screen.getByRole('button', { name: 'Revenir aux connaissances actuelles' }));
    expect(onChooseSequence).toHaveBeenLastCalledWith(null);
    expect(api.activateRestore).not.toHaveBeenCalled();
  });

  it('conserve les effets et résidus inconnus et décrit une variation de sous-totaux sans rendement', async () => {
    const unknown: ConsolidationReport = {
      context: 'person', person_id: 'alice', date: '2026-01-01', knowledge_sequence: 8, engine_version: 'test', status: 'non_calculable',
      gross_eur: '0.00', debt_eur: '0.00', net_eur: '0.00', exact_gross: { num: '0', den: '1' },
      exact_debt: { num: '0', den: '1' }, exact_net: { num: '0', den: '1' }, global_equity_eur: '0.00',
      gross_is_underlying_complete: false, contributions: [], warnings: [],
    };
    const variation: VariationReport = {
      from_date: '2026-01-01', to_date: '2026-11-04', knowledge_sequence: 8,
      opening: unknown, closing: { ...unknown, date: '2026-11-04', status: 'partial', gross_eur: '100.00' },
      delta_eur: '100.00', exact_delta: { num: '100', den: '1' }, explained_eur: '0.00', exact_explained: { num: '0', den: '1' },
      residual_eur: null, exact_residual: null, status: 'partial',
      steps: [{ code: 'valuation', label: 'Effet des estimations', amount_eur: null, exact_amount: null,
        before_eur: '0.00', after_eur: '100.00', dates: { membership_date: '2026-01-01', method_date: '2026-01-01',
          rights_date: '2026-01-01', valuation_date: '2026-11-04', fx_date: '2026-01-01' }, source_ids: ['valuation-r2'], warnings: [] }],
      warnings: [{ code: 'missing_reference', object_id: 'asset', message: 'La valeur de référence est inconnue.' }],
    };
    vi.mocked(api.explainVariation).mockResolvedValue(variation);
    const user = userEvent.setup();
    render(<HistoryPage history={history} historyError="" context={{ personId: 'alice', date: '2026-11-04', knowledgeSequence: 0 }}
      onChooseSequence={vi.fn()} onReload={vi.fn().mockResolvedValue(undefined)} onBusyChange={vi.fn()} />);
    await user.click(screen.getByRole('button', { name: 'Expliquer la variation' }));
    expect(await screen.findByText('Non attribuable')).toBeVisible();
    expect(api.explainVariation).toHaveBeenCalledExactlyOnceWith({ context: 'person', person_id: 'alice',
      from_date: '2026-01-01', to_date: '2026-11-04', knowledge_sequence: 0 });
    expect(screen.getByText('Sous-total connu initial').parentElement).toHaveTextContent('Non calculable');
    expect(screen.getByText('Résidu non expliqué : Non calculable')).toBeVisible();
    const row = screen.getByText('Effet des estimations').closest('tr')!;
    expect(within(row).queryByText('0.00 EUR')).not.toBeInTheDocument();
    expect(screen.getByText(/Ce contrôle n’est ni un rendement d’investissement/)).toBeVisible();
    expect(screen.getByText('La valeur de référence est inconnue.')).toBeVisible();
  });
});
