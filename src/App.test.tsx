import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import App from './App';
import { api } from './services/api';
import type { RestoreInspection, VaultStatus } from './services/api';
import type { DocumentLibrarySnapshot } from './services/documents-types';
import { emptyRegistry } from './services/registry-types';

vi.mock('./services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('./services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});

const status: VaultStatus = {
  generation_id: 'current', data_dir: '/test/coffre', schema_version: 1, app_version: '0.1.0',
  document_count: 0, mutation_sequence: 1, country_count: 4, currency_count: 5, recovery_required: false, recovery_message: null,
  settings: { workspace_name: 'Famille de test', display_currency: 'EUR' },
};
const inspection: RestoreInspection = {
  token: 'validated-token', archive_sha256: 'abc123', created_at: '2026-11-04T10:00:00Z',
  schema_version: 1, app_version: '0.1.0', document_count: 2, blob_count: 1,
  settings: { workspace_name: 'Espace restauré', display_currency: 'CHF' },
  source_generation_id: 'source', current_generation_id: 'current', mutation_sequence: 1,
};
const emptyDocuments: DocumentLibrarySnapshot = { documents: [], versions: [], links: [], targets: [], knowledge_sequence: 0 };

async function openPage(name: string) {
  const user = userEvent.setup();
  render(<App />);
  await screen.findByText('Famille de test');
  await user.click(screen.getByRole('button', { name }));
  return user;
}

async function prepareRestore() {
  const user = await openPage('Sauvegarde et export');
  await user.click(await screen.findByRole('button', { name: 'Choisir une sauvegarde à analyser' }));
  await screen.findByRole('heading', { name: 'Vérifier avant de restaurer' });
  return user;
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.getStatus).mockResolvedValue(status);
  vi.mocked(api.getSettings).mockResolvedValue(status.settings);
  vi.mocked(api.listDocuments).mockResolvedValue([]);
  vi.mocked(api.listDocumentLibraryAt).mockResolvedValue(emptyDocuments);
  vi.mocked(api.listRegistry).mockResolvedValue(emptyRegistry);
  vi.mocked(api.chooseRestoreArchive).mockResolvedValue('/test/archive.zip');
  vi.mocked(api.inspectRestore).mockResolvedValue(inspection);
  vi.mocked(api.activateRestore).mockResolvedValue(status);
});

describe('Sauvegarde et restauration', () => {
  it('présente le résumé sans modifier le coffre et permet d’annuler avant confirmation', async () => {
    const user = await prepareRestore();
    expect(api.inspectRestore).toHaveBeenCalledWith('/test/archive.zip');
    expect(api.activateRestore).not.toHaveBeenCalled();
    expect(screen.getByRole('checkbox', { name: /Créer une sauvegarde de mon état actuel/ })).toBeChecked();
    expect(screen.getByText(/remplacera l’intégralité du coffre actuel/)).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(screen.queryByRole('heading', { name: 'Vérifier avant de restaurer' })).not.toBeInTheDocument();
    expect(api.activateRestore).not.toHaveBeenCalled();
  });

  it('n’active pas la restauration si le dialogue de sauvegarde préalable est annulé', async () => {
    vi.mocked(api.chooseBackupDestination).mockResolvedValue(null);
    const user = await prepareRestore();
    await user.click(screen.getByRole('button', { name: 'Confirmer le remplacement du coffre' }));
    await screen.findByText(/Restauration annulée/);
    expect(api.chooseBackupDestination).toHaveBeenCalledWith(true);
    expect(api.activateRestore).not.toHaveBeenCalled();
  });

  it('transmet le jeton validé et le chemin de sauvegarde après confirmation explicite', async () => {
    vi.mocked(api.chooseBackupDestination).mockResolvedValue('/test/avant-restauration.zip');
    const user = await prepareRestore();
    await user.click(screen.getByRole('button', { name: 'Confirmer le remplacement du coffre' }));
    await screen.findByText(/La sauvegarde a été restaurée/);
    expect(api.activateRestore).toHaveBeenCalledExactlyOnceWith('validated-token', '/test/avant-restauration.zip');
    expect(api.createBackup).not.toHaveBeenCalled(); // Le backend réalise la sauvegarde et la bascule sous le même verrou.
    expect(api.getStatus).toHaveBeenCalledTimes(2);
  });

  it('respecte le choix explicite de ne pas exporter de sauvegarde préalable', async () => {
    const user = await prepareRestore();
    await user.click(screen.getByRole('checkbox', { name: /Créer une sauvegarde de mon état actuel/ }));
    expect(screen.getByText(/Vous avez choisi de ne pas exporter/)).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Confirmer le remplacement du coffre' }));
    await screen.findByText(/La sauvegarde a été restaurée/);
    expect(api.activateRestore).toHaveBeenCalledWith('validated-token', null);
    expect(api.chooseBackupDestination).not.toHaveBeenCalled();
  });

  it('affiche un refus de restauration sans annoncer de réussite', async () => {
    vi.mocked(api.chooseBackupDestination).mockResolvedValue('/test/avant.zip');
    vi.mocked(api.activateRestore).mockRejectedValue({ code: 'STALE_TOKEN', message: 'Le coffre a changé. Analysez de nouveau la sauvegarde.' });
    const user = await prepareRestore();
    await user.click(screen.getByRole('button', { name: 'Confirmer le remplacement du coffre' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Le coffre a changé');
    expect(screen.queryByText(/La sauvegarde a été restaurée/)).not.toBeInTheDocument();
    expect(api.getStatus).toHaveBeenCalledTimes(1);
  });
});

describe('Paramètres locaux', () => {
  it('enregistre les valeurs saisies avec la devise de consolidation toujours en EUR', async () => {
    vi.mocked(api.saveSettings).mockImplementation(async settings => settings);
    const user = await openPage('Paramètres');
    const input = await screen.findByRole('textbox', { name: 'Nom de l’espace familial' });
    await user.clear(input); await user.type(input, '  Notre espace  ');
    await user.selectOptions(screen.getByRole('combobox', { name: 'Devise d’affichage préférée' }), 'TND');
    await user.click(screen.getByRole('button', { name: 'Enregistrer les paramètres' }));
    await screen.findByText(/Les paramètres ont été enregistrés/);
    expect(api.saveSettings).toHaveBeenCalledWith({ workspace_name: 'Notre espace', display_currency: 'TND' });
    expect(screen.getByText('Devise de consolidation').nextElementSibling).toHaveTextContent('EUR');
  });

  it('ne propose pas de valeurs fictives à enregistrer après un échec de lecture', async () => {
    vi.mocked(api.getSettings).mockRejectedValue({ code: 'READ_ERROR', message: 'Lecture des paramètres impossible.' });
    await openPage('Paramètres');
    expect(await screen.findByRole('alert')).toHaveTextContent('Lecture des paramètres impossible');
    expect(screen.queryByRole('button', { name: 'Enregistrer les paramètres' })).not.toBeInTheDocument();
    expect(api.saveSettings).not.toHaveBeenCalled();
  });

  it('conserve les deux champs lorsque les changements sont traités dans le même lot', async () => {
    vi.mocked(api.saveSettings).mockImplementation(async settings => settings);
    const user = await openPage('Paramètres');
    const input = await screen.findByRole('textbox', { name: 'Nom de l’espace familial' });
    const currency = screen.getByRole('combobox', { name: 'Devise d’affichage préférée' });
    act(() => {
      fireEvent.change(input, { target: { value: 'Nom et devise modifiés' } });
      fireEvent.change(currency, { target: { value: 'CHF' } });
    });
    await user.click(screen.getByRole('button', { name: 'Enregistrer les paramètres' }));
    await screen.findByText(/Les paramètres ont été enregistrés/);
    expect(api.saveSettings).toHaveBeenCalledWith({ workspace_name: 'Nom et devise modifiés', display_currency: 'CHF' });
  });

  it('limite le nom saisi à la longueur acceptée par le coffre', async () => {
    vi.mocked(api.saveSettings).mockImplementation(async settings => settings);
    const user = await openPage('Paramètres');
    const input = await screen.findByRole('textbox', { name: 'Nom de l’espace familial' });
    await user.clear(input);
    await user.type(input, 'a'.repeat(105));
    await user.click(screen.getByRole('button', { name: 'Enregistrer les paramètres' }));
    await screen.findByText(/Les paramètres ont été enregistrés/);
    expect(api.saveSettings).toHaveBeenCalledWith({ workspace_name: 'a'.repeat(100), display_currency: 'EUR' });
  });
});

describe('Documents', () => {
  it('importe un original puis exporte une copie avec les chemins choisis', async () => {
    const document = { id: 'doc-1', original_name: 'Acte.pdf', mime_type: 'application/pdf', size_bytes: 1200, sha256: 'sha', added_at: '2026-11-04T10:00:00Z' };
    vi.mocked(api.chooseDocumentImport).mockResolvedValue('/test/source.pdf');
    vi.mocked(api.importDocument).mockResolvedValue(document);
    vi.mocked(api.listDocuments).mockResolvedValueOnce([]).mockResolvedValue([document]);
    const record = { ...document, revision_id: 'doc-r1', description: '', status: 'active' as const, recorded_at: document.added_at };
    vi.mocked(api.listDocumentLibraryAt).mockResolvedValueOnce(emptyDocuments).mockResolvedValue({ ...emptyDocuments, documents: [record], versions: [record], knowledge_sequence: 1 });
    vi.mocked(api.chooseDocumentDestination).mockResolvedValue('/test/copie.pdf');
    vi.mocked(api.exportDocument).mockResolvedValue(undefined);
    vi.mocked(api.exportDocumentRevision).mockResolvedValue(undefined);
    const user = await openPage('Documents');
    await screen.findByText('Votre espace documentaire est vide');
    await user.click(screen.getByRole('button', { name: 'Importer un document' }));
    await screen.findByText('Acte.pdf');
    const exportButton = await screen.findByRole('button', { name: 'Exporter une copie de Acte.pdf' });
    await waitFor(() => expect(exportButton).toBeEnabled());
    await user.click(exportButton);
    await screen.findByText(/Une copie de « Acte.pdf » a été exportée/);
    expect(api.importDocument).toHaveBeenCalledWith('/test/source.pdf');
    expect(api.exportDocumentRevision).toHaveBeenCalledWith('doc-1', 'doc-r1', '/test/copie.pdf');
  });
});

describe('Contexte familial du registre', () => {
  it('revient au périmètre familial si le membre sélectionné n’existe pas dans le coffre restauré', async () => {
    const person = { id: 'alice', revision_id: 'alice-r1', kind: 'person' as const, display_name: 'Alice Martin', first_name: null, last_name: null, birth_date: null, residence_country: 'FR', preferred_currency: 'EUR', activity: '', notes: '', archived: false };
    vi.mocked(api.listRegistry).mockResolvedValueOnce({ ...emptyRegistry, entities: [person] }).mockResolvedValue(emptyRegistry);
    const user = await prepareRestore();
    await user.selectOptions(screen.getByLabelText('Périmètre de consultation'), person.id);
    await user.click(screen.getByRole('checkbox', { name: /Créer une sauvegarde de mon état actuel/ }));
    await user.click(screen.getByRole('button', { name: 'Confirmer le remplacement du coffre' }));
    await screen.findByText(/La sauvegarde a été restaurée/);
    await waitFor(() => expect(screen.getByLabelText('Périmètre de consultation')).toHaveValue('family'));
    expect(screen.queryByRole('option', { name: 'Alice Martin' })).not.toBeInTheDocument();
  });

  it('transmet le membre et la date globaux au contrôle des attributions', async () => {
    const person = { id: 'alice', revision_id: 'alice-r1', kind: 'person' as const, display_name: 'Alice Martin', first_name: 'Alice', last_name: 'Martin', birth_date: null, residence_country: 'FR', preferred_currency: 'EUR', activity: '', notes: '', archived: false };
    vi.mocked(api.listRegistry).mockResolvedValue({ ...emptyRegistry, entities: [person], knowledge_sequence: 4 });
    vi.mocked(api.getConsolidation).mockResolvedValue({
      context: 'person', person_id: person.id, date: '2025-06-30', knowledge_sequence: 4, engine_version: 'test', status: 'partial',
      gross_eur: '200000.00', debt_eur: '0.00', net_eur: '200000.00', exact_gross: { num: '200000', den: '1' },
      exact_debt: { num: '0', den: '1' }, exact_net: { num: '200000', den: '1' }, global_equity_eur: '0.00',
      gross_is_underlying_complete: false, contributions: [], warnings: [],
    });
    const user = await openPage('Patrimoine');
    await user.selectOptions(screen.getByLabelText('Périmètre de consultation'), person.id);
    fireEvent.change(screen.getByLabelText('Date de référence'), { target: { value: '2025-06-30' } });
    await user.click(screen.getByRole('button', { name: 'Contrôler les attributions' }));
    await screen.findByText('200000.00 EUR');
    expect(api.getConsolidation).toHaveBeenCalledWith({ context: 'person', person_id: 'alice', date: '2025-06-30', knowledge_sequence: null });
  });

  it('annonce un échec de lecture du registre sans proposer de données fictives à enregistrer', async () => {
    vi.mocked(api.listRegistry).mockRejectedValue({ code: 'database', message: 'Le registre local ne peut pas être lu.' });
    await openPage('Famille');
    expect(await screen.findByRole('alert')).toHaveTextContent('Le registre local ne peut pas être lu');
    expect(screen.queryByRole('button', { name: 'Créer une personne' })).not.toBeInTheDocument();
  });
});

describe('Compatibilité des sauvegardes', () => {
  it('annonce la mise à niveau d’une ancienne archive avant confirmation', async () => {
    vi.mocked(api.getStatus).mockResolvedValue({ ...status, schema_version: 2 });
    await prepareRestore();
    expect(screen.getByText(/Cette archive utilise le schéma/)).toHaveTextContent('schéma 1');
    expect(screen.getByText(/Cette archive utilise le schéma/)).toHaveTextContent('schéma 2');
    expect(api.activateRestore).not.toHaveBeenCalled();
  });
});
