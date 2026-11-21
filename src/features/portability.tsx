import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { refreshAfterCommit } from '../services/mutations';
import { api, errorMessage } from '../services/api';
import type { BackupSummary, RestoreInspection } from '../services/api';
import type { ExportSummary } from '../services/portability-types';

type ImportKind = 'backup' | 'csv';

function Notice({ children, error = false }: { children: ReactNode; error?: boolean }) {
  return <div className={`notice notice-${error ? 'error' : 'info'}`} role={error ? 'alert' : 'status'}>{children}</div>;
}
function sizeLabel(bytes: number): string {
  if (bytes < 1024) return `${bytes} octets`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Ko`;
  return `${(bytes / (1024 * 1024)).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Mo`;
}
function dateLabel(value: string): string {
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString('fr-FR');
}

export default function PortabilityPage({ currentSchema, onRestored, onBusyChange }: {
  currentSchema: number; onRestored: () => Promise<void>; onBusyChange: (busy: boolean) => void;
}) {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [backup, setBackup] = useState<BackupSummary | null>(null);
  const [exported, setExported] = useState<ExportSummary | null>(null);
  const [inspection, setInspection] = useState<RestoreInspection | null>(null);
  const [importKind, setImportKind] = useState<ImportKind>('backup');
  const [safetyBackup, setSafetyBackup] = useState(true);
  const inspectionRef = useRef<HTMLElement>(null);
  useEffect(() => { onBusyChange(pending); return () => onBusyChange(false); }, [pending, onBusyChange]);
  useEffect(() => { if (inspection) inspectionRef.current?.focus(); }, [inspection]);

  async function createBackup() {
    setPending(true); setError(''); setMessage('');
    try {
      const destination = await api.chooseBackupDestination();
      if (!destination) return;
      const result = await api.createBackup(destination);
      setBackup(result);
      setMessage('La sauvegarde complète a été créée. Conservez une copie sur un autre support.');
    } catch (failure) { setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  async function exportTables(format: 'csv' | 'xlsx') {
    setPending(true); setError(''); setMessage('');
    try {
      const destination = format === 'csv' ? await api.chooseCsvExportDestination() : await api.chooseXlsxDestination();
      if (!destination) return;
      const result = format === 'csv' ? await api.exportCsvArchive(destination) : await api.exportXlsx(destination);
      setExported(result);
      setMessage(format === 'csv' ? 'L’archive CSV intégrale a été créée avec les originaux documentaires.' : 'Le classeur XLSX complet a été créé pour consultation. Les originaux documentaires restent dans les archives ZIP.');
    } catch (failure) { setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  async function inspect(kind: ImportKind) {
    setPending(true); setError(''); setMessage(''); setInspection(null); setImportKind(kind);
    try {
      const archive = kind === 'csv' ? await api.chooseCsvImportArchive() : await api.chooseRestoreArchive();
      if (!archive || Array.isArray(archive)) return;
      const result = kind === 'csv' ? await api.inspectCsvImport(archive) : await api.inspectRestore(archive);
      setInspection(result); setSafetyBackup(true);
    } catch (failure) { setError(errorMessage(failure)); }
    finally { setPending(false); }
  }
  async function restore() {
    if (!inspection) return;
    setPending(true); setError(''); setMessage('');
    try {
      const safetyPath = safetyBackup ? await api.chooseBackupDestination(true) : null;
      if (safetyBackup && !safetyPath) {
        setMessage(`${importKind === 'csv' ? 'Réimportation' : 'Restauration'} annulée : aucun fichier de sauvegarde de l’état actuel n’a été choisi.`);
        return;
      }
      const activated = await api.activateRestore(inspection.token, safetyPath);
      setInspection(null);
      await refreshAfterCommit(onRestored, (importKind === 'csv' ? 'L’archive CSV a été réimportée. Le coffre a été remplacé par les données vérifiées.' : 'La sauvegarde a été restaurée. Le coffre et les documents sont maintenant ceux de la sauvegarde.') + (activated.recovery_required ? ` ${activated.recovery_message || 'Une reprise doit encore être finalisée.'}` : ''), setMessage);
    } catch (failure) { setError(errorMessage(failure)); }
    finally { setPending(false); }
  }

  return <div className="registry-workspace" aria-busy={pending}>
    {error && <Notice error>{error}</Notice>}
    {message && <Notice>{message}</Notice>}
    <section className="card"><h2>Sauvegarder</h2><p>Créez une copie de tout votre coffre, y compris son historique et les documents originaux. Gardez cette archive ZIP pour pouvoir récupérer vos données.</p><button type="button" className="button" disabled={pending} onClick={() => { void createBackup(); }}>{pending ? 'Opération en cours…' : 'Créer une sauvegarde complète'}</button></section>
    <div className="two-columns">
      <section className="card"><h2>Exporter</h2><p>Le classeur XLSX permet de consulter vos données dans un tableur. L’archive CSV contient aussi les documents originaux et permet de reconstruire le coffre.</p><div className="button-row"><button type="button" className="button secondary" disabled={pending} onClick={() => { void exportTables('xlsx'); }}>Exporter un classeur XLSX complet</button><button type="button" className="button secondary" disabled={pending} onClick={() => { void exportTables('csv'); }}>Exporter une archive CSV intégrale</button></div></section>
      <section className="card"><h2>Restaurer</h2><p>Choisissez une sauvegarde à vérifier. Son contenu remplacera le coffre actuel uniquement après votre confirmation.</p><div className="button-row"><button type="button" className="button secondary" disabled={pending} onClick={() => { void inspect('backup'); }}>Choisir une sauvegarde à analyser</button><button type="button" className="button secondary" disabled={pending} onClick={() => { void inspect('csv'); }}>Choisir une archive CSV à analyser</button></div></section>
    </div>
    {backup && <section className="card result-card"><h2>Dernière sauvegarde de cette session</h2><dl className="details"><div><dt>Fichier</dt><dd className="path">{backup.path}</dd></div><div><dt>Créée le</dt><dd>{dateLabel(backup.created_at)}</dd></div><div><dt>Contenu</dt><dd>{backup.document_count} document(s) · {backup.blob_count} fichier(s) distinct(s) · {sizeLabel(backup.size_bytes)}</dd></div><div><dt>SHA-256 de l’archive</dt><dd className="hash">{backup.sha256}</dd></div></dl></section>}
    {exported && <section className="card result-card"><h2>Dernier export intégral de cette session</h2><dl className="details"><div><dt>Format</dt><dd>{exported.format === 'structured_csv' ? 'Archive CSV avec originaux' : 'XLSX de consultation'}</dd></div><div><dt>Fichier</dt><dd className="path">{exported.path}</dd></div><div><dt>État exporté</dt><dd>Schéma {exported.schema_version} · séquence {exported.knowledge_sequence}</dd></div><div><dt>Tables et lignes</dt><dd>{exported.table_count} tables · {exported.row_count} lignes</dd></div><div><dt>Documents</dt><dd>{exported.document_count} document(s) · {exported.blob_count} contenu(s) distinct(s) · {sizeLabel(exported.size_bytes)}</dd></div><div><dt>SHA-256</dt><dd className="hash">{exported.sha256}</dd></div></dl></section>}
    {inspection && <section className="card restore-review" aria-labelledby="restore-heading" tabIndex={-1} ref={inspectionRef}>
      <span className="badge warning-badge">Confirmation nécessaire</span><h2 id="restore-heading">{importKind === 'csv' ? 'Vérifier avant de réimporter' : 'Vérifier avant de restaurer'}</h2>
      {importKind === 'csv' && <Notice>Une base vierge a été reconstruite et vérifiée depuis les CSV. Le coffre actuel n’a pas encore été remplacé.</Notice>}
      {inspection.schema_version < currentSchema && <Notice>Cette archive utilise le schéma {inspection.schema_version}. La restauration utilisera le schéma {currentSchema}, après mise à niveau et vérification dans une copie séparée. L’archive d’origine reste inchangée.</Notice>}
      <p className="restore-warning">Cette opération remplacera l’intégralité du coffre actuel : paramètres, données et documents. Elle ne fusionne pas les deux espaces.</p>
      <dl className="details"><div><dt>Espace sauvegardé</dt><dd>{inspection.settings.workspace_name}</dd></div><div><dt>Date de la sauvegarde</dt><dd>{dateLabel(inspection.created_at)}</dd></div><div><dt>Version de l’application / schéma</dt><dd>{inspection.app_version} / {inspection.schema_version}</dd></div><div><dt>Documents</dt><dd>{inspection.document_count} document(s), {inspection.blob_count} fichier(s) distinct(s)</dd></div><div><dt>Empreinte de l’archive analysée</dt><dd className="hash">{inspection.archive_sha256}</dd></div></dl>
      <label className="checkbox-label"><input type="checkbox" checked={safetyBackup} disabled={pending} onChange={event => setSafetyBackup(event.currentTarget.checked)} /><span>Créer une sauvegarde de mon état actuel avant le remplacement <small>Recommandé. Choisissez son emplacement au moment de confirmer.</small></span></label>
      {!safetyBackup && <p className="restore-warning">Vous avez choisi de ne pas exporter de sauvegarde de l’état actuel avant le remplacement.</p>}
      <div className="button-row"><button type="button" className="button danger" disabled={pending} onClick={() => { void restore(); }}>{pending ? 'Restauration en cours…' : 'Confirmer le remplacement du coffre'}</button><button type="button" className="button secondary" disabled={pending} onClick={() => setInspection(null)}>Annuler</button></div>
    </section>}
    <details className="card portability-note"><summary>Détails sur les formats et la restauration</summary><p>Ces opérations portent sur tout le coffre actuel, indépendamment du membre, de la date, des connaissances K et des filtres de consultation.</p><p>La sauvegarde ZIP contient la base SQLite et les documents originaux. L’archive CSV contient les tables exactes, les versions antérieures, les relations, le dictionnaire et les documents originaux. La restauration vérifie l’archive et remplace le coffre entier après confirmation.</p><p>Le XLSX est un format de consultation, sans import XLSX. Il contient les métadonnées des documents. Les originaux sont dans les archives ZIP.</p><p className="muted small">Les archives ne sont pas chiffrées par l’application. Choisissez un emplacement protégé et un support de sauvegarde indépendant.</p></details>
  </div>;
}
