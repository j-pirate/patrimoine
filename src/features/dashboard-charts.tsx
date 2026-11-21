import { useId, useState } from 'react';
import { Bar, BarChart, CartesianGrid, Legend, Line, LineChart, ReferenceLine, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import type { DashboardBucket, DashboardMember, DashboardPoint } from '../services/dashboard-types';
import type { CompanyPeriodAnalysis } from '../services/companies-types';

// The backend has already calculated and rounded every financial value. Only
// the chart geometry is converted to Number; labels keep the exact source text.
function geometry(value: string | null): number | null {
  if (value === null) return null;
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}
function money(value: string | null, currency = 'EUR'): string {
  if (value === null) return 'Non calculable';
  const [integer, decimals] = value.split('.');
  return `${integer.replace(/\B(?=(\d{3})+(?!\d))/g, '\u202f')}${decimals === undefined ? '' : `,${decimals}`} ${currency}`;
}
function dateLabel(value: string): string {
  const [year, month, day] = value.split('-');
  return `${day}/${month}/${year}`;
}
function timestamp(value: string): number { return Date.parse(`${value}T00:00:00Z`); }
function timeLabel(value: number): string { return dateLabel(new Date(value).toISOString().slice(0, 10)); }
function axisLabel(value: number): string {
  if (Math.abs(value) >= 1_000_000) return `${(value / 1_000_000).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} M`;
  if (Math.abs(value) >= 1_000) return `${(value / 1_000).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} k`;
  return value.toLocaleString('fr-FR', { maximumFractionDigits: 0 });
}
const chartMargin = { top: 16, right: 20, bottom: 8, left: 12 };
const colors = { assets: '#386c58', liabilities: '#b7844d', net: '#306887', grid: '#e4ebe5' };

function Sources({ sourceIds }: { sourceIds: string[] }) {
  return sourceIds.length ? <details><summary>Sources de cette date</summary><p className="hash">{sourceIds.join(', ')}</p></details> : <span className="muted">Aucune source</span>;
}

export function DashboardHistoryChart({ points }: { points: DashboardPoint[] }) {
  const id = useId();
  const [mode, setMode] = useState<'net' | 'known'>('net');
  const data = points.map(point => ({
    date: point.date,
    timestamp: timestamp(point.date),
    net: geometry(point.totals.net_eur),
    assets: geometry(point.totals.known_assets_eur),
    liabilities: geometry(point.totals.known_liabilities_eur),
    point,
  }));
  const hasValues = data.some(row => mode === 'net' ? row.net !== null : row.assets !== null || row.liabilities !== null);
  return <section className="dashboard-chart" aria-labelledby={`${id}-title`}>
    <div className="section-title"><div><span className="eyebrow">LES MÊMES CONNAISSANCES À CHAQUE DATE</span><h2 id={`${id}-title`}>Évolution patrimoniale</h2></div>
      <div className="dashboard-chart-legend" role="group" aria-label="Série historique affichée">
        <button type="button" className={`button small-button ${mode === 'net' ? '' : 'secondary'}`} aria-pressed={mode === 'net'} onClick={() => setMode('net')}>Patrimoine net</button>
        <button type="button" className={`button small-button ${mode === 'known' ? '' : 'secondary'}`} aria-pressed={mode === 'known'} onClick={() => setMode('known')}>Sous-totaux connus</button>
      </div>
    </div>
    <p className="muted small">{mode === 'net' ? 'Les dates non calculables restent des interruptions de la courbe, sans remplacement par zéro.' : 'Actifs et passifs connus seulement. Ces séries peuvent rester partielles et ne constituent pas un patrimoine net complet.'} Une variation de stock ne mesure pas un rendement.</p>
    {hasValues ? <div className="dashboard-chart-frame" role="group" aria-label={mode === 'net' ? 'Courbe du patrimoine net en EUR' : 'Courbes des actifs et passifs connus en EUR'}>
      <ResponsiveContainer width="100%" height={260} minWidth={0}>
        <LineChart data={data} margin={chartMargin} accessibilityLayer>
          <CartesianGrid stroke={colors.grid} vertical={false} />
          <XAxis dataKey="timestamp" type="number" scale="time" domain={['dataMin', 'dataMax']} tickFormatter={timeLabel} minTickGap={40} tick={{ fontSize: 11 }} tickLine={false} axisLine={false} />
          <YAxis tickFormatter={axisLabel} width={65} tick={{ fontSize: 11 }} tickLine={false} axisLine={false} />
          <ReferenceLine y={0} stroke="#a9b9ad" />
          <Tooltip content={({ active, payload }) => {
            if (!active || !payload?.length) return null;
            const row = payload[0].payload as (typeof data)[number];
            return <div className="dashboard-chart-tooltip"><strong>{dateLabel(row.date)}</strong>{mode === 'net' ? <p>Patrimoine net : {money(row.point.totals.net_eur)}</p> : <><p>Actifs connus : {money(row.point.totals.known_assets_eur)}</p><p>Passifs connus : {money(row.point.totals.known_liabilities_eur)}</p></>}</div>;
          }} />
          {mode === 'net' ? <Line type="linear" dataKey="net" name="Patrimoine net" stroke={colors.net} strokeWidth={3} connectNulls={false} dot={{ r: 3 }} activeDot={{ r: 5 }} isAnimationActive={false} /> : <>
            <Line type="linear" dataKey="assets" name="Actifs connus" stroke={colors.assets} strokeWidth={3} connectNulls={false} dot={{ r: 2 }} isAnimationActive={false} />
            <Line type="linear" dataKey="liabilities" name="Passifs connus" stroke={colors.liabilities} strokeWidth={2} strokeDasharray="5 4" connectNulls={false} dot={{ r: 2 }} isAnimationActive={false} />
            <Legend iconType="plainline" />
          </>}
        </LineChart>
      </ResponsiveContainer>
    </div> : <p className="dashboard-chart-empty" role="status">{points.length ? 'Patrimoine net non calculable aux dates observées.' : 'Aucune date à représenter.'}</p>}
    <details className="dashboard-chart-table"><summary>Consulter les valeurs exactes de l’historique</summary><div className="table-scroll" tabIndex={0}><table>
      <caption>Historique des positions calculées en EUR</caption><thead><tr><th>Date</th><th>Actifs connus</th><th>Passifs connus</th><th>Patrimoine net complet</th><th>Couverture et sources</th></tr></thead>
      <tbody>{points.map(point => <tr key={point.date}><td>{dateLabel(point.date)}</td><td>{money(point.totals.known_assets_eur)}</td><td>{money(point.totals.known_liabilities_eur)}</td><td>{money(point.totals.net_eur)}</td><td>{point.status === 'complete' ? 'Complète' : 'Partielle'}{point.warnings.length > 0 && <span> · {point.warnings.length} alerte{point.warnings.length > 1 ? 's' : ''}</span>}<Sources sourceIds={point.source_ids} /></td></tr>)}</tbody>
    </table></div></details>
    <p className="muted small">L’espacement des dates suit la durée écoulée. Les graduations donnent un ordre de grandeur. Les valeurs exactes et les références sont conservées dans le tableau.</p>
  </section>;
}

export function DashboardAllocationChart({ buckets, title }: { buckets: DashboardBucket[]; title: string }) {
  const id = useId();
  const data = buckets.map(bucket => ({ label: bucket.label, assets: geometry(bucket.known_assets_eur), liabilities: geometry(bucket.known_liabilities_eur), bucket }));
  return <section className="dashboard-chart" aria-labelledby={`${id}-title`}>
    <div className="section-title"><h2 id={`${id}-title`}>{title}</h2><span className="badge">EUR · après éliminations</span></div>
    <p className="muted small">Répartition des positions connues sélectionnées. Les montants inconnus restent signalés dans le tableau ; les garanties conditionnelles sont hors bilan.</p>
    {buckets.length ? <div className="dashboard-chart-frame" role="group" aria-label={`${title}, actifs et passifs connus en EUR`}>
      <ResponsiveContainer width="100%" height={Math.max(220, Math.min(480, 60 + buckets.length * 48))} minWidth={0}>
        <BarChart data={data} layout="vertical" margin={{ ...chartMargin, left: 2 }} accessibilityLayer>
          <CartesianGrid stroke={colors.grid} horizontal={false} />
          <XAxis type="number" tickFormatter={axisLabel} tick={{ fontSize: 11 }} tickLine={false} axisLine={false} />
          <YAxis type="category" dataKey="label" width={115} tick={{ fontSize: 11 }} tickLine={false} axisLine={false} />
          <ReferenceLine x={0} stroke="#a9b9ad" />
          <Tooltip content={({ active, payload }) => {
            if (!active || !payload?.length) return null;
            const { bucket } = payload[0].payload as (typeof data)[number];
            return <div className="dashboard-chart-tooltip"><strong>{bucket.label}</strong><p>Actifs connus : {money(bucket.known_assets_eur)}</p><p>Passifs connus : {money(bucket.known_liabilities_eur)}</p>{bucket.unknown_count > 0 && <p>{bucket.unknown_count} montant{bucket.unknown_count > 1 ? 's' : ''} inconnu{bucket.unknown_count > 1 ? 's' : ''}</p>}</div>;
          }} />
          <Legend iconType="circle" />
          <Bar dataKey="assets" name="Actifs connus" fill={colors.assets} radius={[0, 3, 3, 0]} isAnimationActive={false} />
          <Bar dataKey="liabilities" name="Passifs connus" fill={colors.liabilities} radius={[0, 3, 3, 0]} isAnimationActive={false} />
        </BarChart>
      </ResponsiveContainer>
    </div> : <p className="dashboard-chart-empty">Aucune position dans cette répartition.</p>}
    <details className="dashboard-chart-table"><summary>Consulter les valeurs exactes de cette répartition</summary><div className="table-scroll" tabIndex={0}><table>
      <caption>{title} — valeurs exactes en EUR</caption><thead><tr><th>Groupe</th><th>Actifs connus</th><th>Passifs connus</th><th>Différence connue</th><th>Positions</th><th>Montants inconnus</th></tr></thead>
      <tbody>{buckets.map(bucket => <tr key={bucket.key}><td>{bucket.label}</td><td>{money(bucket.known_assets_eur)}</td><td>{money(bucket.known_liabilities_eur)}</td><td>{money(bucket.known_net_eur)}</td><td>{bucket.position_count}</td><td>{bucket.unknown_count}</td></tr>)}</tbody>
    </table></div></details>
  </section>;
}

export function DashboardMembersChart({ members }: { members: DashboardMember[] }) {
  const id = useId();
  const data = members.map(member => ({ label: member.label, known: geometry(member.totals.known_net_eur), member }));
  return <section className="dashboard-chart" aria-labelledby={`${id}-title`}>
    <div className="section-title"><h2 id={`${id}-title`}>Différences connues par membre</h2><span className="badge">Vues individuelles</span></div>
    <p className="muted small">Chaque membre est calculé séparément selon ses droits économiques. Les actifs et passifs individuels peuvent inclure des créances internes éliminées dans la vue familiale ; leurs bruts ne s’additionnent pas au brut familial. Une différence connue n’est pas un net complet.</p>
    {members.length ? <div className="dashboard-chart-frame" role="group" aria-label="Comparaison des différences connues par membre en EUR">
      <ResponsiveContainer width="100%" height={Math.max(220, Math.min(480, 60 + members.length * 44))} minWidth={0}>
        <BarChart data={data} layout="vertical" margin={{ ...chartMargin, left: 2 }} accessibilityLayer>
          <CartesianGrid stroke={colors.grid} horizontal={false} /><XAxis type="number" tickFormatter={axisLabel} tick={{ fontSize: 11 }} tickLine={false} axisLine={false} /><YAxis type="category" dataKey="label" width={115} tick={{ fontSize: 11 }} tickLine={false} axisLine={false} />
          <ReferenceLine x={0} stroke="#a9b9ad" />
          <Tooltip content={({ active, payload }) => {
            if (!active || !payload?.length) return null;
            const { member } = payload[0].payload as (typeof data)[number];
            return <div className="dashboard-chart-tooltip"><strong>{member.label}</strong><p>Différence connue : {money(member.totals.known_net_eur)}</p><p>Patrimoine net complet : {money(member.totals.net_eur)}</p></div>;
          }} />
          <Bar dataKey="known" name="Différence connue" fill={colors.net} radius={[0, 3, 3, 0]} isAnimationActive={false} />
        </BarChart>
      </ResponsiveContainer>
    </div> : <p className="dashboard-chart-empty">Aucun membre dans le périmètre familial à cette date.</p>}
    <details className="dashboard-chart-table"><summary>Consulter les valeurs exactes par membre</summary><div className="table-scroll" tabIndex={0}><table>
      <caption>Vues individuelles — valeurs exactes en EUR</caption><thead><tr><th>Membre</th><th>Actifs connus</th><th>Passifs connus</th><th>Différence connue</th><th>Patrimoine net complet</th></tr></thead>
      <tbody>{members.map(member => <tr key={member.entity_id}><td>{member.label}</td><td>{money(member.totals.known_assets_eur)}</td><td>{money(member.totals.known_liabilities_eur)}</td><td>{money(member.totals.known_net_eur)}</td><td>{money(member.totals.net_eur)}</td></tr>)}</tbody>
    </table></div></details>
  </section>;
}

export function DashboardCompanyChart({ periods }: { periods: CompanyPeriodAnalysis[] }) {
  const id = useId();
  const [frequency, setFrequency] = useState('annual');
  const [currency, setCurrency] = useState('');
  const frequencies = [...new Set(periods.map(analysis => analysis.period.frequency))].sort();
  const selectedFrequency = frequencies.includes(frequency) ? frequency : frequencies[0] ?? '';
  const currencies = [...new Set(periods.filter(analysis => analysis.period.frequency === selectedFrequency).map(analysis => analysis.period.currency))].sort();
  const selectedCurrency = currencies.includes(currency) ? currency : currencies[0] ?? '';
  const selected = periods.filter(analysis => analysis.period.frequency === selectedFrequency && analysis.period.currency === selectedCurrency)
    .sort((left, right) => left.period.period_end.localeCompare(right.period.period_end) || left.period.period_start.localeCompare(right.period.period_start) || left.period.id.localeCompare(right.period.id));
  const data = selected.map(analysis => ({ date: analysis.period.period_end, timestamp: timestamp(analysis.period.period_end), revenue: geometry(analysis.period.revenue), result: geometry(analysis.period.net_result), analysis }));
  const hasValues = data.some(row => row.revenue !== null || row.result !== null);
  const frequencyLabels: Record<string, string> = { monthly: 'Mensuelle', quarterly: 'Trimestrielle', annual: 'Annuelle' };
  return <section className="dashboard-chart" aria-labelledby={`${id}-title`}>
    <div className="section-title"><div><span className="eyebrow">PILOTAGE DE LA SOCIÉTÉ</span><h2 id={`${id}-title`}>Évolution des observations de société</h2></div><span className="badge">Devise d’origine</span></div>
    <p className="muted small">Indicateurs de pilotage, hors bilan patrimonial. Une seule fréquence et une seule devise sont représentées ; elles ne sont ni additionnées ni converties en patrimoine. L’abscisse donne la fin de période, exclue, avec un espacement proportionnel à la durée écoulée.</p>
    <div className="dashboard-chart-legend">
      <div><label htmlFor={`${id}-frequency`}>Fréquence des observations</label><select id={`${id}-frequency`} value={selectedFrequency} disabled={!frequencies.length} onChange={event => setFrequency(event.currentTarget.value)}>{frequencies.map(value => <option key={value} value={value}>{frequencyLabels[value] ?? value}</option>)}</select></div>
      <div><label htmlFor={`${id}-currency`}>Devise des observations</label><select id={`${id}-currency`} value={selectedCurrency} disabled={!currencies.length} onChange={event => setCurrency(event.currentTarget.value)}>{currencies.map(value => <option key={value} value={value}>{value}</option>)}</select></div>
    </div>
    {hasValues ? <div className="dashboard-chart-frame" role="group" aria-label={`Observations ${frequencyLabels[selectedFrequency] ?? selectedFrequency} en ${selectedCurrency}`}>
      <ResponsiveContainer width="100%" height={260} minWidth={0}>
        <LineChart data={data} margin={chartMargin} accessibilityLayer>
          <CartesianGrid stroke={colors.grid} vertical={false} /><XAxis dataKey="timestamp" type="number" scale="time" domain={['dataMin', 'dataMax']} tickFormatter={timeLabel} minTickGap={40} tick={{ fontSize: 11 }} tickLine={false} axisLine={false} /><YAxis tickFormatter={axisLabel} width={65} tick={{ fontSize: 11 }} tickLine={false} axisLine={false} />
          <ReferenceLine y={0} stroke="#a9b9ad" />
          <Tooltip content={({ active, payload }) => {
            if (!active || !payload?.length) return null;
            const { analysis } = payload[0].payload as (typeof data)[number];
            return <div className="dashboard-chart-tooltip"><strong>{analysis.period.period_start} → {analysis.period.period_end} (exclu)</strong><p>Chiffre d’affaires : {money(analysis.period.revenue, analysis.period.currency)}</p><p>Résultat net déclaré : {money(analysis.period.net_result, analysis.period.currency)}</p></div>;
          }} />
          <Legend iconType="plainline" />
          <Line type="linear" dataKey="revenue" name="Chiffre d’affaires" stroke={colors.assets} strokeWidth={3} connectNulls={false} dot={{ r: 3 }} isAnimationActive={false} />
          <Line type="linear" dataKey="result" name="Résultat net déclaré" stroke={colors.net} strokeWidth={2} strokeDasharray="5 4" connectNulls={false} dot={{ r: 3 }} isAnimationActive={false} />
        </LineChart>
      </ResponsiveContainer>
    </div> : <p className="dashboard-chart-empty">{periods.length ? 'Aucun montant renseigné pour cette fréquence et cette devise.' : 'Aucune observation historique de société.'}</p>}
    <details className="dashboard-chart-table"><summary>Consulter les observations exactes sélectionnées</summary><div className="table-scroll" tabIndex={0}><table>
      <caption>Observations de société — {frequencyLabels[selectedFrequency] ?? 'aucune fréquence'}, {selectedCurrency || 'aucune devise'}</caption><thead><tr><th>Période</th><th>Chiffre d’affaires</th><th>Résultat net déclaré</th><th>Sources</th></tr></thead>
      <tbody>{selected.map(analysis => <tr key={analysis.period.id}><td>{analysis.period.period_start} → {analysis.period.period_end} (exclu)</td><td>{money(analysis.period.revenue, analysis.period.currency)}</td><td>{money(analysis.period.net_result, analysis.period.currency)}</td><td><p className="hash">{analysis.period.id} · {analysis.period.revision_id}</p></td></tr>)}</tbody>
    </table></div></details>
  </section>;
}
