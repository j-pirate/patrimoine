import { useState } from 'react';
import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { api } from '../services/api';
import type { VaultStatus } from '../services/api';
import { emptyRegistry } from '../services/registry-types';
import type { RegistrySnapshot } from '../services/registry-types';
import Onboarding from './onboarding';

vi.mock('../services/api', async importOriginal => {
  const actual = await importOriginal<typeof import('../services/api')>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});

const newVault: VaultStatus = {
  generation_id: 'g1', data_dir: '/vault/test', schema_version: 6, app_version: '0.2.0', document_count: 0,
  mutation_sequence: 0, settings: { workspace_name: 'Mon patrimoine familial', display_currency: 'EUR' },
  country_count: 4, currency_count: 5, recovery_required: false, recovery_message: null,
};

const populated: RegistrySnapshot = {
  ...emptyRegistry,
  entities: [{
    id: 'camille', revision_id: 'camille-v1', kind: 'person', display_name: 'Camille Rivière',
    first_name: 'Camille', last_name: 'Rivière', birth_date: null, residence_country: 'FR',
    preferred_currency: 'EUR', activity: '', notes: '', archived: false,
  }],
};

function AppHarness({ status, registry, onSaved, onDiscover }: {
  status: VaultStatus;
  registry: RegistrySnapshot;
  onSaved: () => void;
  onDiscover?: () => void;
}) {
  const [request, setRequest] = useState(0);
  return <>
    <main><h1 tabIndex={-1}>Tableau de bord</h1><button onClick={() => setRequest(current => current + 1)}>Reprendre la configuration</button></main>
    <Onboarding status={status} registry={registry} openRequested={request} onSaved={onSaved} onDiscover={onDiscover} />
  </>;
}
function dialog() {
  return within(screen.getByRole('dialog'));
}

function open({ status = newVault, registry = emptyRegistry, onDiscover }: {
  status?: VaultStatus; registry?: RegistrySnapshot; onDiscover?: () => void;
} = {}) {
  const user = userEvent.setup();
  const onSaved = vi.fn();
  const view = render(<AppHarness status={status} registry={registry} onSaved={onSaved} onDiscover={onDiscover} />);
  const click = async (name: string) => user.click(await within(await screen.findByRole('dialog')).findByRole('button', { name }));
  const type = (name: string, value: string) => user.type(dialog().getByRole('textbox', { name }), value);
  const select = (name: string, value: string) => user.selectOptions(dialog().getByRole('combobox', { name }), value);
  return { ...view, user, onSaved, click, type, select };
}

const configByGeneration = new Map<string, string>();
async function persist(generation: string, data: string) {
  configByGeneration.set(generation, data);
}

beforeEach(() => {
  configByGeneration.clear();
  vi.resetAllMocks();
  vi.mocked(api.readOnboardingState).mockImplementation(async generation => configByGeneration.get(generation) ?? null);
  vi.mocked(api.writeOnboardingState).mockImplementation(persist);
  vi.mocked(api.createGuidedFamily)
    .mockImplementation(async input => ({ id: input.first_name, name: `${input.first_name} ${input.last_name}` }));
  vi.mocked(api.createGuidedProperty).mockResolvedValue({ id: 'house', name: 'Les Tilleuls' });
  vi.mocked(api.createGuidedAccount).mockResolvedValue({ id: 'cash', name: 'Caisse voyage' });
  vi.mocked(api.createGuidedMetal).mockResolvedValue({ id: 'metal', name: 'Pièces test' });
  vi.mocked(api.createGuidedCompany).mockResolvedValue({ id: 'company', name: 'Ateliers du Canal' });
});

describe('Assistant de configuration', () => {
  it('permet de passer l’accueil et ne se représente pas au prochain lancement', async () => {
    const first = open();
    await screen.findByRole('dialog');
    expect(dialog().getByRole('heading', { name: 'Bienvenue dans Patrimoine' })).toHaveFocus();
    await first.click('Passer et accéder à l’application');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(first.onSaved).not.toHaveBeenCalled();
    first.unmount();
    const reopened = open();
    await waitFor(() => expect(api.readOnboardingState).toHaveBeenCalledTimes(2));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await reopened.user.click(screen.getByRole('button', { name: 'Reprendre la configuration' }));
    expect(dialog().getByRole('heading', { name: 'Bienvenue dans Patrimoine' })).toBeVisible();
    expect(api.createGuidedFamily).not.toHaveBeenCalled();
  });

  it('permet de passer les cinq saisies et de terminer sans créer de données', async () => {
    const { click, onSaved } = open();
    await click('Commencer la configuration');
    for (const step of ['Famille', 'Immobilier', 'Comptes et espèces', 'Or et argent', 'Sociétés']) {
      expect(dialog().getByRole('heading', { name: step })).toBeVisible();
      await click('Passer cette étape');
    }
    expect(dialog().getByRole('heading', { name: 'Votre configuration' })).toBeVisible();
    expect(dialog().getByText(/Aucune information n’a été enregistrée/)).toBeVisible();
    expect(dialog().queryByRole('button', { name: 'Découvrir Patrimoine' })).not.toBeInTheDocument();
    await click('Accéder à mon patrimoine');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(onSaved).not.toHaveBeenCalled();
  });

  it('ouvre manuellement un coffre utilisé et rend le focus au bouton après une pause', async () => {
    const { user, click, type } = open({ status: { ...newVault, mutation_sequence: 5 }, registry: populated });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    const launcher = screen.getByRole('button', { name: 'Reprendre la configuration' });
    await user.click(launcher);
    await click('Commencer la configuration');
    await click('Passer cette étape');
    await type('Nom du bien', 'Brouillon');
    await click('Quitter et reprendre plus tard');
    await waitFor(() => expect(launcher).toHaveFocus());
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();

    await user.click(launcher);
    expect(dialog().getByRole('heading', { name: 'Immobilier' })).toHaveFocus();
    expect(dialog().getByRole('textbox', { name: 'Nom du bien' })).toHaveValue('Brouillon');
    expect(api.createGuidedProperty).not.toHaveBeenCalled();
  });

  it('reprend une confirmation interrompue avec le même identifiant après la fermeture', async () => {
    const first = open();
    let settle = (_: { id: string; name: string }) => {};
    vi.mocked(api.createGuidedFamily).mockImplementationOnce(() => new Promise(resolve => { settle = resolve; }));
    await first.click('Commencer la configuration');
    await first.type('Prénom', 'Camille');
    await first.type('Nom', 'Rivière');
    await first.click('Enregistrer le membre');
    expect(api.createGuidedFamily).toHaveBeenCalledOnce();

    first.unmount();
    const next = open({ status: { ...newVault, mutation_sequence: 1 }, registry: populated });
    await screen.findByRole('dialog');
    expect(dialog().getByRole('heading', { name: 'Famille' })).toBeVisible();
    await next.click('Confirmer l’enregistrement');
    const invocations = vi.mocked(api.createGuidedFamily).mock.calls;
    expect(invocations).toHaveLength(2);
    expect(invocations[1]).toEqual(invocations[0]);
    expect(next.onSaved).toHaveBeenCalledOnce();
    expect(dialog().getAllByRole('listitem')).toHaveLength(1);

    await act(async () => { settle({ id: 'Camille', name: 'Camille Rivière' }); });
    expect(first.onSaved).not.toHaveBeenCalled();
  });

  it('attend la copie native de la tentative avant de créer une fiche', async () => {
    const { click, type } = open();
    await click('Commencer la configuration');
    await type('Prénom', 'Camille');
    await type('Nom', 'Rivière');
    let release: (() => Promise<void>) | undefined;
    vi.mocked(api.writeOnboardingState).mockImplementation(async (generation, data) => {
      if (JSON.parse(data).drafts.family?.requestId) {
        await new Promise<void>(resolve => { release = async () => { await persist(generation, data); resolve(); }; });
      } else {
        await persist(generation, data);
      }
    });

    await click('Enregistrer le membre');
    await waitFor(() => expect(release).toBeTypeOf('function'));
    expect(api.createGuidedFamily).not.toHaveBeenCalled();
    expect(dialog().getByRole('button', { name: 'Passer cette étape' })).toBeDisabled();
    await act(async () => { await release?.(); });
    expect(api.createGuidedFamily).toHaveBeenCalledOnce();
  });

  it('conserve une seule saisie valide et le brouillon rejeté quand on revient en arrière', async () => {
    const { click, type, select, onSaved } = open();
    await click('Commencer la configuration');
    await type('Prénom', 'Camille');
    await type('Nom', 'Rivière');
    await click('Enregistrer le membre');
    await click('Continuer');

    vi.mocked(api.createGuidedProperty).mockRejectedValueOnce({ code: 'validation', message: 'La valeur doit être positive.' });
    await type('Nom du bien', 'Les Tilleuls');
    await select('Type de bien', 'house');
    await select('Pays du bien', 'FR');
    await select('État du bien', 'vacant');
    await type('Valeur estimée (facultatif)', '-120000');
    await click('Enregistrer le bien');
    expect(await dialog().findByRole('alert')).toHaveTextContent('La valeur doit être positive.');

    await click('Précédent');
    expect(dialog().getByRole('listitem')).toHaveTextContent('Camille Rivière');
    await click('Continuer');
    expect(dialog().getByRole('textbox', { name: 'Valeur estimée (facultatif)' })).toHaveValue('-120000');
    for (let index = 0; index < 4; index++) await click('Passer cette étape');
    expect(dialog().getByText('Camille Rivière')).toBeVisible();
    expect(dialog().queryByText('Les Tilleuls')).not.toBeInTheDocument();
    expect(onSaved).toHaveBeenCalledOnce();
    await click('Accéder à mon patrimoine');
  });

  it('enregistre les cinq catégories et récapitule les deux membres et les biens', async () => {
    const discover = vi.fn();
    const { click, type, select, onSaved } = open({ registry: populated, onDiscover: discover });
    await click('Commencer la configuration');
    await type('Prénom', 'Camille');
    await type('Nom', 'Rivière');
    await click('Enregistrer le membre');
    await type('Prénom', 'Jules');
    await type('Nom', 'Rivière');
    await click('Enregistrer le membre');
    await click('Continuer');

    await type('Nom du bien', 'Les Tilleuls');
    await select('Type de bien', 'house');
    await select('Pays du bien', 'FR');
    await select('État du bien', 'vacant');
    await click('Enregistrer le bien');
    await click('Continuer');

    await type('Banque ou nom du compte', 'Brouillon bancaire');
    await click('Caisse d’espèces');
    await type('Nom de la caisse', 'Caisse voyage');
    await select('Pays de conservation', 'FR');
    await type('Lieu de conservation', 'Coffre');
    await type('Solde de départ', '500,25');
    await select('Propriétaire unique (100 %)', 'camille');
    await click('Enregistrer la caisse');
    await click('Compte bancaire');
    expect(dialog().getByRole('textbox', { name: 'Banque ou nom du compte' })).toHaveValue('Brouillon bancaire');
    await click('Continuer');

    await select('Métal', 'gold');
    await select('Forme', 'coin');
    await type('Nom du lot', 'Pièces test');
    await type('Quantité', '3');
    await type('Poids d’un objet', '6,45');
    await type('Pureté', '0,9');
    await select('Propriétaire unique (100 %)', 'camille');
    await select('Pays de conservation', 'FR');
    await type('Lieu de conservation', 'Coffre');
    await click('Enregistrer le lot');
    await click('Continuer');

    await type('Nom de la société', 'Ateliers du Canal');
    await select('Pays de la société', 'FR');
    await click('Enregistrer la société');
    await click('Continuer');

    for (const name of ['Camille Rivière', 'Jules Rivière', 'Les Tilleuls', 'Caisse voyage', 'Pièces test', 'Ateliers du Canal']) {
      expect(dialog().getByText(name)).toBeVisible();
    }
    expect(dialog().queryByText('Brouillon bancaire')).not.toBeInTheDocument();
    expect(onSaved).toHaveBeenCalledTimes(6);
    await click('Découvrir Patrimoine');
    expect(discover).toHaveBeenCalledOnce();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});
