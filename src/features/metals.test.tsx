import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../services/api';
import { emptyRegistry } from '../services/registry-types';
import type { RegistrySnapshot } from '../services/registry-types';
import type { MetalLot, MetalMovement, MetalReport, MetalSnapshot } from '../services/metals-types';
import type { TreasurySnapshot } from '../services/treasury-types';
import MetalsPage from './metals';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});
const one = { num: '1', den: '1' };
const registry: RegistrySnapshot = { ...emptyRegistry, knowledge_sequence: 42,
  entities: ['owner', 'recipient'].map(id => ({ id, revision_id: `${id}-r`, kind: 'person', display_name: id === 'owner' ? 'Alice déclarée' : 'Benoît déclaré', first_name: null, last_name: null, birth_date: null, residence_country: 'FR', preferred_currency: 'EUR', activity: '', notes: '', archived: false })),
  assets: [{ id: 'metal-asset', revision_id: 'asset-r', name: 'Or familial identifié', country: 'FR', category: 'metal', notes: '', archived: false }, { id: 'bank-asset', revision_id: 'bank-asset-r', name: 'Compte déclaré', country: 'FR', category: 'bank', notes: '', archived: false }],
  ownerships: [{ id: 'title', revision_id: 'title-r', asset_id: 'metal-asset', owner_entity_id: 'owner', legal_share: one, economic_share: one, valid_from: '2020-01-01', valid_to: null }],
};
const lot: MetalLot = { id: 'lot', revision_id: 'lot-r', asset_id: 'metal-asset', metal: 'gold', form: 'coin', designation: 'Pièces identifiées', unit_weight: '31.1034768', weight_unit: 'g', purity: '0.9', tracking_start: '2026-01-01', cost_currency: 'EUR', legal_owner_entity_id: 'owner', country: 'FR', storage_location: 'Coffre identifié', valuation_basis: 'intrinsic', notes: '', archived: false };
const opening: MetalMovement = { id: 'opening', revision_id: 'opening-r', lot_id: 'lot', kind: 'opening', effective_on: '2026-01-01', quantity_change: '2', cost_amount: '2000', trade_amount: null, trade_fees: '0', trade_currency: null, cash_entity_id: '', counterparty_entity_id: null, country: 'FR', storage_location: 'Coffre identifié', unlinked_reason: '', notes: '' };
const sale: MetalMovement = { ...opening, id: 'sale', revision_id: 'sale-r2', kind: 'sale', effective_on: '2026-03-01', quantity_change: '-1', cost_amount: null, trade_amount: '1500', trade_fees: '20', trade_currency: 'EUR', cash_entity_id: 'owner' };
const snapshot: MetalSnapshot = { lots: [lot], movements: [opening], quotes: [], premiums: [], cash_links: [], knowledge_sequence: 42 };
const treasury: TreasurySnapshot = { accounts: [{ id: 'bank', revision_id: 'bank-r', asset_id: 'bank-asset', kind: 'bank', currency: 'EUR', tracking_start: '2020-01-01', legal_owner_entity_id: 'owner', bank_name: 'Banque', reference: 'REF', country: 'FR', storage_location: '', notes: '', archived: false }], transactions: [{ id: 'bank-sale', revision_id: 'bank-sale-r3', kind: 'asset_trade', effective_on: '2026-03-01', label: 'Vente déjà au relevé', reference: '', notes: '', entries: [{ line_id: 'sale-line', account_id: 'bank', amount: '1480', role: 'principal' }] }], reconciliations: [], cash_links: [], deposits: [], knowledge_sequence: 42 };
const report: MetalReport = { lot_id: 'lot', asset_id: 'metal-asset', date: '2026-03-31', knowledge_sequence: 42, status: 'partial', valuation_basis: 'intrinsic', quantity_remaining: '2', exact_quantity: { num: '2', den: '1' }, fine_weight_g: '55.98625824', exact_fine_weight_g: { num: '874785285', den: '15625000' }, cost_currency: 'EUR', currency: 'EUR', cost_remaining_original: '2000.00', exact_cost_remaining_original: { num: '2000', den: '1' }, cost_remaining_eur: '2000.00', exact_cost_remaining_eur: { num: '2000', den: '1' }, quote: { id: 'quote', revision_id: 'quote-r', metal: 'gold', effective_on: '2026-03-01', price: '60', currency: 'EUR', weight_unit: 'g', source: 'Cours documenté', notes: '' }, premium: null, intrinsic_original: '3359.18', exact_intrinsic_original: { num: '524871171', den: '156250' }, intrinsic_eur: '3359.18', exact_intrinsic_eur: { num: '524871171', den: '156250' }, premium_eur: null, premium_original: null, exact_premium_original: null, exact_premium_eur: null, discount_eur: null, discount_original: null, exact_discount_original: null, exact_discount_eur: null, resale_eur: null, resale_original: null, exact_resale_original: null, exact_resale_eur: null, valuation_eur: '3359.18', exact_valuation_eur: { num: '524871171', den: '156250' }, realized_gain_eur: '0.00', exact_realized_gain_eur: { num: '0', den: '1' }, unrealized_gain_eur: '1359.18', exact_unrealized_gain_eur: { num: '212371171', den: '156250' }, country: 'FR', storage_location: 'Coffre identifié', cost_allocations: [], conversions: [], source_ids: ['lot-r', 'opening-r', 'quote-r'], warnings: [{ code: 'premium_missing', object_id: 'lot', message: 'La prime de revente n’est pas renseignée.' }] };
async function open(k: number | null = null) {
  const props = { registry, context: { personId: null, date: '2026-03-31', knowledgeSequence: k }, onChanged: vi.fn().mockResolvedValue(undefined), onBusyChange: vi.fn(), onNavigateRegistry: vi.fn() };
  const view = render(<MetalsPage {...props} />);
  await screen.findByRole('heading', { name: 'Lots enregistrés' });
  return { ...view, props, user: userEvent.setup() };
}
async function saleForm(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Mouvements de métal' }));
  await user.click(screen.getByRole('button', { name: 'Enregistrer un mouvement de métal' }));
  await user.selectOptions(screen.getByLabelText('Lot du mouvement'), 'lot');
  await user.selectOptions(screen.getByLabelText('Nature du mouvement de métal'), 'sale');
  await user.type(screen.getByLabelText('Variation signée de quantité'), '-1');
  await user.type(screen.getByLabelText('Prix total de la transaction avant frais'), '1500');
  await user.clear(screen.getByLabelText('Frais réels de la transaction'));
  await user.type(screen.getByLabelText('Frais réels de la transaction'), '20');
  await user.selectOptions(screen.getByLabelText('Bénéficiaire ou payeur du cash'), 'owner');
  await user.type(screen.getByLabelText('Motif de l’enregistrement métal'), 'Vente documentée de la moitié du lot');
}
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(api.listMetalsAt).mockResolvedValue(snapshot);
  vi.mocked(api.listTreasuryAt).mockResolvedValue(treasury);
  vi.mocked(api.saveMetalLot).mockResolvedValue(lot);
  vi.mocked(api.saveMetalMovement).mockResolvedValue(opening);
  vi.mocked(api.saveMetalQuote).mockResolvedValue(report.quote!);
  vi.mocked(api.postMetalMovement).mockResolvedValue(sale);
  vi.mocked(api.getMetalReport).mockResolvedValue(report);
  vi.mocked(api.transferMetalOwnership).mockResolvedValue(registry);
});

describe('Métaux physiques et leurs sources', () => {
  it('rattache le lot à son actif sans convertir les entrées exactes en nombres JS', async () => {
    const { user } = await open();
    await user.click(screen.getByText('Options avancées'));
    await user.click(screen.getByRole('button', { name: 'Rattacher un lot à un bien existant' }));
    await user.selectOptions(screen.getByLabelText('Actif du lot'), 'metal-asset');
    await user.type(screen.getByLabelText('Désignation du lot'), 'Pièces neuves');
    await user.type(screen.getByLabelText('Poids brut unitaire'), '31,1034768');
    await user.type(screen.getByLabelText('Pureté (fraction décimale entre 0 et 1)'), '0,9');
    await user.selectOptions(screen.getByLabelText('Titulaire juridique initial du lot'), 'owner');
    await user.type(screen.getByLabelText('Lieu de stockage initial'), 'Lieu documenté');
    await user.type(screen.getByLabelText('Motif de l’enregistrement métal'), 'Lot identifié');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source métal' }));
    await waitFor(() => expect(api.saveMetalLot).toHaveBeenCalledOnce());
    expect(api.saveMetalLot).toHaveBeenCalledWith(expect.objectContaining({ id: null, asset_id: 'metal-asset', unit_weight: '31,1034768', purity: '0,9', valuation_basis: 'intrinsic' }));
    expect(api.saveAsset).not.toHaveBeenCalled(); expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('conserve une ouverture explicitement nulle au début du suivi sans nouveau cash', async () => {
    vi.mocked(api.listMetalsAt).mockResolvedValue({ ...snapshot, movements: [] });
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Confirmer le stock d’ouverture' }));
    expect(screen.getByLabelText('Date du mouvement de métal')).toHaveValue('2026-01-01');
    await user.type(screen.getByLabelText('Variation signée de quantité'), '0');
    await user.type(screen.getByLabelText('Coût ajouté dans la devise du lot (vide : inconnu)'), '0');
    await user.type(screen.getByLabelText('Motif de l’enregistrement métal'), 'Inventaire confirmé vide');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source métal' }));
    await waitFor(() => expect(api.saveMetalMovement).toHaveBeenCalledOnce());
    expect(api.saveMetalMovement).toHaveBeenCalledWith(expect.objectContaining({ kind: 'opening', effective_on: '2026-01-01', quantity_change: '0', cost_amount: '0', trade_amount: null, trade_currency: null, cash_entity_id: '' }));
    expect(api.postMetalMovement).not.toHaveBeenCalled();
  });

  it('distingue l’once troy et une cotation de métal fin en conservant ses décimales', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Cours et primes' }));
    await user.click(screen.getByRole('button', { name: 'Ajouter un cours métal' }));
    await user.selectOptions(screen.getByLabelText('Unité de cotation du métal fin'), 'troy_oz');
    await user.type(screen.getByLabelText('Prix d’une unité de métal fin'), '3000,00000001');
    await user.type(screen.getByLabelText('Source du cours métal'), 'Document du cours');
    await user.type(screen.getByLabelText('Motif de l’enregistrement métal'), 'Cours manuel vérifié');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source métal' }));
    await waitFor(() => expect(api.saveMetalQuote).toHaveBeenCalledWith(expect.objectContaining({ weight_unit: 'troy_oz', price: '3000,00000001', source: 'Document du cours' })));
    expect(api.convertCurrency).not.toHaveBeenCalled();
  });

  it('présente le poids et l’intrinsèque de Rust sans présumer une prime ou une revente nulle', async () => {
    const { user } = await open();
    expect(screen.getByText('Stock de départ renseigné')).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Confirmer le stock d’ouverture' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Voir la valeur' }));
    await screen.findByText('55.98625824 g');
    expect(screen.getAllByText('3359.18 EUR')).toHaveLength(2);
    expect(screen.getByText('La prime de revente n’est pas renseignée.')).toBeVisible();
    expect(screen.getAllByText('Non calculable')).toHaveLength(3);
    expect(api.getMetalReport).toHaveBeenCalledExactlyOnceWith({ lot_id: 'lot', date: '2026-03-31', knowledge_sequence: null });
  });

  it('enregistre la vente partielle et son unique crédit net dans une même commande', async () => {
    const { user } = await open(); await saleForm(user);
    await user.selectOptions(screen.getByLabelText('Traitement de la trésorerie métal'), 'new');
    await user.selectOptions(screen.getByLabelText('Compte du mouvement métal'), 'bank');
    await user.type(screen.getByLabelText('Montant bancaire total signé'), '1480,00');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source métal' }));
    await waitFor(() => expect(api.postMetalMovement).toHaveBeenCalledOnce());
    const posted = vi.mocked(api.postMetalMovement).mock.calls[0][0];
    expect(posted.expected_sequence).toBe(42);
    expect(posted.movement).toEqual(expect.objectContaining({ kind: 'sale', quantity_change: '-1', cost_amount: null, trade_amount: '1500', trade_fees: '20', cash_entity_id: 'owner' }));
    expect(posted.transaction?.entries).toEqual([{ line_id: expect.any(String), account_id: 'bank', amount: '1480,00', role: 'principal' }]);
    expect(posted.links).toEqual([{ line_id: posted.transaction?.entries[0].line_id, amount: '1480,00' }]);
    expect(api.saveMetalMovement).not.toHaveBeenCalled(); expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('réutilise une révision bancaire existante sans recréer le produit de vente', async () => {
    const { user } = await open(); await saleForm(user);
    await user.selectOptions(screen.getByLabelText('Traitement de la trésorerie métal'), 'existing');
    await user.selectOptions(screen.getByLabelText('Opération bancaire du métal'), 'bank-sale-r3');
    await user.selectOptions(screen.getByLabelText('Ligne bancaire métal 1'), 'sale-line');
    await user.type(screen.getByLabelText('Montant signé rapproché métal 1'), '1480');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source métal' }));
    await waitFor(() => expect(api.postMetalMovement).toHaveBeenCalledOnce());
    expect(api.postMetalMovement).toHaveBeenCalledWith(expect.objectContaining({ transaction: null, existing_transaction_revision_id: 'bank-sale-r3', links: [{ line_id: 'sale-line', amount: '1480' }] }));
    expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('corrige la source liée et le même journal avec leurs révisions attendues', async () => {
    vi.mocked(api.listMetalsAt).mockResolvedValue({ ...snapshot, movements: [opening, sale], cash_links: [{ id: 'cash-link', revision_id: 'cash-link-r', movement_id: 'sale', movement_revision_id: 'sale-r2', transaction_revision_id: 'bank-sale-r3', line_id: 'sale-line', amount: '1480', notes: '' }] });
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Mouvements de métal' }));
    const row = screen.getByText('Vente, y compris partielle').closest('tr')!;
    await user.click(within(row).getByRole('button', { name: 'Modifier' }));
    expect(screen.getByLabelText('Traitement de la trésorerie métal')).toHaveValue('existing');
    await user.selectOptions(screen.getByLabelText('Traitement de la trésorerie métal'), 'correct');
    await user.clear(screen.getByLabelText('Prix total de la transaction avant frais'));
    await user.type(screen.getByLabelText('Prix total de la transaction avant frais'), '1600');
    await user.clear(screen.getByLabelText('Montant signé rapproché métal 1'));
    await user.type(screen.getByLabelText('Montant signé rapproché métal 1'), '1580');
    await user.clear(screen.getByLabelText('Nouveau montant bancaire signé 1 (Compte déclaré · EUR)'));
    await user.type(screen.getByLabelText('Nouveau montant bancaire signé 1 (Compte déclaré · EUR)'), '1580');
    await user.type(screen.getByLabelText('Motif de l’enregistrement métal'), 'Prix corrigé avec crédit bancaire');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la correction métal' }));
    await waitFor(() => expect(api.postMetalMovement).toHaveBeenCalledOnce());
    const posted = vi.mocked(api.postMetalMovement).mock.calls[0][0];
    expect(posted.movement.expected_revision_id).toBe('sale-r2');
    expect(posted.transaction).toEqual(expect.objectContaining({ id: 'bank-sale', expected_revision_id: 'bank-sale-r3', entries: [{ line_id: 'sale-line', account_id: 'bank', amount: '1580', role: 'principal' }] }));
    expect(posted.existing_transaction_revision_id).toBeNull();
  });

  it('transfère les droits en fermant les anciens et ouvrant les nouveaux sans toucher au stock', async () => {
    const { user } = await open();
    await user.selectOptions(screen.getByLabelText('Lot à consulter'), 'lot');
    await user.click(screen.getByText('Options avancées'));
    await user.click(screen.getByRole('button', { name: 'Transférer les droits du lot' }));
    await user.selectOptions(screen.getByLabelText('Nouveau propriétaire 1'), 'recipient');
    await user.type(screen.getByLabelText('Motif du transfert des droits'), 'Don interne documenté');
    await user.click(screen.getByRole('button', { name: 'Enregistrer le transfert des droits' }));
    await waitFor(() => expect(api.transferMetalOwnership).toHaveBeenCalledOnce());
    const input = vi.mocked(api.transferMetalOwnership).mock.calls[0][0];
    expect(input).toEqual(expect.objectContaining({ expected_sequence: 42, lot_id: 'lot', effective_on: '2026-03-31' }));
    expect(input.ownerships).toEqual([
      expect.objectContaining({ id: 'title', expected_revision_id: 'title-r', owner_entity_id: 'owner', valid_to: '2026-03-31' }),
      expect.objectContaining({ id: null, owner_entity_id: 'recipient', legal_share: one, economic_share: one, valid_from: '2026-03-31' }),
    ]);
    expect(api.saveMetalMovement).not.toHaveBeenCalled(); expect(api.saveTreasuryTransaction).not.toHaveBeenCalled();
  });

  it('interdit les mutations à K=0 mais permet le contrôle du stock ancien', async () => {
    vi.mocked(api.listMetalsAt).mockResolvedValue({ ...snapshot, movements: [] });
    const { user } = await open(0);
    await user.click(screen.getByText('Options avancées'));
    expect(screen.getByRole('button', { name: 'Rattacher un lot à un bien existant' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Confirmer le stock d’ouverture' })).toBeDisabled();
    await user.selectOptions(screen.getByLabelText('Lot à consulter'), 'lot');
    await user.click(screen.getByRole('button', { name: 'Contrôler un lot' }));
    await user.click(screen.getByRole('button', { name: 'Calculer le lot de métal' }));
    await waitFor(() => expect(api.getMetalReport).toHaveBeenCalledWith(expect.objectContaining({ knowledge_sequence: 0 })));
    expect(api.saveMetalMovement).not.toHaveBeenCalled();
  });

  it('masque le snapshot courant quand le chargement d’un autre K échoue', async () => {
    const { rerender, props } = await open();
    expect(screen.getByText('Pièces identifiées', { selector: 'strong' })).toBeVisible();
    vi.mocked(api.listMetalsAt).mockRejectedValueOnce({ message: 'État historique inaccessible.' });
    rerender(<MetalsPage {...props} context={{ ...props.context, knowledgeSequence: 3 }} />);
    await screen.findByText('État historique inaccessible.');
    expect(screen.queryByText('Pièces identifiées')).not.toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Lots enregistrés' })).not.toBeInTheDocument();
  });

  it('préserve les saisies quand une vente est refusée sans confirmer une réussite', async () => {
    vi.mocked(api.postMetalMovement).mockRejectedValueOnce({ message: 'La quantité vendue dépasse le stock historique.' });
    const { user } = await open(); await saleForm(user);
    await user.selectOptions(screen.getByLabelText('Traitement de la trésorerie métal'), 'new');
    await user.selectOptions(screen.getByLabelText('Compte du mouvement métal'), 'bank');
    await user.type(screen.getByLabelText('Montant bancaire total signé'), '1480');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source métal' }));
    await screen.findByText('La quantité vendue dépasse le stock historique.');
    expect(screen.getByLabelText('Motif de l’enregistrement métal')).toHaveValue('Vente documentée de la moitié du lot');
    expect(screen.getByLabelText('Variation signée de quantité')).toHaveValue('-1');
    expect(screen.queryByText('Les sources du métal ont été enregistrées avec leurs révisions.')).not.toBeInTheDocument();
  });

  it('un transfert de stockage reste de quantité nulle sans source bancaire', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: 'Mouvements de métal' }));
    await user.click(screen.getByRole('button', { name: 'Enregistrer un mouvement de métal' }));
    await user.selectOptions(screen.getByLabelText('Lot du mouvement'), 'lot');
    await user.selectOptions(screen.getByLabelText('Nature du mouvement de métal'), 'storage_transfer');
    await user.type(screen.getByLabelText('Variation signée de quantité'), '0');
    await user.selectOptions(screen.getByLabelText('Nouveau pays de stockage'), 'CH');
    await user.clear(screen.getByLabelText('Nouveau lieu de stockage'));
    await user.type(screen.getByLabelText('Nouveau lieu de stockage'), 'Lieu suisse déclaré');
    await user.type(screen.getByLabelText('Motif de l’enregistrement métal'), 'Déplacement physique sans vente');
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source métal' }));
    await waitFor(() => expect(api.saveMetalMovement).toHaveBeenCalledWith(expect.objectContaining({ kind: 'storage_transfer', quantity_change: '0', country: 'CH', storage_location: 'Lieu suisse déclaré', cost_amount: null, trade_amount: null, cash_entity_id: '' })));
    expect(api.postMetalMovement).not.toHaveBeenCalled();
  });
  it('MET-01 annonce la cible et ferme le transfert au changement de lot sans modifier aucun droit', async () => {
    const secondLot = { ...lot, id: 'lot-b', revision_id: 'lot-b-r', asset_id: 'metal-b', designation: 'Second lot' };
    vi.mocked(api.listMetalsAt).mockResolvedValue({ ...snapshot, lots: [lot, secondLot] });
    const { user } = await open();
    await user.selectOptions(screen.getByLabelText('Lot à consulter'), 'lot');
    await user.click(screen.getByText('Options avancées'));
    await user.click(screen.getByRole('button', { name: 'Transférer les droits du lot' }));
    expect(screen.getByText('Lot concerné : Pièces identifiées')).toBeVisible();
    await user.selectOptions(screen.getByLabelText('Nouveau propriétaire 1'), 'recipient');
    await user.type(screen.getByLabelText('Motif du transfert des droits'), 'Don du premier lot');
    await user.selectOptions(screen.getByLabelText('Lot à consulter'), 'lot-b');
    expect(screen.queryByRole('button', { name: 'Enregistrer le transfert des droits' })).not.toBeInTheDocument();
    expect(api.transferMetalOwnership).not.toHaveBeenCalled();
    await user.selectOptions(screen.getByLabelText('Lot à consulter'), 'lot');
    await user.click(screen.getByRole('button', { name: 'Transférer les droits du lot' }));
    await user.selectOptions(screen.getByLabelText('Nouveau propriétaire 1'), 'recipient');
    await user.type(screen.getByLabelText('Motif du transfert des droits'), 'Don du premier lot confirmé');
    await user.click(screen.getByRole('button', { name: 'Enregistrer le transfert des droits' }));
    await waitFor(() => expect(api.transferMetalOwnership).toHaveBeenCalledOnce());
    const input = vi.mocked(api.transferMetalOwnership).mock.calls[0][0];
    expect(input.lot_id).toBe('lot');
    expect(input.ownerships.every(row => row.asset_id === 'metal-asset')).toBe(true);
  });

  it.each(['parent', 'sources'])('REG-03 conserve la vente confirmée quand le rechargement %s échoue', async source => {
    const { user, props } = await open(); await saleForm(user);
    await user.type(screen.getByLabelText('Motif du cash non rapproché'), 'Relevé conservé séparément');
    if (source === 'parent') props.onChanged.mockRejectedValueOnce(new Error('Relecture interrompue'));
    else vi.mocked(api.listMetalsAt).mockRejectedValueOnce(new Error('Relecture interrompue'));
    await user.click(screen.getByRole('button', { name: 'Enregistrer la source métal' }));
    expect(await screen.findByText(/Les sources du métal ont été enregistrées.*L’affichage n’a pas pu être actualisé/)).toHaveTextContent('Relecture interrompue');
    expect(screen.queryByRole('button', { name: 'Enregistrer la source métal' })).not.toBeInTheDocument();
    expect(api.saveMetalMovement).toHaveBeenCalledOnce();
    expect(api.postMetalMovement).not.toHaveBeenCalled();
  });

});
