import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import GlobalSearch from './global-search';
import { api } from '../services/api';
import type { DocumentLibrarySnapshot } from '../services/documents-types';
import type { TreasurySnapshot } from '../services/treasury-types';
import { emptyRegistry } from '../services/registry-types';

vi.mock('../services/api', async importOriginal => {
  const original = await importOriginal<typeof import('../services/api')>();
  return { ...original, api: { ...original.api, listDocumentLibraryAt: vi.fn(), listTreasuryAt: vi.fn() } };
});
const library: DocumentLibrarySnapshot = {
  knowledge_sequence: 8, versions: [], links: [],
  targets: [{ id: 'lot-ancien', kind: 'metal_lot', label: 'Pièces argent été', retired: true }],
  documents: [{ id: 'doc-1', revision_id: 'r1', original_name: 'contrat.pdf', mime_type: 'application/pdf', description: 'Bail de la résidence Émilie', status: 'active', sha256: '1234567890', size_bytes: 12, added_at: '2026-01-01', recorded_at: '2026-01-01' }],
};
const registry = { ...emptyRegistry, knowledge_sequence: 8 };
const context = { personId: 'alice', date: '2026-11-12', knowledgeSequence: null };
function setup() {
  const onNavigate = vi.fn();
  const view = render(<GlobalSearch registry={registry} context={context} disabled={false} onNavigate={onNavigate} />);
  return { ...view, onNavigate };
}
beforeEach(() => {
  vi.resetAllMocks(); vi.mocked(api.listDocumentLibraryAt).mockResolvedValue(library);
  vi.mocked(api.listTreasuryAt).mockImplementation(async k => ({ accounts: [], transactions: [], reconciliations: [], cash_links: [], deposits: [], knowledge_sequence: k ?? 8 }));
});

describe('Recherche globale au K sélectionné', () => {
  it('distingue une caisse physique d’un compte bancaire pour ouvrir le bon module', async () => {
    const { onNavigate } = setup();
    vi.mocked(api.listDocumentLibraryAt).mockResolvedValue({ ...library, targets: [{ id: 'cash-account', kind: 'treasury_account', label: 'treasury_account · cash-account', retired: false }] });
    vi.mocked(api.listTreasuryAt).mockResolvedValue({ accounts: [{ id: 'cash-account', revision_id: 'r-cash', asset_id: 'asset-cash', kind: 'cash', currency: 'TND', tracking_start: '2020-01-01', legal_owner_entity_id: 'alice', bank_name: '', reference: '', country: 'TN', storage_location: 'Coffre de Tunis', notes: '', archived: false }], transactions: [], reconciliations: [], cash_links: [], deposits: [], knowledge_sequence: 8 });
    fireEvent.click(screen.getByRole('button', { name: 'Rechercher dans le coffre' }));
    await screen.findByText('Saisir au moins deux caractères.');
    fireEvent.change(screen.getByLabelText('Nom, description documentaire ou identifiant'), { target: { value: 'Tunis' } });
    fireEvent.click(screen.getByRole('button', { name: 'Ouvrir le module Espèces et devises' }));
    expect(onNavigate).toHaveBeenCalledWith('cash-accounts');
    expect(api.listTreasuryAt).toHaveBeenCalledWith(8);
  });
  it.each([
    ['cash-receipt', 'cash-accounts', 'Espèces et devises'],
    ['cash-count', 'cash-accounts', 'Espèces et devises'],
    ['cash-link', 'cash-accounts', 'Espèces et devises'],
    ['mixed-receipt', 'bank', 'Comptes bancaires'],
    ['bank-count', 'bank', 'Comptes bancaires'],
  ])('SEARCH-01 ouvre %s dans son module correspondant', async (id, page, label) => {
    const cash = { id: 'cash', revision_id: 'cash-r1', asset_id: 'cash-asset', kind: 'cash', currency: 'EUR', tracking_start: '2026-01-01', legal_owner_entity_id: 'alice', bank_name: '', reference: '', country: 'FR', storage_location: 'Coffre privé', notes: '', archived: false };
    const treasury: TreasurySnapshot = {
      accounts: [cash, { ...cash, id: 'bank', kind: 'bank' }],
      transactions: [
        { id: 'cash-receipt', revision_id: 'cash-receipt-r1', kind: 'receipt', effective_on: '2026-02-01', label: 'Recette en espèces', reference: '', notes: '', entries: [{ line_id: 'line', account_id: 'cash', amount: '10', role: 'principal' }] },
        { id: 'mixed-receipt', revision_id: 'mixed-receipt-r1', kind: 'transfer', effective_on: '2026-02-01', label: 'Transfert banque et caisse', reference: '', notes: '', entries: [{ line_id: 'out', account_id: 'cash', amount: '-10', role: 'principal' }, { line_id: 'in', account_id: 'bank', amount: '10', role: 'principal' }] },
      ],
      reconciliations: ['cash', 'bank'].map(account => ({ id: `${account}-count`, revision_id: `${account}-count-r1`, account_id: account, effective_on: '2026-02-01', statement_balance: '10', statement_reference: 'Comptage', document_id: null, notes: '' })),
      cash_links: [{ id: 'cash-link', revision_id: 'cash-link-r1', source_kind: 'rent_payment', source_id: 'payment', source_revision_id: 'payment-r1', transaction_revision_id: 'cash-receipt-r1', line_id: 'line', amount: '10', component: 'rent', side: 'receipt', notes: '' }], deposits: [], knowledge_sequence: 8,
    };
    vi.mocked(api.listTreasuryAt).mockResolvedValue(treasury);
    vi.mocked(api.listDocumentLibraryAt).mockResolvedValue({ ...library, targets: [{ id, label: id, retired: false, kind: id.endsWith('count') ? 'account_reconciliation' : id.endsWith('link') ? 'cash_link' : 'treasury_transaction' }] });
    const { onNavigate } = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Rechercher dans le coffre' }));
    await screen.findByText('Saisir au moins deux caractères.');
    fireEvent.change(screen.getByLabelText('Nom, description documentaire ou identifiant'), { target: { value: id } });
    fireEvent.click(screen.getByRole('button', { name: `Ouvrir le module ${label}` }));
    expect(onNavigate).toHaveBeenCalledExactlyOnceWith(page);
  });

  it('REG-05 conserve le focus de la destination après ouverture d’un résultat', async () => {
    render(<><h1 tabIndex={-1}>Destination</h1><GlobalSearch registry={registry} context={context} disabled={false} onNavigate={() => requestAnimationFrame(() => screen.getByRole('heading', { name: 'Destination' }).focus())} /></>);
    const toggle = screen.getByRole('button', { name: 'Rechercher dans le coffre' });
    fireEvent.click(toggle);
    await screen.findByText('Saisir au moins deux caractères.');
    fireEvent.change(screen.getByLabelText('Nom, description documentaire ou identifiant'), { target: { value: 'contrat' } });
    fireEvent.click(screen.getByRole('button', { name: 'Ouvrir le module Documents' }));
    await waitFor(() => expect(screen.getByRole('heading', { name: 'Destination' })).toHaveFocus());
    expect(toggle).not.toHaveFocus();
  });

  it('REG-05 rend le focus au bouton après une fermeture simple par Échap', async () => {
    setup();
    const toggle = screen.getByRole('button', { name: 'Rechercher dans le coffre' });
    fireEvent.click(toggle);
    await screen.findByText('Saisir au moins deux caractères.');
    fireEvent.keyDown(screen.getByLabelText('Nom, description documentaire ou identifiant'), { key: 'Escape' });
    await waitFor(() => expect(toggle).toHaveFocus());
    expect(screen.queryByRole('region', { name: 'Recherche globale dans le coffre' })).not.toBeInTheDocument();
  });
  it('charge à la demande et trouve une description avec accents indépendamment du membre choisi', async () => {
    setup();
    expect(api.listDocumentLibraryAt).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Rechercher dans le coffre' }));
    await screen.findByText('Saisir au moins deux caractères.');
    fireEvent.change(screen.getByLabelText('Nom, description documentaire ou identifiant'), { target: { value: 'residence emilie' } });
    expect(screen.getByText('contrat.pdf')).toBeVisible();
    expect(screen.getByText(/elle ne se limite pas au membre choisi/)).toBeVisible();
    expect(api.listDocumentLibraryAt).toHaveBeenCalledExactlyOnceWith(8);
  });
  it('marque les objets retirés et ouvre leur module réel', async () => {
    const { onNavigate } = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Rechercher dans le coffre' }));
    await screen.findByText('Saisir au moins deux caractères.');
    fireEvent.change(screen.getByLabelText('Nom, description documentaire ou identifiant'), { target: { value: 'lot-ancien' } });
    expect(screen.getByText('Or et argent · retiré')).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Ouvrir le module Or et argent' }));
    expect(onNavigate).toHaveBeenCalledWith('metals');
    expect(screen.queryByLabelText('Recherche globale dans le coffre')).not.toBeInTheDocument();
  });
  it('ignore une ancienne réponse après changement de K et respecte explicitement K=0', async () => {
    let resolveOld!: (result: DocumentLibrarySnapshot) => void;
    vi.mocked(api.listDocumentLibraryAt).mockImplementationOnce(() => new Promise(resolve => { resolveOld = resolve; }));
    vi.mocked(api.listDocumentLibraryAt).mockResolvedValueOnce({ ...library, knowledge_sequence: 0, documents: [], targets: [] });
    const { rerender, onNavigate } = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Rechercher dans le coffre' }));
    await waitFor(() => expect(api.listDocumentLibraryAt).toHaveBeenCalledWith(8));
    rerender(<GlobalSearch registry={registry} context={{ ...context, knowledgeSequence: 0 }} disabled={false} onNavigate={onNavigate} />);
    await waitFor(() => expect(api.listDocumentLibraryAt).toHaveBeenCalledWith(0));
    fireEvent.change(screen.getByLabelText('Nom, description documentaire ou identifiant'), { target: { value: 'contrat' } });
    await act(async () => { resolveOld(library); });
    expect(await screen.findByText('0 résultat.')).toBeVisible();
    expect(screen.queryByText('contrat.pdf')).not.toBeInTheDocument();
  });
  it('refuse une réponse à un autre K et permet un rechargement', async () => {
    vi.mocked(api.listDocumentLibraryAt).mockResolvedValueOnce({ ...library, knowledge_sequence: 9 });
    setup();
    fireEvent.click(screen.getByRole('button', { name: 'Rechercher dans le coffre' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('ne correspondent pas');
    fireEvent.click(screen.getByRole('button', { name: 'Recharger la recherche' }));
    await screen.findByText('Saisir au moins deux caractères.');
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
  it('affiche le profil daté au lieu du seul libellé courant', async () => {
    const person = { id: 'alice', revision_id: 'current', kind: 'person' as const, display_name: 'Alice actuelle', first_name: 'Alice', last_name: null, birth_date: null, residence_country: 'FR', preferred_currency: 'EUR', activity: '', notes: '', archived: false };
    vi.mocked(api.listDocumentLibraryAt).mockResolvedValue({ ...library, targets: [{ id: 'alice', kind: 'entity', label: 'Alice actuelle', retired: false }] });
    render(<GlobalSearch registry={{ ...registry, entities: [person], profile_revisions: [{ ...person, revision_id: 'old', display_name: 'Alice historique' }], profile_periods: [{ id: 'profile', revision_id: 'period', target_kind: 'entity', target_id: 'alice', profile_revision_id: 'old', valid_from: '2020-01-01', valid_to: null }] }} context={context} disabled={false} onNavigate={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: 'Rechercher dans le coffre' }));
    await screen.findByText('Saisir au moins deux caractères.');
    fireEvent.change(screen.getByLabelText('Nom, description documentaire ou identifiant'), { target: { value: 'historique' } });
    expect(screen.getByText('Alice historique')).toBeVisible();
  });
});
