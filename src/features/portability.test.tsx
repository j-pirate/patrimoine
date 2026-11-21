import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import PortabilityPage from './portability';
import { api } from '../services/api';
import type { RestoreInspection, VaultStatus } from '../services/api';
import type { ExportSummary } from '../services/portability-types';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});
const inspection: RestoreInspection = {
  token: 'csv-candidate', archive_sha256: 'csv-sha256', created_at: '2026-11-11T12:00:00Z',
  schema_version: 6, app_version: '0.1.0', document_count: 3, blob_count: 2,
  settings: { workspace_name: 'Données reconstruites', display_currency: 'CHF' },
  source_generation_id: 'source', current_generation_id: 'active', mutation_sequence: 52,
};
const exported: ExportSummary = { path: '/test/csv.zip', sha256: 'export-hash', size_bytes: 1500,
  format: 'structured_csv', schema_version: 6, knowledge_sequence: 52, table_count: 62,
  row_count: 410, document_count: 3, blob_count: 2 };
const restored: VaultStatus = { generation_id: 'new-generation', data_dir: '/test/coffre', schema_version: 6,
  app_version: '0.1.0', document_count: 3, mutation_sequence: 52, settings: inspection.settings, country_count: 4, currency_count: 5, recovery_required: false, recovery_message: null };
const onRestored = vi.fn<() => Promise<void>>();
const onBusyChange = vi.fn<(busy: boolean) => void>();
function show() { render(<PortabilityPage currentSchema={6} onRestored={onRestored} onBusyChange={onBusyChange} />); return userEvent.setup(); }
async function analyze() {
  const user = show();
  await user.click(screen.getByRole('button', { name: 'Choisir une archive CSV à analyser' }));
  await screen.findByRole('heading', { name: 'Vérifier avant de réimporter' });
  return user;
}
beforeEach(() => {
  vi.resetAllMocks();
  onRestored.mockResolvedValue(undefined);
  vi.mocked(api.chooseCsvExportDestination).mockResolvedValue('/test/csv.zip');
  vi.mocked(api.chooseXlsxDestination).mockResolvedValue('/test/consultation.xlsx');
  vi.mocked(api.chooseCsvImportArchive).mockResolvedValue('/test/source-csv.zip');
  vi.mocked(api.exportCsvArchive).mockResolvedValue(exported);
  vi.mocked(api.exportXlsx).mockResolvedValue({ ...exported, path: '/test/consultation.xlsx', format: 'xlsx' });
  vi.mocked(api.inspectCsvImport).mockResolvedValue(inspection);
  vi.mocked(api.chooseBackupDestination).mockResolvedValue('/test/préalable.zip');
  vi.mocked(api.activateRestore).mockResolvedValue(restored);
});

describe('Portabilité intégrale', () => {
  it('PORT-01 rafraîchit une activation effectuée et affiche la reprise encore nécessaire', async () => {
    vi.mocked(api.activateRestore).mockResolvedValue({ ...restored, recovery_required: true, recovery_message: 'Le coffre restauré est actif. Fermez et rouvrez l’application pour terminer la reprise.' });
    const user = await analyze();
    await user.click(screen.getByRole('button', { name: 'Confirmer le remplacement du coffre' }));
    expect(await screen.findByText(/L’archive CSV a été réimportée.*Le coffre restauré est actif/)).toBeVisible();
    expect(onRestored).toHaveBeenCalledOnce();
    expect(screen.queryByRole('heading', { name: 'Vérifier avant de réimporter' })).not.toBeInTheDocument();
    expect(api.activateRestore).toHaveBeenCalledOnce();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('REG-03 garde l’activation confirmée si le rafraîchissement échoue sans permettre sa répétition', async () => {
    onRestored.mockRejectedValue(new Error('Lecture après activation indisponible'));
    const user = await analyze();
    await user.click(screen.getByRole('button', { name: 'Confirmer le remplacement du coffre' }));
    expect(await screen.findByText(/L’archive CSV a été réimportée.*L’affichage n’a pas pu être actualisé/)).toBeVisible();
    expect(api.activateRestore).toHaveBeenCalledOnce();
    expect(screen.queryByRole('button', { name: 'Confirmer le remplacement du coffre' })).not.toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Choisir une archive CSV à analyser' })).toBeEnabled();
  });

  it('exporte toutes les tables et les originaux sans transmettre un filtre de consultation', async () => {
    const user = show();
    await user.click(screen.getByText('Détails sur les formats et la restauration'));
    expect(screen.getByText(/indépendamment du membre, de la date, des connaissances K/)).toBeVisible();
    await user.click(screen.getByRole('button', { name: 'Exporter une archive CSV intégrale' }));
    await screen.findByText('L’archive CSV intégrale a été créée avec les originaux documentaires.');
    expect(api.exportCsvArchive).toHaveBeenCalledExactlyOnceWith('/test/csv.zip');
    expect(screen.getByText('62 tables · 410 lignes')).toBeVisible();
    expect(screen.getByText('Schéma 6 · séquence 52')).toBeVisible();
    expect(screen.getByText('export-hash')).toBeVisible();
    expect(api.activateRestore).not.toHaveBeenCalled();
  });

  it('annule le choix de destination sans exporter ni annoncer de réussite', async () => {
    vi.mocked(api.chooseCsvExportDestination).mockResolvedValue(null);
    const user = show();
    await user.click(screen.getByRole('button', { name: 'Exporter une archive CSV intégrale' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Exporter une archive CSV intégrale' })).toBeEnabled());
    expect(api.exportCsvArchive).not.toHaveBeenCalled();
    expect(screen.queryByRole('heading', { name: 'Dernier export intégral de cette session' })).not.toBeInTheDocument();
  });

  it('distingue le XLSX de consultation des ZIP contenant les originaux', async () => {
    const user = show();
    await user.click(screen.getByRole('button', { name: 'Exporter un classeur XLSX complet' }));
    await screen.findByText(/Le classeur XLSX complet a été créé pour consultation/);
    expect(api.exportXlsx).toHaveBeenCalledExactlyOnceWith('/test/consultation.xlsx');
    expect(screen.getByText('XLSX de consultation')).toBeVisible();
    await user.click(screen.getByText('Détails sur les formats et la restauration'));
    expect(screen.getByText(/Le XLSX est un format de consultation, sans import XLSX/)).toBeVisible();
    expect(api.exportCsvArchive).not.toHaveBeenCalled();
  });

  it('présente une reconstruction vérifiée sans remplacer le coffre et laisse annuler', async () => {
    const user = await analyze();
    expect(api.inspectCsvImport).toHaveBeenCalledExactlyOnceWith('/test/source-csv.zip');
    expect(screen.getByText('Données reconstruites')).toBeVisible();
    expect(screen.getByText(/Une base vierge a été reconstruite et vérifiée/)).toBeVisible();
    expect(screen.getByRole('checkbox', { name: /Créer une sauvegarde de mon état actuel/ })).toBeChecked();
    expect(api.activateRestore).not.toHaveBeenCalled();
    await user.click(screen.getByRole('button', { name: 'Annuler' }));
    expect(screen.queryByRole('heading', { name: 'Vérifier avant de réimporter' })).not.toBeInTheDocument();
    expect(api.activateRestore).not.toHaveBeenCalled();
  });

  it('préserve le résumé si la sauvegarde préalable est annulée', async () => {
    vi.mocked(api.chooseBackupDestination).mockResolvedValue(null);
    const user = await analyze();
    await user.click(screen.getByRole('button', { name: 'Confirmer le remplacement du coffre' }));
    await screen.findByText(/Réimportation annulée/);
    expect(screen.getByRole('heading', { name: 'Vérifier avant de réimporter' })).toBeVisible();
    expect(api.activateRestore).not.toHaveBeenCalled();
    expect(onRestored).not.toHaveBeenCalled();
  });

  it('active seulement le candidat CSV confirmé avec la sauvegarde choisie', async () => {
    const user = await analyze();
    await user.click(screen.getByRole('button', { name: 'Confirmer le remplacement du coffre' }));
    await screen.findByText(/L’archive CSV a été réimportée/);
    expect(api.chooseBackupDestination).toHaveBeenCalledExactlyOnceWith(true);
    expect(api.activateRestore).toHaveBeenCalledExactlyOnceWith('csv-candidate', '/test/préalable.zip');
    expect(onRestored).toHaveBeenCalledExactlyOnceWith();
    expect(screen.queryByRole('heading', { name: 'Vérifier avant de réimporter' })).not.toBeInTheDocument();
  });

  it('affiche le refus d’un candidat périmé sans perdre son résumé ni annoncer une importation', async () => {
    vi.mocked(api.activateRestore).mockRejectedValue({ code: 'conflict', message: 'Le coffre a changé depuis l’analyse.' });
    const user = await analyze();
    await user.click(screen.getByRole('button', { name: 'Confirmer le remplacement du coffre' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Le coffre a changé depuis l’analyse.');
    expect(screen.getByRole('heading', { name: 'Vérifier avant de réimporter' })).toBeVisible();
    expect(screen.queryByText(/L’archive CSV a été réimportée/)).not.toBeInTheDocument();
    expect(onRestored).not.toHaveBeenCalled();
  });

  it('montre un diagnostic CSV sans proposer l’activation d’un fichier invalide', async () => {
    vi.mocked(api.inspectCsvImport).mockRejectedValue({ code: 'invalid_archive', message: 'Table valuations, ligne 4, colonne amount : décimal invalide.' });
    const user = show();
    await user.click(screen.getByRole('button', { name: 'Choisir une archive CSV à analyser' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Table valuations, ligne 4, colonne amount');
    expect(screen.queryByRole('button', { name: 'Confirmer le remplacement du coffre' })).not.toBeInTheDocument();
    expect(api.activateRestore).not.toHaveBeenCalled();
  });
});
