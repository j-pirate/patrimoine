import { StrictMode, useState } from 'react';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import DiscoveryTour from './discovery-tour';
import type { TourDestination, TourStepId } from './discovery-tour';

function setup(initialStep: TourStepId = 'overview') {
  const user = userEvent.setup();
  const selectDestination = vi.fn<(destination: TourDestination) => void>();
  function Harness() {
    const [open, setOpen] = useState(false);
    return <>
      <main><h1 tabIndex={-1}>Aide</h1><button onClick={() => setOpen(true)}>Découvrir Patrimoine</button></main>
      {open && <DiscoveryTour initialStep={initialStep} onClose={() => setOpen(false)} onNavigate={selectDestination} />}
    </>;
  }
  const view = render(<StrictMode><Harness /></StrictMode>);
  const trigger = screen.getByRole('button', { name: 'Découvrir Patrimoine' });
  async function open() {
    await user.click(trigger);
    return screen.getByRole('dialog', { name: 'Découvrir Patrimoine' });
  }
  return { ...view, user, trigger, open, selectDestination };
}

describe('Découvrir Patrimoine', () => {
  it('avance, revient et termine en rendant le focus au bouton d’ouverture', async () => {
    const { user, trigger, open, selectDestination } = setup();
    const modal = within(await open());
    expect(modal.getByRole('heading', { name: 'Comprendre le tableau de bord' })).toHaveFocus();
    expect(modal.getByRole('button', { name: 'Précédent' })).toBeDisabled();
    expect(modal.getByRole('progressbar', { name: 'Avancement du tour' })).toHaveValue(1);

    await user.click(modal.getByRole('button', { name: 'Suivant' }));
    expect(modal.getByRole('heading', { name: 'Ajouter un membre de la famille' })).toHaveFocus();
    expect(modal.getByText('Étape 2 sur 8')).toBeVisible();
    await user.click(modal.getByRole('button', { name: 'Précédent' }));
    expect(modal.getByText('Étape 1 sur 8')).toBeVisible();

    for (let next = 2; next <= 8; next++) {
      await user.click(modal.getByRole('button', { name: 'Suivant' }));
      expect(modal.getByText(`Étape ${next} sur 8`)).toBeVisible();
    }
    await user.click(modal.getByRole('button', { name: 'Terminer' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await waitFor(() => expect(trigger).toHaveFocus());
    expect(selectDestination).not.toHaveBeenCalled();
  });

  it('ouvre l’aide contextuelle, répond aux flèches et traite la fermeture native', async () => {
    const { user, trigger, open, selectDestination } = setup('rentals');
    const dialog = await open();
    const modal = within(dialog);
    expect(modal.getByRole('heading', { name: 'Gérer les loyers' })).toHaveFocus();
    await user.keyboard('{ArrowLeft}');
    expect(modal.getByRole('heading', { name: 'Ajouter un bien immobilier' })).toHaveFocus();
    await user.keyboard('{ArrowRight}');
    expect(modal.getByText('Étape 4 sur 8')).toBeVisible();

    // WebKit dispatches `cancel` for Escape; jsdom has no dialog top layer.
    fireEvent(dialog, new Event('cancel', { cancelable: true }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await waitFor(() => expect(trigger).toHaveFocus());
    expect(selectDestination).not.toHaveBeenCalled();
  });

  it('renvoie vers la page de sauvegarde et rend le focus à son titre', async () => {
    const { user, open, selectDestination } = setup('backup');
    const modal = within(await open());
    expect(modal.getByText(/Une sauvegarde crée une copie du coffre/)).toBeVisible();
    await user.click(modal.getByRole('button', { name: 'Ouvrir la page Sauvegarde et export' }));
    expect(selectDestination).toHaveBeenCalledExactlyOnceWith('backup');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole('heading', { level: 1 })).toHaveFocus());
  });

  it('embarque huit illustrations différentes et indique leurs données fictives', async () => {
    const { user, open } = setup();
    const modal = within(await open());
    const urls = new Set<string>();
    for (let index = 0; index < 8; index++) {
      const illustration = modal.getByRole('img');
      expect(illustration).toHaveAttribute('alt', expect.stringMatching(/.{20,}/));
      expect(modal.getByText('Exemple fictif.')).toBeVisible();
      const src = illustration.getAttribute('src') ?? '';
      expect(src.startsWith('/') || src.startsWith('data:image/webp')).toBe(true);
      urls.add(src);
      if (index < 7) await user.click(modal.getByRole('button', { name: 'Suivant' }));
    }
    expect(urls.size).toBe(8);
    await user.click(modal.getByRole('button', { name: 'Fermer le tour' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});
