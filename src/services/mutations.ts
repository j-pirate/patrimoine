import { errorMessage } from './api';

/** Call only after the write has completed. A refresh failure never replays it. */
export async function refreshAfterCommit(
  refresh: () => Promise<void>,
  success: string,
  onMessage: (message: string) => void,
): Promise<void> {
  onMessage(success);
  try {
    await refresh();
  } catch (error) {
    onMessage(`${success} L’affichage n’a pas pu être actualisé : ${errorMessage(error)} Rechargez les données avant une autre modification.`);
  }
}
