import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../services/api';
import { emptyRegistry } from '../services/registry-types';
import type { Entity, RegistrySnapshot } from '../services/registry-types';
import type { DebtContract, DebtGuarantee, DebtMovement, DebtReport, DebtSnapshot, DebtTerms } from '../services/debt-types';
import DebtsPage from './debts';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});
const entity = (id: string, name: string, kind: 'person' | 'company' = 'person'): Entity => ({ id, revision_id: `${id}-r1`, kind, display_name: name, first_name: null, last_name: null, birth_date: null, residence_country: 'FR', preferred_currency: 'EUR', activity: '', notes: '', archived: false });
const registry: RegistrySnapshot = { ...emptyRegistry, knowledge_sequence: 18, entities: [entity('alice', 'Alice'), entity('bank', 'Créancier extérieur'), entity('company', 'Société valorisée globalement', 'company')], assets: [{ id: 'house', revision_id: 'house-r1', name: 'Bien commun', country: 'FR', category: 'real_estate', notes: '', archived: false }] };
const contract: DebtContract = { id: 'loan', revision_id: 'loan-r1', name: 'Emprunt du bien', currency: 'EUR', tracking_start: '2026-01-01', notes: 'Contrat de test', archived: false };
const terms: DebtTerms = { id: 'terms', revision_id: 'terms-r1', debt_id: contract.id, valid_from: '2026-01-01', valid_to: null, principal_status: 'complete', interest_rate: null, interest_convention: '', parties: [{ line_id: 'creditor-line', side: 'creditor', entity_id: 'bank', legal_share: { num: '1', den: '1' }, economic_share: { num: '1', den: '1' }, liability_kind: 'single', notes: '' }, { line_id: 'debtor-line', side: 'debtor', entity_id: 'alice', legal_share: { num: '1', den: '1' }, economic_share: { num: '1', den: '1' }, liability_kind: 'single', notes: '' }], notes: '' };
const movement: DebtMovement = { id: 'repayment', revision_id: 'repayment-r1', debt_id: 'loan', kind: 'principal_repayment', effective_on: '2026-03-01', principal_change: '-1000', interest_paid: '200', fees_paid: '10', unlinked_reason: 'Relevé à rapprocher', notes: '' };
const guarantee: DebtGuarantee = { id: 'guarantee', revision_id: 'guarantee-r1', debt_id: 'loan', guarantor_entity_id: 'alice', limit_amount: '50000', valid_from: '2026-01-01', valid_to: null, notes: '' };
const snapshot: DebtSnapshot = { contracts: [contract], terms: [terms], movements: [movement], receivable_adjustments: [], schedules: [], payment_allocations: [], asset_links: [], guarantees: [guarantee], coverages: [], knowledge_sequence: 18 };
const report: DebtReport = { debt_id: 'loan', date: '2026-03-31', knowledge_sequence: 18, status: 'partial', contract, terms, balance: { debt_id: 'loan', date: '2026-03-31', currency: 'EUR', principal_original: null, exact_principal: null, interest_paid_original: '200.00', exact_interest_paid: { num: '200', den: '1' }, fees_paid_original: '10.00', exact_fees_paid: { num: '10', den: '1' }, opening_known: false, source_ids: ['repayment-r1'], warnings: [] }, principal_eur: null, exact_principal_eur: null, conversion: null, parties: [], schedules: [], asset_links: [], guarantees: [guarantee], coverages: [], source_ids: ['loan-r1', 'terms-r1', 'repayment-r1'], warnings: [{ code: 'missing_opening', object_id: 'loan', message: 'L’ouverture du principal n’est pas confirmée.' }] };

async function open(knowledgeSequence: number | null = null) {
  const onChanged = vi.fn().mockResolvedValue(undefined);
  const props = { registry, context: { personId: null as string | null, date: '2026-03-31', knowledgeSequence }, onChanged, onBusyChange: vi.fn() };
  const view = render(<DebtsPage {...props} />);
  await screen.findByRole('button', { name: 'Créer un contrat de financement' });
  return { ...view, user: userEvent.setup(), props, onChanged };
}
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.listDebtsAt).mockResolvedValue(snapshot);
  vi.mocked(api.saveDebtContract).mockResolvedValue(contract);
  vi.mocked(api.saveDebtTerms).mockResolvedValue(terms);
  vi.mocked(api.saveDebtMovement).mockResolvedValue(movement);
  vi.mocked(api.saveDebtGuarantee).mockResolvedValue(guarantee);
  vi.mocked(api.saveDebtCoverage).mockResolvedValue({ id: 'coverage', revision_id: 'coverage-r1', debt_id: 'loan', company_id: 'company', side: 'debtor', valid_from: '2026-01-01', valid_to: null, included: false, source: 'Annexe du bilan', notes: '' });
  vi.mocked(api.getDebtReport).mockResolvedValue(report);
});

describe('Contrats, remboursements et engagements', () => {
  it('crée un contrat TND sans inventer de principal ni mouvement bancaire', async () => {
    const { user, onChanged } = await open();
    await user.click(screen.getByRole('button', { name: 'Créer un contrat de financement' }));
    await user.type(screen.getByLabelText('Nom du financement'), 'Prêt tunisien');
    await user.selectOptions(screen.getByLabelText('Devise du financement'), 'TND');
    fireEvent.change(screen.getByLabelText('Début du suivi du financement'), { target: { value: '2026-02-01' } });
    await user.type(screen.getByLabelText('Motif de l’enregistrement du financement'), 'Contrat signé');
    await user.click(screen.getByRole('button', { name: 'Enregistrer le financement' }));
    await waitFor(() => expect(onChanged).toHaveBeenCalledOnce());
    expect(api.saveDebtContract).toHaveBeenCalledExactlyOnceWith({ id: null, expected_revision_id: null, reason: 'Contrat signé', name: 'Prêt tunisien', currency: 'TND', tracking_start: '2026-02-01', notes: '', archived: false });
    expect(api.saveDebtMovement).not.toHaveBeenCalled();
  });

  it('garde les droits économiques inconnus distincts des droits juridiques et du bien financé', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Définir les parties du financement' }));
    await user.selectOptions(screen.getByLabelText('Contrat du financement'), 'loan');
    await user.selectOptions(screen.getByLabelText('Entité de la partie 1'), 'bank');
    await user.selectOptions(screen.getByLabelText('Entité de la partie 2'), 'alice');
    await user.type(screen.getByLabelText('Droit économique 1 — numérateur'), '1');
    await user.type(screen.getByLabelText('Droit économique 1 — dénominateur'), '1');
    await user.type(screen.getByLabelText('Droit juridique 2 — numérateur'), '1');
    await user.type(screen.getByLabelText('Droit juridique 2 — dénominateur'), '1');
    await user.selectOptions(screen.getByLabelText('Portée juridique de la partie 1'), 'single');
    await user.selectOptions(screen.getByLabelText('Portée juridique de la partie 2'), 'joint_and_several');
    await user.type(screen.getByLabelText('Motif de l’enregistrement du financement'), 'Allocation à qualifier');
    await user.click(screen.getByRole('button', { name: 'Enregistrer le financement' }));
    await waitFor(() => expect(api.saveDebtTerms).toHaveBeenCalledOnce());
    expect(api.saveDebtTerms).toHaveBeenCalledWith(expect.objectContaining({ principal_status: 'unknown', interest_rate: null, parties: [expect.objectContaining({ side: 'creditor', entity_id: 'bank', economic_share: { num: '1', den: '1' }, legal_share: null }), expect.objectContaining({ side: 'debtor', entity_id: 'alice', economic_share: null, legal_share: { num: '1', den: '1' }, liability_kind: 'joint_and_several' })] }));
    expect(api.saveDebtAssetLink).not.toHaveBeenCalled();
  });

  it('transmet principal, intérêts et frais séparément en texte exact', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Mouvements et échéances' }));
    await user.click(screen.getByRole('button', { name: 'Enregistrer un mouvement de financement' }));
    await user.selectOptions(screen.getByLabelText('Contrat du financement'), 'loan');
    await user.selectOptions(screen.getByLabelText('Nature du mouvement de financement'), 'principal_repayment');
    await user.type(screen.getByLabelText('Variation signée du principal'), '-1000,00000001');
    await user.clear(screen.getByLabelText('Intérêts payés'));
    await user.type(screen.getByLabelText('Intérêts payés'), '200');
    await user.clear(screen.getByLabelText('Frais payés'));
    await user.type(screen.getByLabelText('Frais payés'), '10');
    await user.type(screen.getByLabelText('Motif d’absence de rapprochement bancaire'), 'Pièce à rapprocher');
    await user.type(screen.getByLabelText('Motif de l’enregistrement du financement'), 'Remboursement réel');
    await user.click(screen.getByRole('button', { name: 'Enregistrer le financement' }));
    await waitFor(() => expect(api.saveDebtMovement).toHaveBeenCalledOnce());
    expect(api.saveDebtMovement).toHaveBeenCalledExactlyOnceWith({ id: null, expected_revision_id: null, reason: 'Remboursement réel', debt_id: 'loan', kind: 'principal_repayment', effective_on: '2026-03-31', principal_change: '-1000,00000001', interest_paid: '200', fees_paid: '10', unlinked_reason: 'Pièce à rapprocher', notes: '' });
  });

  it('conserve le montant et la révision attendue lorsqu’un mouvement lié refuse une correction isolée', async () => {
    vi.mocked(api.saveDebtMovement).mockRejectedValue({ message: 'Corrigez aussi les liens de trésorerie dans la même opération.' });
    const { user, onChanged } = await open();
    await user.click(screen.getByRole('button', { name: 'Mouvements et échéances' }));
    await user.click(screen.getByRole('button', { name: 'Modifier le mouvement de Emprunt du bien du 2026-03-01' }));
    await user.clear(screen.getByLabelText('Frais payés'));
    await user.type(screen.getByLabelText('Frais payés'), '12');
    await user.type(screen.getByLabelText('Motif de l’enregistrement du financement'), 'Frais corrigés');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la correction du financement' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Corrigez aussi les liens de trésorerie');
    expect(screen.getByLabelText('Frais payés')).toHaveValue('12');
    expect(api.saveDebtMovement).toHaveBeenCalledWith(expect.objectContaining({ id: 'repayment', expected_revision_id: 'repayment-r1', fees_paid: '12' }));
    expect(onChanged).not.toHaveBeenCalled();
  });

  it('présente une ouverture manquante comme inconnue et une garantie hors dette certaine', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Contrôler un financement' }));
    await user.selectOptions(screen.getByLabelText('Financement à contrôler'), 'loan');
    await user.click(screen.getByRole('button', { name: 'Calculer l’encours du financement' }));
    expect(await screen.findByText('Principal restant contractuel')).toBeInTheDocument();
    expect(screen.getByText('Principal restant contractuel').nextElementSibling).toHaveTextContent('Non calculable');
    expect(screen.getByText('Engagements conditionnels, hors dette certaine')).toBeInTheDocument();
    expect(screen.getByText(/50000 EUR/)).toBeInTheDocument();
    expect(api.getDebtReport).toHaveBeenCalledExactlyOnceWith({ debt_id: 'loan', date: '2026-03-31', knowledge_sequence: null });
  });

  it('transmet K=0 et désactive toutes les commandes de mutation historiques', async () => {
    const { user } = await open(0);
    expect(api.listDebtsAt).toHaveBeenCalledWith(0);
    expect(screen.getByRole('button', { name: 'Créer un contrat de financement' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Définir les parties du financement' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Biens et garanties' }));
    expect(screen.getByRole('button', { name: 'Enregistrer une garantie conditionnelle' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Contrôler un financement' }));
    await user.selectOptions(screen.getByLabelText('Financement à contrôler'), 'loan');
    await user.click(screen.getByRole('button', { name: 'Calculer l’encours du financement' }));
    await waitFor(() => expect(api.getDebtReport).toHaveBeenCalledWith({ debt_id: 'loan', date: '2026-03-31', knowledge_sequence: 0 }));
    expect(api.saveDebtContract).not.toHaveBeenCalled();
  });

  it('ne garde aucun ancien contrat si le chargement d’une autre connaissance échoue', async () => {
    const { rerender, props } = await open();
    expect(screen.getAllByText('Emprunt du bien').length).toBeGreaterThan(0);
    vi.mocked(api.listDebtsAt).mockRejectedValueOnce({ message: 'Séquence indisponible' });
    rerender(<DebtsPage {...props} context={{ ...props.context, knowledgeSequence: 2 }} />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Séquence indisponible');
    expect(screen.queryByText('Emprunt du bien')).not.toBeInTheDocument();
  });

  it('exige une couverture explicite de société et transmet une exclusion déclarée sans la déduire du bien', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Biens et garanties' }));
    await user.click(screen.getByRole('button', { name: 'Déclarer une couverture de société' }));
    await user.selectOptions(screen.getByLabelText('Contrat du financement'), 'loan');
    await user.selectOptions(screen.getByLabelText('Société valorisée globalement'), 'company');
    await user.type(screen.getByLabelText('Source de la couverture déclarée'), 'Annexe du bilan');
    await user.type(screen.getByLabelText('Motif de l’enregistrement du financement'), 'Couverture vérifiée');
    await user.click(screen.getByRole('button', { name: 'Enregistrer le financement' }));
    expect(api.saveDebtCoverage).not.toHaveBeenCalled();
    await user.selectOptions(screen.getByLabelText('Présence dans la valeur globale'), 'false');
    await user.click(screen.getByRole('button', { name: 'Enregistrer le financement' }));
    await waitFor(() => expect(api.saveDebtCoverage).toHaveBeenCalledOnce());
    expect(api.saveDebtCoverage).toHaveBeenCalledWith(expect.objectContaining({ debt_id: 'loan', company_id: 'company', side: 'debtor', included: false, source: 'Annexe du bilan' }));
    expect(api.saveDebtAssetLink).not.toHaveBeenCalled();
  });

  it('archive sans envoyer le champ interne de révision dans un input strict', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Archiver' }));
    await user.type(screen.getByLabelText('Motif de la modification du financement'), 'Contrat clos hors nouvelles saisies');
    await user.click(screen.getByRole('button', { name: 'Confirmer la modification du financement' }));
    await waitFor(() => expect(api.saveDebtContract).toHaveBeenCalledOnce());
    expect(api.saveDebtContract).toHaveBeenCalledExactlyOnceWith({ id: 'loan', expected_revision_id: 'loan-r1', reason: 'Contrat clos hors nouvelles saisies', name: contract.name, currency: 'EUR', tracking_start: '2026-01-01', notes: 'Contrat de test', archived: true });
  });
});
