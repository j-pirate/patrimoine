import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../services/api';
import type { DocumentLibrarySnapshot, DocumentRecord } from '../services/documents-types';
import type { ViewContext } from './registry-ui';
import DocumentsPage from './documents';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});
const current: DocumentRecord = { id: 'document', revision_id: 'document-r2', original_name: 'Contrat.pdf', mime_type: 'application/pdf', description: 'Pièce signée du bail', status: 'active', sha256: 'current-sha', size_bytes: 1250, added_at: '2026-01-02T10:00:00Z', recorded_at: '2026-02-02T10:00:00Z' };
const old: DocumentRecord = { ...current, revision_id: 'document-r1', original_name: 'Ancien.pdf', description: 'Ancien original conservé', sha256: 'old-sha', recorded_at: '2026-01-02T10:00:00Z' };
const library: DocumentLibrarySnapshot = {
  documents: [current], versions: [old, current], knowledge_sequence: 42,
  targets: [{ id: 'lease', kind: 'lease', label: 'Bail des Lilas', retired: false }, { id: 'asset', kind: 'asset', label: 'Appartement des Lilas', retired: false }, { id: 'person', kind: 'entity', label: 'Alice', retired: false }, { id: 'retired', kind: 'debt_contract', label: 'Ancien prêt', retired: true }],
  links: [{ id: 'link-lease', revision_id: 'link-lease-r', document_id: current.id, target_object_id: 'lease', role: 'Contrat de location', valid_from: '2026-01-01', valid_to: null }, { id: 'link-asset', revision_id: 'link-asset-r', document_id: current.id, target_object_id: 'asset', role: 'Justificatif du bien', valid_from: '2026-04-01', valid_to: null }],
};
const context: ViewContext = { personId: null, date: '2026-03-31', knowledgeSequence: null };
const empty: DocumentLibrarySnapshot = { documents: [], versions: [], links: [], targets: [], knowledge_sequence: 0 };
function props(view = context) { return { context: view, onChanged: vi.fn().mockResolvedValue(undefined), onBusyChange: vi.fn() }; }
async function open(view = context) {
  const p = props(view); const result = render(<DocumentsPage {...p} />); const user = userEvent.setup();
  await screen.findByRole('heading', { name: 'Documents du coffre' });
  return { user, ...result, p };
}
const documentCard = () => screen.getByRole('region', { name: 'Document Contrat.pdf' });
const reason = () => screen.getByLabelText('Motif de la modification documentaire');
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.listDocumentLibraryAt).mockResolvedValue(library);
  vi.mocked(api.importDocument).mockResolvedValue({ id: current.id, original_name: current.original_name, mime_type: current.mime_type, size_bytes: current.size_bytes, sha256: current.sha256, added_at: current.added_at });
  vi.mocked(api.saveDocumentMetadata).mockResolvedValue(current);
  vi.mocked(api.replaceDocument).mockResolvedValue(current);
  vi.mocked(api.saveDocumentLink).mockResolvedValue(library.links[0]);
  vi.mocked(api.removeDocumentLink).mockResolvedValue(undefined);
  vi.mocked(api.openDocumentRevision).mockResolvedValue(undefined);
  vi.mocked(api.exportDocumentRevision).mockResolvedValue(undefined);
});

describe('Bibliothèque documentaire et versions', () => {
  it('importe les octets choisis avec leur description puis relit la bibliothèque', async () => {
    vi.mocked(api.listDocumentLibraryAt).mockResolvedValueOnce(empty).mockResolvedValue(library);
    vi.mocked(api.chooseDocumentImport).mockResolvedValue('/test/source.pdf');
    const { user, p } = await open();
    await screen.findByText('Votre espace documentaire est vide');
    await user.type(screen.getByLabelText('Description du document à importer (facultative)'), 'Document signé et original');
    await user.click(screen.getByRole('button', { name: 'Importer un document' }));
    await screen.findByRole('heading', { name: current.original_name });
    expect(api.importDocument).toHaveBeenCalledExactlyOnceWith('/test/source.pdf', 'Document signé et original');
    expect(p.onChanged).toHaveBeenCalledOnce();
    expect(screen.getByLabelText('Description du document à importer (facultative)')).toHaveValue('');
    expect(screen.getByText('Le document a été importé dans le coffre.')).toBeVisible();
  });

  it('conserve le chemin natif direct pour un import sans description et respecte une annulation', async () => {
    vi.mocked(api.chooseDocumentImport).mockResolvedValueOnce(null).mockResolvedValue('/test/source.pdf');
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Importer un document' }));
    await screen.findByText('Choix de fichier annulé ; la saisie est conservée.');
    expect(api.importDocument).not.toHaveBeenCalled();
    await user.click(screen.getByRole('button', { name: 'Importer un document' }));
    await waitFor(() => expect(api.importDocument).toHaveBeenCalledExactlyOnceWith('/test/source.pdf'));
  });

  it('ouvre uniquement la révision demandée et exporte les octets d’une ancienne version', async () => {
    vi.mocked(api.chooseDocumentDestination).mockResolvedValue('/test/ancien-original.pdf');
    const { user } = await open();
    await user.click(within(documentCard()).getByRole('button', { name: 'Ouvrir le document' }));
    await screen.findByText('L’ouverture a été demandée à l’application par défaut.');
    expect(api.openDocumentRevision).toHaveBeenCalledExactlyOnceWith('document', 'document-r2');
    await user.click(screen.getByText('Versions antérieures de ce document'));
    await user.click(screen.getByRole('button', { name: 'Exporter cette version' }));
    await screen.findByText('Une copie de « Ancien.pdf » a été exportée.');
    expect(api.chooseDocumentDestination).toHaveBeenCalledWith('Ancien.pdf');
    expect(api.exportDocumentRevision).toHaveBeenCalledExactlyOnceWith('document', 'document-r1', '/test/ancien-original.pdf');
  });

  it('exporte la version courante avec le libellé natif conservé', async () => {
    vi.mocked(api.chooseDocumentDestination).mockResolvedValue('/test/copie.pdf');
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Exporter une copie de Contrat.pdf' }));
    await screen.findByText('Une copie de « Contrat.pdf » a été exportée.');
    expect(api.exportDocumentRevision).toHaveBeenCalledExactlyOnceWith('document', 'document-r2', '/test/copie.pdf');
  });

  it('remplace l’original avec la révision attendue et garde la saisie après conflit', async () => {
    vi.mocked(api.chooseDocumentReplacement).mockResolvedValue('/test/nouveau.pdf');
    vi.mocked(api.replaceDocument).mockRejectedValue({ message: 'Cette révision a déjà été remplacée.' });
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Remplacer l’original' }));
    await user.clear(screen.getByLabelText('Description du document'));
    await user.type(screen.getByLabelText('Description du document'), 'Nouvelle pièce signée');
    await user.type(reason(), 'Remplacement avec signature finale');
    await user.click(screen.getByRole('button', { name: 'Choisir et enregistrer le nouvel original' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Cette révision a déjà été remplacée.');
    expect(api.replaceDocument).toHaveBeenCalledExactlyOnceWith({ id: 'document', expected_revision_id: 'document-r2', reason: 'Remplacement avec signature finale', description: 'Nouvelle pièce signée' }, '/test/nouveau.pdf');
    expect(screen.getByLabelText('Description du document')).toHaveValue('Nouvelle pièce signée');
    expect(reason()).toHaveValue('Remplacement avec signature finale');
    expect(api.saveDocumentMetadata).not.toHaveBeenCalled();
  });

  it('exige une confirmation pour retirer le document sans retirer ses autres associations', async () => {
    vi.mocked(api.listDocumentLibraryAt).mockResolvedValueOnce(library).mockResolvedValue({ ...library, documents: [{ ...current, revision_id: 'document-r3', status: 'deleted' }] });
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Supprimer le document' }));
    await user.type(reason(), 'Pièce remplacée, conservation de son historique');
    const save = screen.getByRole('button', { name: 'Enregistrer la description et le statut' });
    expect(save).toBeDisabled();
    await user.click(screen.getByRole('checkbox', { name: 'Confirmer le retrait du document du classement courant' }));
    await user.click(save);
    await screen.findByText('Aucun document ne correspond aux filtres.');
    expect(api.saveDocumentMetadata).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({ id: 'document', expected_revision_id: 'document-r2', status: 'deleted', confirmed: true }));
    expect(api.removeDocumentLink).not.toHaveBeenCalled();
    await user.selectOptions(screen.getByLabelText('Statut des documents'), '');
    await screen.findByRole('heading', { name: current.original_name });
    expect(screen.getByRole('button', { name: 'Rétablir le document' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Remplacer l’original' })).toBeDisabled();
    expect(screen.getByRole('row', { name: /Bail des Lilas/ })).toBeVisible();
  });

  it('archive par une révision motivée en gardant la description exacte', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Modifier la description ou le statut' }));
    await user.selectOptions(screen.getByLabelText('Nouveau statut du document'), 'archived');
    await user.type(reason(), 'Classement du dossier terminé');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la description et le statut' }));
    await waitFor(() => expect(api.saveDocumentMetadata).toHaveBeenCalledExactlyOnceWith({ id: 'document', expected_revision_id: 'document-r2', reason: 'Classement du dossier terminé', description: current.description, status: 'archived', confirmed: false }));
  });

  it('associe la même pièce à un autre objet avec dates et valeurs nulles explicites', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Associer un objet' }));
    expect(screen.getByLabelText('Début de l’association')).toBeRequired();
    expect(screen.getByLabelText('Début de l’association')).toHaveValue(context.date);
    const editor = screen.getByRole('region', { name: 'Association documentaire' });
    expect(within(editor).queryByRole('option', { name: /Ancien prêt/ })).not.toBeInTheDocument();
    await user.selectOptions(screen.getByLabelText('Objet métier à associer'), 'person');
    fireEvent.change(screen.getByLabelText('Début de l’association'), { target: { value: '2026-03-01' } });
    await user.type(reason(), 'Pièce partagée avec la personne');
    await user.click(screen.getByRole('button', { name: 'Enregistrer l’association documentaire' }));
    await waitFor(() => expect(api.saveDocumentLink).toHaveBeenCalledExactlyOnceWith({ id: null, expected_revision_id: null, reason: 'Pièce partagée avec la personne', document_id: 'document', target_object_id: 'person', role: null, valid_from: '2026-03-01', valid_to: null }));
    expect(api.importDocument).not.toHaveBeenCalled();
  });

  it('corrige un rôle sans déplacer une association vers un autre objet', async () => {
    const { user } = await open();
    await user.click(within(screen.getByRole('row', { name: /Bail des Lilas/ })).getByRole('button', { name: 'Modifier l’association' }));
    expect(screen.getByLabelText('Objet métier à associer')).toBeDisabled();
    await user.clear(screen.getByLabelText('Rôle de l’association (facultatif)'));
    await user.type(screen.getByLabelText('Rôle de l’association (facultatif)'), 'Annexe de bail signée');
    await user.type(reason(), 'Qualification du rôle');
    await user.click(screen.getByRole('button', { name: 'Enregistrer l’association documentaire' }));
    await waitFor(() => expect(api.saveDocumentLink).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({ id: 'link-lease', expected_revision_id: 'link-lease-r', target_object_id: 'lease', role: 'Annexe de bail signée' })));
  });

  it('retire seulement l’association choisie avec son motif et sa révision', async () => {
    const { user } = await open();
    await user.click(within(screen.getByRole('row', { name: /Bail des Lilas/ })).getByRole('button', { name: 'Retirer l’association' }));
    await user.type(screen.getByLabelText('Motif du retrait de l’association'), 'Association devenue inutile');
    await user.click(screen.getByRole('button', { name: 'Confirmer le retrait de l’association' }));
    await screen.findByText('L’association a été retirée ; le document et ses autres associations sont conservés.');
    expect(api.removeDocumentLink).toHaveBeenCalledExactlyOnceWith('link-lease', 'link-lease-r', 'Association devenue inutile');
    expect(api.saveDocumentMetadata).not.toHaveBeenCalled();
  });

  it('filtre les associations sur leur période civile sans masquer leur source dans la bibliothèque', async () => {
    const { user, rerender, p } = await open();
    expect(screen.getByText('Hors de la période choisie')).toBeVisible();
    await user.selectOptions(screen.getByLabelText('Type d’objet associé à la date'), 'asset');
    await screen.findByText('Aucun document ne correspond aux filtres.');
    rerender(<DocumentsPage {...p} context={{ ...context, date: '2026-05-01' }} />);
    await screen.findByRole('heading', { name: current.original_name });
    expect(api.listDocumentLibraryAt).toHaveBeenCalledTimes(1);
  });

  it('garde K=0 en lecture seule tout en permettant l’export d’une version connue', async () => {
    vi.mocked(api.chooseDocumentDestination).mockResolvedValue('/test/copie.pdf');
    const { user } = await open({ ...context, knowledgeSequence: 0 });
    expect(api.listDocumentLibraryAt).toHaveBeenCalledExactlyOnceWith(0);
    expect(screen.getByRole('button', { name: 'Importer un document' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Supprimer le document' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Associer un objet' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Exporter une copie de Contrat.pdf' }));
    await waitFor(() => expect(api.exportDocumentRevision).toHaveBeenCalledExactlyOnceWith('document', 'document-r2', '/test/copie.pdf'));
    expect(api.saveDocumentMetadata).not.toHaveBeenCalled();
  });

  it('efface le snapshot courant si le chargement d’un autre K échoue', async () => {
    const { rerender, p } = await open();
    expect(screen.getByRole('heading', { name: current.original_name })).toBeVisible();
    vi.mocked(api.listDocumentLibraryAt).mockRejectedValue({ message: 'Connaissances indisponibles.' });
    rerender(<DocumentsPage {...p} context={{ ...context, knowledgeSequence: 99 }} />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Connaissances indisponibles.');
    expect(screen.queryByRole('heading', { name: current.original_name })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Importer un document' })).not.toBeInTheDocument();
  });

  it('ignore une ancienne réponse documentaire arrivée après le snapshot demandé ensuite', async () => {
    const { rerender, p } = await open();
    let resolveOld!: (value: DocumentLibrarySnapshot) => void;
    vi.mocked(api.listDocumentLibraryAt).mockImplementation(k => k === 1 ? new Promise(resolve => { resolveOld = resolve; }) : Promise.resolve(empty));
    rerender(<DocumentsPage {...p} context={{ ...context, knowledgeSequence: 1 }} />);
    await waitFor(() => expect(api.listDocumentLibraryAt).toHaveBeenCalledWith(1));
    rerender(<DocumentsPage {...p} context={{ ...context, knowledgeSequence: 2 }} />);
    await screen.findByText('Votre espace documentaire est vide');
    await act(async () => { resolveOld(library); });
    expect(screen.queryByRole('heading', { name: current.original_name })).not.toBeInTheDocument();
  });
});
