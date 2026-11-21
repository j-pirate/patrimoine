import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import type { DashboardBucket, DashboardMember, DashboardPoint, DashboardTotals } from '../services/dashboard-types';
import type { CompanyPeriodAnalysis } from '../services/companies-types';
import { DashboardAllocationChart, DashboardCompanyChart, DashboardHistoryChart, DashboardMembersChart } from './dashboard-charts';

const drawn = vi.hoisted(() => ({ charts: [] as { kind: string; data: Record<string, unknown>[] }[], lines: [] as Record<string, unknown>[], axes: [] as Record<string, unknown>[] }));
vi.mock('recharts', async () => {
  const React = await import('react');
  const box = ({ children }: { children?: React.ReactNode }) => React.createElement('div', {}, children);
  const chart = (kind: string) => ({ children, data }: { children?: React.ReactNode; data: Record<string, unknown>[] }) => {
    drawn.charts.push({ kind, data });
    return React.createElement('div', { 'data-testid': kind }, children);
  };
  return { ResponsiveContainer: box, LineChart: chart('line-chart'), BarChart: chart('bar-chart'),
    Line: (props: Record<string, unknown>) => { drawn.lines.push(props); return null; },
    Bar: () => null, CartesianGrid: () => null, Legend: () => null, ReferenceLine: () => null,
    Tooltip: () => null, XAxis: (props: Record<string, unknown>) => { drawn.axes.push(props); return null; }, YAxis: () => null };
});
const fraction = { num: '0', den: '1' };
function totals(net: string | null, knownAssets = '100.00', knownLiabilities = '10.00'): DashboardTotals {
  return { known_assets_eur: knownAssets, known_liabilities_eur: knownLiabilities, known_net_eur: '90.00',
    exact_known_assets: fraction, exact_known_liabilities: fraction, exact_known_net: fraction,
    assets_eur: net === null ? null : knownAssets, liabilities_eur: net === null ? null : knownLiabilities,
    net_eur: net, position_count: 2, unknown_count: net === null ? 1 : 0 };
}
function point(date: string, net: string | null): DashboardPoint {
  return { date, status: net === null ? 'partial' : 'complete', totals: totals(net), source_ids: [`source-${date}`], warnings: [] };
}
beforeEach(() => { drawn.charts.length = 0; drawn.lines.length = 0; drawn.axes.length = 0; });

describe('Graphiques alimentés par les résultats exacts', () => {
  it('DASH-03 représente les durées irrégulières sur un axe temporel numérique en UTC', () => {
    const points = [point('2026-01-01', '100.00'), point('2026-01-02', null), point('2026-01-11', '0.00')];
    render(<DashboardHistoryChart points={points} />);
    const data = drawn.charts.at(-1)!.data;
    const times = data.map(row => row.timestamp as number);
    expect(times).toEqual(points.map(row => Date.parse(`${row.date}T00:00:00Z`)));
    expect(times[2] - times[1]).toBe(9 * (times[1] - times[0]));
    expect(drawn.axes.at(-1)).toMatchObject({ dataKey: 'timestamp', type: 'number', scale: 'time', domain: ['dataMin', 'dataMax'] });
    const format = drawn.axes.at(-1)!.tickFormatter as (time: number) => string;
    expect(format(times[0])).toBe('01/01/2026');
    expect(data.map(row => row.net)).toEqual([100, null, 0]);
    expect(data.map(row => row.point)).toEqual(points);
    expect(drawn.lines.at(-1)).toMatchObject({ connectNulls: false });
  });
  it('interrompt le net inconnu sans transformer une vraie valeur zéro en lacune', () => {
    render(<DashboardHistoryChart points={[point('2026-01-01', '100.00'), point('2026-02-01', null), point('2026-03-01', '0.00')]} />);
    expect(drawn.charts.at(-1)?.data.map(row => row.net)).toEqual([100, null, 0]);
    expect(drawn.lines.at(-1)).toMatchObject({ dataKey: 'net', connectNulls: false, isAnimationActive: false });
    fireEvent.click(screen.getByText('Consulter les valeurs exactes de l’historique'));
    const table = screen.getByRole('table', { name: 'Historique des positions calculées en EUR' });
    expect(within(table).getByText('Non calculable')).toBeVisible();
    expect(within(table).getByText('0,00 EUR')).toBeVisible();
    expect(screen.getByText(/sans remplacement par zéro/)).toBeVisible();
  });

  it('propose les sous-totaux distinctement quand toutes les dates restent incomplètes', () => {
    render(<DashboardHistoryChart points={[point('2026-02-01', null)]} />);
    expect(screen.queryByTestId('line-chart')).not.toBeInTheDocument();
    expect(screen.getByText('Patrimoine net non calculable aux dates observées.')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Sous-totaux connus' }));
    expect(screen.getByRole('button', { name: 'Sous-totaux connus' })).toHaveAttribute('aria-pressed', 'true');
    expect(drawn.charts.at(-1)?.data[0]).toMatchObject({ assets: 100, liabilities: 10, net: null });
    expect(drawn.lines.slice(-2).map(line => line.dataKey)).toEqual(['assets', 'liabilities']);
    expect(screen.getByText(/ne constituent pas un patrimoine net complet/)).toBeVisible();
  });

  it('conserve les chiffres au-delà de la précision JavaScript dans le tableau et les sources', () => {
    const record = point('2026-03-31', '9007199254740993.12');
    render(<DashboardHistoryChart points={[record]} />);
    fireEvent.click(screen.getByText('Consulter les valeurs exactes de l’historique'));
    expect(screen.getByText('9\u202f007\u202f199\u202f254\u202f740\u202f993,12 EUR', { normalizer: value => value })).toBeVisible();
    fireEvent.click(screen.getByText('Sources de cette date'));
    expect(screen.getByText('source-2026-03-31')).toBeVisible();
    expect(drawn.charts.at(-1)?.data[0].point).toBe(record);
    expect(record.totals.net_eur).toBe('9007199254740993.12');
  });

  it('affiche les actifs et passifs séparés avec les inconnus sans fabriquer un pourcentage', () => {
    const bucket: DashboardBucket = { key: 'FR', label: 'France', known_assets_eur: '1000.00', known_liabilities_eur: '450.00', known_net_eur: '550.00', exact_assets: fraction, exact_liabilities: fraction, exact_net: fraction, position_count: 5, unknown_count: 2 };
    render(<DashboardAllocationChart buckets={[bucket]} title="Répartition par pays" />);
    expect(drawn.charts.at(-1)?.data[0]).toMatchObject({ assets: 1000, liabilities: 450 });
    fireEvent.click(screen.getByText('Consulter les valeurs exactes de cette répartition'));
    const table = screen.getByRole('table', { name: 'Répartition par pays — valeurs exactes en EUR' });
    expect(within(table).getByText('1\u202f000,00 EUR', { normalizer: value => value })).toBeVisible();
    expect(within(table).getByText('450,00 EUR')).toBeVisible();
    expect(within(table).getByText('Montants inconnus')).toBeVisible();
    expect(within(table).getByText('2')).toBeVisible();
    expect(screen.queryByText('%')).not.toBeInTheDocument();
  });

  it('conserve une différence individuelle négative sans l’annoncer comme net complet', () => {
    const member: DashboardMember = { entity_id: 'membre', label: 'Alice', totals: { ...totals(null), known_net_eur: '-40.00' } };
    render(<DashboardMembersChart members={[member]} />);
    expect(drawn.charts.at(-1)?.data[0].known).toBe(-40);
    expect(screen.getByText(/leurs bruts ne s’additionnent pas au brut familial/)).toBeVisible();
    fireEvent.click(screen.getByText('Consulter les valeurs exactes par membre'));
    const table = screen.getByRole('table', { name: 'Vues individuelles — valeurs exactes en EUR' });
    expect(within(table).getByText('-40,00 EUR')).toBeVisible();
    expect(within(table).getByText('Non calculable')).toBeVisible();
  });

  it('laisse les graphiques vides sans données de démonstration', () => {
    render(<><DashboardHistoryChart points={[]} /><DashboardAllocationChart buckets={[]} title="Répartition" /><DashboardMembersChart members={[]} /></>);
    expect(drawn.charts).toHaveLength(0);
    expect(screen.getByText('Aucune date à représenter.')).toBeVisible();
    expect(screen.getByText('Aucune position dans cette répartition.')).toBeVisible();
    expect(screen.getByText('Aucun membre dans le périmètre familial à cette date.')).toBeVisible();
  });

  it('sépare les fréquences et devises de société, conserve les montants manquants et les vrais zéros', () => {
    const period = (id: string, frequency: string, currency: string, end: string, revenue: string | null, result: string | null): CompanyPeriodAnalysis => ({
      period: { id, revision_id: `${id}-revision`, company_id: 'company', period_start: '2025-01-01', period_end: end,
        frequency, currency, revenue, net_result: result, operating_expenses: null, operating_result: null, indicative_cash: null,
        indicative_debt: null, important_receivables: null, dividends_declared: null, indicative_global_value: null, notes: '' },
      calculated_operating_result: null, operating_result_difference: null, trends: [],
    });
    const periods = [period('month', 'monthly', 'EUR', '2025-02-01', '900', '100'),
      period('annual-usd', 'annual', 'USD', '2026-01-01', '888', '88'),
      period('annual-eur-new', 'annual', 'EUR', '2026-01-01', null, '0'),
      period('annual-eur-old', 'annual', 'EUR', '2025-01-01', '12345678901234567890.1234', null)];
    render(<DashboardCompanyChart periods={periods} />);
    expect(drawn.charts.at(-1)?.data.map(row => [row.date, row.revenue, row.result])).toEqual([
      ['2025-01-01', Number('12345678901234567890.1234'), null], ['2026-01-01', null, 0],
    ]);
    expect(drawn.charts.at(-1)?.data.map(row => row.timestamp)).toEqual([Date.parse('2025-01-01T00:00:00Z'), Date.parse('2026-01-01T00:00:00Z')]);
    expect(drawn.axes.at(-1)).toMatchObject({ dataKey: 'timestamp', type: 'number', scale: 'time', domain: ['dataMin', 'dataMax'] });
    expect(drawn.lines.slice(-2).every(line => line.connectNulls === false)).toBe(true);
    fireEvent.click(screen.getByText('Consulter les observations exactes sélectionnées'));
    expect(screen.getByText('12\u202f345\u202f678\u202f901\u202f234\u202f567\u202f890,1234 EUR', { normalizer: value => value })).toBeVisible();
    expect(screen.queryByText('888 USD')).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Devise des observations'), { target: { value: 'USD' } });
    expect(drawn.charts.at(-1)?.data.map(row => row.revenue)).toEqual([888]);
    fireEvent.change(screen.getByLabelText('Fréquence des observations'), { target: { value: 'monthly' } });
    expect(screen.getByLabelText('Devise des observations')).toHaveValue('EUR');
    expect(drawn.charts.at(-1)?.data.map(row => row.revenue)).toEqual([900]);
    expect(screen.getByText(/Indicateurs de pilotage, hors bilan patrimonial/)).toBeVisible();
  });
});
