import { describe, expect, it, vi } from 'vitest';
import { errorAction, errorCode, errorCodes, errorMessage } from './api';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(), save: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

describe('Erreurs du contrat IPC', () => {
  it.each(errorCodes)('reconnaît le code stable %s', code => {
    expect(errorCode({ code, message: 'Diagnostic conservé' })).toBe(code);
  });

  it.each([null, undefined, false, 3, 'validation', {}, { code: null }, { code: 3 }, { code: 'legacy_custom' }])('traite une valeur inconnue sans lever d’erreur : %j', value => {
    expect(errorCode(value)).toBeNull();
    expect(errorAction(value)).toBeNull();
  });

  it('conserve le diagnostic d’un code ancien ou personnalisé', () => {
    const error = { code: 'legacy_custom', message: 'Diagnostic précis de l’archive historique.' };
    expect(errorMessage(error)).toBe(error.message);
    expect(error).toEqual({ code: 'legacy_custom', message: 'Diagnostic précis de l’archive historique.' });
  });

  it('associe le conflit à une comparaison des révisions', () => {
    const action = errorAction({ code: 'conflict' });
    expect(action).toContain('comparez les révisions');
    expect(errorMessage({ code: 'conflict', message: 'Conflit détecté.' })).toBe(`Conflit détecté. ${action}`);
  });

  it('garde les actions de récupération et la portée du fichier explicites', () => {
    expect(errorAction({ code: 'recovery_required' })).toContain('en conservant ses données');
    expect(errorAction({ code: 'FILE_SCOPE' })).toContain('dialogue');
    expect(errorAction({ code: 'snapshot_timeout' })).toContain('écritures concurrentes');
  });

  it('propose une action adaptée aux erreurs SQLite, de version et de lecture du CSV', () => {
    expect(errorAction({ code: 'database' })).toContain('outils qui accèdent au coffre');
    expect(errorAction({ code: 'schema_version' })).toContain('version de Patrimoine');
    expect(errorAction({ code: 'invalid_csv' })).toContain('table, la ligne et la colonne');
  });

  it('n’ajoute pas deux fois la même action', () => {
    const message = errorAction({ code: 'validation' })!;
    expect(errorMessage({ code: 'validation', message })).toBe(message);
  });

  it('affiche les erreurs textuelles anciennes et garde un message pour les erreurs mal formées', () => {
    expect(errorMessage('Échec historique précis.')).toBe('Échec historique précis.');
    expect(errorMessage({ code: 'unknown', message: '' })).toContain('aucune réussite n’a été confirmée');
    expect(errorMessage(null)).toContain('aucune réussite n’a été confirmée');
  });
});
