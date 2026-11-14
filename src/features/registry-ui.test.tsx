import { useState } from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { EditorFrame } from './registry-ui';

function EditorExample() {
  const [opened, setOpened] = useState(false);
  return <><button type="button" onClick={() => setOpened(true)}>Ouvrir le formulaire</button>{opened && <EditorFrame title="Modifier une fiche" pending={false} onCancel={() => setOpened(false)}><label htmlFor="example-name">Nom</label><input id="example-name" /></EditorFrame>}</>;
}

describe('Clavier des formulaires en ligne', () => {
  it('annonce le titre à l’ouverture et rend le focus au bouton après fermeture', async () => {
    render(<EditorExample />);
    const opener = screen.getByRole('button', { name: 'Ouvrir le formulaire' });
    opener.focus(); fireEvent.click(opener);
    expect(screen.getByRole('heading', { name: 'Modifier une fiche' })).toHaveFocus();
    fireEvent.click(screen.getByRole('button', { name: 'Fermer le formulaire' }));
    await waitFor(() => expect(opener).toHaveFocus());
  });

  it('conserve le focus pendant une actualisation et ignore un ancien bouton retiré', async () => {
    const { rerender } = render(<><button type="button">Ancienne action</button><EditorFrame title="Modifier une fiche" pending={false} onCancel={() => undefined}><input aria-label="Nom" /></EditorFrame></>);
    const input = screen.getByRole('textbox', { name: 'Nom' });
    input.focus();
    rerender(<><button type="button">Ancienne action</button><EditorFrame title="Modifier une fiche" pending={true} onCancel={() => undefined}><input aria-label="Nom" /></EditorFrame></>);
    expect(input).toHaveFocus();
    rerender(<button type="button">Nouvelle page</button>);
    const next = screen.getByRole('button', { name: 'Nouvelle page' });
    next.focus();
    await waitFor(() => expect(next).toHaveFocus());
  });
});
