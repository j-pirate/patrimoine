import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import { api } from './services/api';
import { emptyRegistry } from './services/registry-types';
import type { Entity } from './services/registry-types';

vi.mock('./services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('./services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});

const person: Entity = { id: 'alice', revision_id: 'r2', kind: 'person', display_name: 'Alice actuelle',
  first_name: 'Alice', last_name: null, birth_date: null, residence_country: 'TN', preferred_currency: 'EUR',
  activity: '', notes: '', archived: false };

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.getStatus).mockResolvedValue({ generation_id: 'vault', data_dir: '/local', schema_version: 3,
    app_version: '0.1.0', document_count: 0, mutation_sequence: 4, country_count: 4, currency_count: 5,
    settings: { workspace_name: 'Coffre de test', display_currency: 'EUR' } });
  vi.mocked(api.listRegistry).mockResolvedValue({ ...emptyRegistry, entities: [person], knowledge_sequence: 4 });
  vi.mocked(api.listRegistryAt).mockResolvedValue(emptyRegistry);
  vi.mocked(api.listRegistryHistory).mockResolvedValue({ latest_sequence: 4, events: [{ sequence: 4,
    recorded_at: '2026-11-04T12:00:00Z', object_id: 'alice', revision_id: 'r2', kind: 'entity', label: 'Alice actuelle',
    reason: 'Correction rétrospective', is_void: false }] });
});

describe('Date économique et connaissances globales', () => {
  it('consulte K=0 sans écriture puis retrouve les fiches et actions aux connaissances actuelles', async () => {
    const user = userEvent.setup(); render(<App />);
    await screen.findByText('Coffre de test');
    await user.click(screen.getByRole('button', { name: 'Famille' }));
    await screen.findByRole('table', { name: 'Personnes, membres et tiers' });
    await user.selectOptions(screen.getByLabelText('Connaissances disponibles'), '0');
    await waitFor(() => expect(api.listRegistryAt).toHaveBeenCalledExactlyOnceWith(0));
    const create = await screen.findByRole('button', { name: 'Créer une personne' });
    expect(create).toBeDisabled();
    expect(screen.queryByRole('option', { name: 'Alice actuelle' })).not.toBeInTheDocument();
    expect(api.saveEntity).not.toHaveBeenCalled();
    await user.selectOptions(screen.getByLabelText('Connaissances disponibles'), 'current');
    await waitFor(() => expect(screen.getByRole('button', { name: 'Créer une personne' })).toBeEnabled());
    expect(within(screen.getByRole('table', { name: 'Personnes, membres et tiers' })).getByText('Alice actuelle')).toBeVisible();
    expect(screen.getByLabelText('Date de référence')).not.toHaveValue('');
  });

  it('choisit une séquence depuis le journal et conserve D indépendante de K', async () => {
    const user = userEvent.setup(); render(<App />);
    await screen.findByText('Coffre de test');
    fireEvent.change(screen.getByLabelText('Date de référence'), { target: { value: '2021-06-30' } });
    await user.click(screen.getByRole('button', { name: 'Historique' }));
    await user.click(await screen.findByRole('button', { name: 'Consulter la séquence 4' }));
    await waitFor(() => expect(api.listRegistryAt).toHaveBeenCalledWith(4));
    expect(screen.getByLabelText('Connaissances disponibles')).toHaveValue('4');
    expect(screen.getByLabelText('Date de référence')).toHaveValue('2021-06-30');
    expect(api.listRegistryHistory).toHaveBeenCalledWith();
    expect(api.activateRestore).not.toHaveBeenCalled();
  });
});
