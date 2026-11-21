import { screen, within } from '@testing-library/react';
import type { UserEvent } from '@testing-library/user-event';

export async function openModule(user: UserEvent, name: string) {
  const nav = within(screen.getByRole('navigation', { name: 'Navigation principale' }));
  if (!nav.queryByRole('button', { name })) {
    const group = ['Documents', 'Historique', 'Paramètres'].includes(name) ? 'Outils et réglages' : 'Autres biens et dettes';
    await user.click(nav.getByRole('button', { name: group }));
  }
  await user.click(nav.getByRole('button', { name }));
}
