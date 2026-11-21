import { useCallback, useEffect, useRef, useState } from 'react';
import type { FormEvent } from 'react';
import { refreshAfterCommit } from '../services/mutations';
import { api, errorMessage } from '../services/api';
import type { DocumentLibrarySnapshot, DocumentLink, DocumentRecord } from '../services/documents-types';
import { EditorFrame, Field, RegistryNotice, useFocusedRegion } from './registry-ui';
import type { ViewContext } from './registry-ui';

type Editor = { kind: 'metadata' | 'replace'; document: DocumentRecord; description: string; status: string; reason: string; confirmed: boolean }
  | { kind: 'link'; document: DocumentRecord; record: DocumentLink | null; target: string; role: string; from: string; to: string; reason: string };
const statusNames: Record<string, string> = { active: 'Actif', archived: 'Archivé', deleted: 'Retiré du classement' };
const targetKinds: Record<string, string> = {
  entity: 'Personne ou société', membership: 'Période familiale', participation: 'Participation', asset: 'Actif', ownership: 'Droit de propriété', valuation: 'Estimation', company_policy: 'Méthode de société', exchange_rate: 'Cours de change', profile_period: 'Période descriptive', property: 'Fiche immobilière', rental_unit: 'Unité locative', property_event: 'Événement immobilier', lease: 'Bail', lease_terms: 'Conditions de bail', rent_due: 'Échéance locative', rent_payment: 'Paiement locatif', rent_allocation: 'Affectation de loyer', rental_expense: 'Dépense immobilière', treasury_account: 'Compte ou caisse', treasury_transaction: 'Opération bancaire', account_reconciliation: 'Relevé bancaire', cash_link: 'Rapprochement bancaire', deposit_movement: 'Dépôt locatif', debt_contract: 'Contrat de financement', debt_terms: 'Conditions de financement', debt_movement: 'Mouvement de financement', debt_receivable_adjustment: 'Dépréciation de créance', debt_schedule: 'Échéance de financement', debt_payment_allocation: 'Affectation de remboursement', debt_asset_link: 'Lien de financement à un actif', debt_guarantee: 'Garantie', debt_coverage: 'Couverture de financement', finance_coverage: 'Couverture d’inventaire', rent_claim: 'Créance locative', rent_claim_adjustment: 'Ajustement de créance locative', metal_lot: 'Lot de métal', metal_movement: 'Mouvement de métal', metal_quote: 'Cours métal', metal_premium: 'Prime métal', metal_cash_link: 'Rapprochement métal', company_financial_period: 'Observation de société', company_distribution: 'Distribution de société',
};
const applicable = (link: DocumentLink, date: string) => (!link.valid_from || link.valid_from <= date) && (!link.valid_to || date < link.valid_to);
const dateLabel = (value: string) => { const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString('fr-FR'); };
const sizeLabel = (bytes: number) => bytes < 1024 ? `${bytes} octets` : `${(bytes / 1024).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Ko`;

export default function DocumentsPage({ context, onChanged, onBusyChange }: {
  context: ViewContext; onChanged: () => Promise<void>; onBusyChange: (busy: boolean) => void;
}) {
  const [library, setLibrary] = useState<DocumentLibrarySnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [description, setDescription] = useState('');
  const [query, setQuery] = useState('');
  const [status, setStatus] = useState('active');
  const [kind, setKind] = useState('');
  const [targetFilter, setTargetFilter] = useState('');
  const [editor, setEditor] = useState<Editor | null>(null);
  const [removal, setRemoval] = useState<DocumentLink | null>(null);
  const [removalReason, setRemovalReason] = useState('');
  const request = useRef(0);
  const removalFocus = useFocusedRegion(!!removal);
  const readOnly = context.knowledgeSequence != null;
  useEffect(() => { onBusyChange(loading || pending); return () => onBusyChange(false); }, [loading, pending, onBusyChange]);
  const reload = useCallback(async (reportFailure = false) => {
    const selected = ++request.current;
    setLoading(true); setLibrary(null); setError('');
    try { const result = await api.listDocumentLibraryAt(context.knowledgeSequence ?? null); if (selected === request.current) setLibrary(result); }
    catch (reason) { if (selected === request.current) { if (reportFailure) throw reason; setError(errorMessage(reason)); } }
    finally { if (selected === request.current) setLoading(false); }
  }, [context.knowledgeSequence]);
  useEffect(() => { void reload(); return () => { request.current += 1; }; }, [reload]);
  useEffect(() => { setEditor(null); setRemoval(null); setMessage(''); }, [context.knowledgeSequence, context.date]);
  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    void Promise.resolve(api.onDocumentOpenFailed(error => { if (active) setError(errorMessage(error)); }))
      .then(unlisten => { if (typeof unlisten === 'function') { if (active) stop = unlisten; else unlisten(); } })
      .catch(() => undefined);
    return () => { active = false; stop?.(); };
  }, []);
  const targetName = (id: string) => { const target = library?.targets.find(t => t.id === id); return target ? `${target.label}${target.retired ? ' · retiré' : ''}` : id; };
  const linksFor = (id: string) => library?.links.filter(link => link.document_id === id) || [];
  const currentDocument = editor && library?.documents.find(document => document.id === editor.document.id);
  const currentLink = editor?.kind === 'link' && editor.record ? library?.links.find(link => link.id === editor.record?.id) : null;
  const staleEditor = !!editor && !!library && (!currentDocument || currentDocument.revision_id !== editor.document.revision_id
    || (editor.kind === 'link' && !!editor.record && currentLink?.revision_id !== editor.record.revision_id));
  function resumeEditor() {
    if (!editor || !currentDocument || (editor.kind === 'link' && editor.record && !currentLink)) return;
    // The user explicitly adopts the reloaded source; every draft field stays intact.
    setEditor(editor.kind === 'link' ? { ...editor, document: currentDocument, record: currentLink || null }
      : { ...editor, document: currentDocument, confirmed: false });
    setError('');
    setMessage('La source du formulaire a été actualisée après comparaison. Votre saisie et votre motif sont conservés ; vérifiez-les avant d’enregistrer.');
  }
  async function changed() { await onChanged(); await reload(true); }
  async function mutate(action: () => Promise<unknown>, success: string) {
    if (readOnly) return;
    setPending(true); setError(''); setMessage('');
    try {
      if (await action() === false) { setMessage('Choix de fichier annulé ; la saisie est conservée.'); return; }
      setEditor(null); setRemoval(null); await refreshAfterCommit(changed, success, setMessage);
    } catch (reason) { setError(errorMessage(reason)); }
    finally { setPending(false); }
  }
  async function importDocument() {
    await mutate(async () => {
      const path = await api.chooseDocumentImport();
      if (typeof path !== 'string') return false;
      const result = description ? await api.importDocument(path, description) : await api.importDocument(path);
      setDescription(''); return result;
    }, 'Le document a été importé dans le coffre.');
  }
  async function readDocument(document: DocumentRecord, open: boolean) {
    setPending(true); setError(''); setMessage('');
    try {
      if (open) { await api.openDocumentRevision(document.id, document.revision_id); setMessage('L’ouverture a été demandée à l’application par défaut.'); }
      else {
        const destination = await api.chooseDocumentDestination(document.original_name);
        if (!destination) return;
        await api.exportDocumentRevision(document.id, document.revision_id, destination);
        setMessage(`Une copie de « ${document.original_name} » a été exportée.`);
      }
    } catch (reason) { setError(errorMessage(reason)); }
    finally { setPending(false); }
  }
  function edit(document: DocumentRecord, action: 'metadata' | 'replace', nextStatus = document.status) {
    if (readOnly) return;
    setError(''); setMessage(''); setRemoval(null);
    setEditor({ kind: action, document, description: document.description, status: nextStatus, reason: '', confirmed: false });
  }
  function editLink(document: DocumentRecord, record: DocumentLink | null = null) {
    if (readOnly) return;
    setError(''); setMessage(''); setRemoval(null);
    setEditor({ kind: 'link', document, record, target: record?.target_object_id || '', role: record?.role || '', from: record?.valid_from || context.date, to: record?.valid_to || '', reason: '' });
  }
  async function save(event: FormEvent) {
    event.preventDefault(); if (!editor || readOnly || staleEditor) return;
    const e = editor;
    if (e.kind === 'metadata') await mutate(() => api.saveDocumentMetadata({ id: e.document.id, expected_revision_id: e.document.revision_id, reason: e.reason.trim(), description: e.description, status: e.status, confirmed: e.status === 'deleted' && e.confirmed }), 'La description et le statut ont été enregistrés dans une nouvelle révision.');
    if (e.kind === 'replace') await mutate(async () => {
      const path = await api.chooseDocumentReplacement();
      if (typeof path !== 'string') return false;
      return api.replaceDocument({ id: e.document.id, expected_revision_id: e.document.revision_id, reason: e.reason.trim(), description: e.description }, path);
    }, 'Le nouvel original a été enregistré ; les anciennes versions restent conservées.');
    if (e.kind === 'link') await mutate(() => api.saveDocumentLink({ id: e.record?.id || null, expected_revision_id: e.record?.revision_id || null, reason: e.reason.trim(), document_id: e.document.id, target_object_id: e.target, role: e.role || null, valid_from: e.from || null, valid_to: e.to || null }), 'L’association documentaire a été enregistrée avec son historique.');
  }
  const filtered = (library?.documents || []).filter(document => {
    const links = linksFor(document.id).filter(link => applicable(link, context.date));
    return (!status || document.status === status)
      && (!query || `${document.original_name} ${document.description} ${document.mime_type}`.toLocaleLowerCase('fr').includes(query.toLocaleLowerCase('fr')))
      && (!kind || links.some(link => library?.targets.find(t => t.id === link.target_object_id)?.kind === kind))
      && (!targetFilter || links.some(link => link.target_object_id === targetFilter));
  });
  const kinds = [...new Set(library?.targets.map(t => t.kind) || [])].sort();
  return <div className="registry-workspace" aria-busy={loading || pending}>
    <RegistryNotice>La bibliothèque présente tous les documents du coffre connus à la séquence choisie. Utilisez les filtres d’association pour consulter les pièces d’un objet. Les associations suivent la date {context.date} ; les versions suivent les connaissances sélectionnées. Les originaux et leurs anciennes versions sont conservés. Ouvrir confie une copie contrôlée au lecteur associé du système.</RegistryNotice>
    {readOnly && <RegistryNotice><strong>Lecture seule historique.</strong> Documents et associations connus à la séquence {context.knowledgeSequence}. L’ouverture et l’export d’une copie restent disponibles.</RegistryNotice>}
    {error && <RegistryNotice error><strong>Opération documentaire impossible.</strong><p>{error}</p><p>Votre saisie est conservée. Après un conflit, rechargez la bibliothèque, comparez les versions puis choisissez explicitement de reprendre sur la source actualisée. Vous pouvez aussi fermer et rouvrir le formulaire depuis le document actuel.</p><button type="button" className="button secondary" disabled={pending} onClick={() => { void reload(); }}>Recharger la bibliothèque</button></RegistryNotice>}
    {message && <RegistryNotice>{message}</RegistryNotice>}
    {loading && <p role="status">Chargement des documents…</p>}
    {!loading && library && <>
      <section className="card"><div className="section-title"><div><h2>Documents du coffre</h2><p>Pièces originales, descriptions, versions et associations aux objets métier.</p></div><button type="button" className="button" disabled={pending || readOnly} onClick={() => { void importDocument(); }}>Importer un document</button></div><Field id="document-import-description" label="Description du document à importer (facultative)"><textarea id="document-import-description" disabled={pending || readOnly} maxLength={10000} value={description} onChange={event => setDescription(event.currentTarget.value)} /></Field></section>
      <section className="card registry-filters"><div className="form-grid"><Field id="document-search" label="Rechercher dans les noms et descriptions"><input id="document-search" value={query} disabled={pending} onChange={event => setQuery(event.currentTarget.value)} /></Field><Field id="document-status-filter" label="Statut des documents"><select id="document-status-filter" value={status} disabled={pending} onChange={event => setStatus(event.currentTarget.value)}><option value="">Tous les statuts</option>{Object.entries(statusNames).map(([value, name]) => <option key={value} value={value}>{name}</option>)}</select></Field><Field id="document-kind-filter" label="Type d’objet associé à la date"><select id="document-kind-filter" value={kind} disabled={pending} onChange={event => { setKind(event.currentTarget.value); setTargetFilter(''); }}><option value="">Tous les types d’objet</option>{kinds.map(value => <option key={value} value={value}>{targetKinds[value] || value}</option>)}</select></Field><Field id="document-target-filter" label="Objet associé à la date"><select id="document-target-filter" value={targetFilter} disabled={pending} onChange={event => setTargetFilter(event.currentTarget.value)}><option value="">Tous les objets</option>{library.targets.filter(t => !kind || t.kind === kind).map(target => <option value={target.id} key={target.id}>{target.label}{target.retired ? ' · retiré' : ''}</option>)}</select></Field></div></section>
      {library.documents.length === 0 ? <section className="card empty-state compact"><span className="large-symbol" aria-hidden="true">▱</span><h3>Votre espace documentaire est vide</h3><p>Importez votre première pièce. Elle restera disponible même si le fichier source est déplacé.</p></section> : filtered.length === 0 ? <section className="card"><p>Aucun document ne correspond aux filtres.</p><p>Les versions retirées restent consultables en choisissant « Tous les statuts ».</p></section> : filtered.map(document => <section className="card" key={document.id} aria-label={`Document ${document.original_name}`}>
        <div className="section-title"><div><h2>{document.original_name}</h2><span className="badge">{statusNames[document.status] || document.status}</span><p>{document.description || 'Description non renseignée.'}</p><small>{document.mime_type} · {sizeLabel(document.size_bytes)} · ajouté le {dateLabel(document.added_at)}</small></div><div className="row-actions"><button type="button" className="button secondary" disabled={pending} onClick={() => { void readDocument(document, true); }}>Ouvrir le document</button><button type="button" className="button secondary" disabled={pending} aria-label={`Exporter une copie de ${document.original_name}`} onClick={() => { void readDocument(document, false); }}>Exporter une copie</button></div></div>
        <details className="document-hash"><summary>Identité et empreinte de l’original</summary><p className="hash">Document : {document.id}<br />Révision : {document.revision_id}<br />SHA-256 : {document.sha256}</p><p>Révision enregistrée le {dateLabel(document.recorded_at)}.</p></details>
        <div className="row-actions"><button type="button" className="button secondary" disabled={pending || readOnly} onClick={() => edit(document, 'metadata')}>Modifier la description ou le statut</button><button type="button" className="button secondary" disabled={pending || readOnly || document.status === 'deleted'} onClick={() => edit(document, 'replace')}>Remplacer l’original</button><button type="button" className="button secondary" disabled={pending || readOnly || document.status === 'deleted'} onClick={() => editLink(document)}>Associer un objet</button>{document.status !== 'deleted' ? <button type="button" className="text-button" disabled={pending || readOnly} onClick={() => edit(document, 'metadata', 'deleted')}>Supprimer le document</button> : <button type="button" className="button secondary" disabled={pending || readOnly} onClick={() => edit(document, 'metadata', 'active')}>Rétablir le document</button>}</div>
        <h3>Associations documentaires</h3>
        {linksFor(document.id).length ? <div className="table-scroll"><table><thead><tr><th>Objet associé</th><th>Rôle</th><th>Période</th><th>Actions</th></tr></thead><tbody>{linksFor(document.id).map(link => <tr key={link.id}><td>{targetName(link.target_object_id)}<small>{applicable(link, context.date) ? 'Applicable à la date choisie' : 'Hors de la période choisie'}</small></td><td>{link.role || 'Non précisé'}</td><td>{link.valid_from ? `Depuis le ${link.valid_from}` : 'Sans début défini'}{link.valid_to ? `, jusqu’au ${link.valid_to} exclu` : ', sans fin définie'}</td><td><div className="row-actions"><button type="button" className="button secondary small-button" disabled={pending || readOnly} onClick={() => editLink(document, link)}>Modifier l’association</button><button type="button" className="text-button" disabled={pending || readOnly} onClick={() => { setEditor(null); setRemoval(link); setRemovalReason(''); }}>Retirer l’association</button></div></td></tr>)}</tbody></table></div> : <p className="muted">Aucune association documentaire.</p>}
        <details><summary>Versions antérieures de ce document</summary>{library.versions.filter(v => v.id === document.id && v.revision_id !== document.revision_id).map(version => <article key={version.revision_id}><h4>Version du {dateLabel(version.recorded_at)}</h4><p>{version.original_name} · {statusNames[version.status] || version.status} · {sizeLabel(version.size_bytes)}</p><p>{version.description || 'Description non renseignée.'}</p><p className="hash">Révision : {version.revision_id}<br />SHA-256 : {version.sha256}</p><div className="row-actions"><button type="button" className="button secondary small-button" disabled={pending} onClick={() => { void readDocument(version, true); }}>Ouvrir cette version</button><button type="button" className="button secondary small-button" disabled={pending} onClick={() => { void readDocument(version, false); }}>Exporter cette version</button></div></article>)}{!library.versions.some(v => v.id === document.id && v.revision_id !== document.revision_id) && <p>Aucune version antérieure connue à cette séquence.</p>}</details>
      </section>)}
      {editor && !readOnly && <EditorFrame title={editor.kind === 'link' ? 'Association documentaire' : editor.kind === 'replace' ? 'Remplacer l’original documentaire' : editor.status === 'deleted' ? 'Retirer le document du classement' : 'Description et statut du document'} pending={pending} onCancel={() => setEditor(null)}><form onSubmit={event => { void save(event); }}><fieldset disabled={pending}><p>Document : {editor.document.original_name}. La révision attendue est celle de la source affichée.</p>
        {staleEditor && <section className="notice notice-error" aria-label="Comparer les versions documentaires"><h3>La source du formulaire a changé</h3><p>L’enregistrement attend votre comparaison. La révision attendue n’a pas été modifiée et votre saisie est conservée.</p>
          <div className="table-scroll"><table><caption>Source initiale, version actuelle et saisie conservée</caption><thead><tr><th>Champ</th><th>Source initiale</th><th>Version actuelle</th><th>Votre saisie</th></tr></thead><tbody>
            <tr><th>Révision du document</th><td>{editor.document.revision_id}</td><td>{currentDocument?.revision_id || 'Document absent'}</td><td>—</td></tr>
            <tr><th>Original et empreinte</th><td>{editor.document.original_name}<p className="hash">{editor.document.sha256}</p></td><td>{currentDocument?.original_name || 'Document absent'}<p className="hash">{currentDocument?.sha256}</p></td><td>{editor.kind === 'replace' ? 'Nouvel original à choisir après validation' : 'Original conservé'}</td></tr>
            <tr><th>Description</th><td>{editor.document.description || 'Non renseignée'}</td><td>{currentDocument?.description || 'Non renseignée'}</td><td>{editor.kind === 'link' ? 'Description conservée' : editor.description || 'Non renseignée'}</td></tr>
            <tr><th>Statut</th><td>{statusNames[editor.document.status] || editor.document.status}</td><td>{currentDocument ? statusNames[currentDocument.status] || currentDocument.status : 'Document absent'}</td><td>{editor.kind === 'metadata' ? statusNames[editor.status] || editor.status : 'Statut conservé'}</td></tr>
            {editor.kind === 'link' && editor.record && <><tr><th>Révision de l’association</th><td>{editor.record.revision_id}</td><td>{currentLink?.revision_id || 'Association retirée'}</td><td>—</td></tr><tr><th>Objet associé</th><td>{targetName(editor.record.target_object_id)}</td><td>{currentLink ? targetName(currentLink.target_object_id) : 'Association retirée'}</td><td>{targetName(editor.target)}</td></tr><tr><th>Rôle</th><td>{editor.record.role || 'Non précisé'}</td><td>{currentLink?.role || 'Non précisé'}</td><td>{editor.role || 'Non précisé'}</td></tr><tr><th>Période</th><td>{editor.record.valid_from || 'Sans début'} → {editor.record.valid_to || 'Sans fin'}</td><td>{currentLink ? `${currentLink.valid_from || 'Sans début'} → ${currentLink.valid_to || 'Sans fin'}` : 'Association retirée'}</td><td>{editor.from || 'Sans début'} → {editor.to || 'Sans fin'}</td></tr></>}
          </tbody></table></div>
          {currentDocument && (editor.kind !== 'link' || !editor.record || currentLink) ? <button type="button" className="button secondary" onClick={resumeEditor}>Reprendre sur cette version en conservant ma saisie</button> : <p>Cette source a été retirée. Conservez votre saisie puis fermez le formulaire avant de choisir une autre action depuis la bibliothèque actuelle.</p>}
        </section>}
        {editor.kind === 'link' ? <><Field id="document-link-target" label="Objet métier à associer"><select id="document-link-target" required disabled={!!editor.record} value={editor.target} onChange={event => { const value = event.currentTarget.value; setEditor(e => e?.kind === 'link' ? { ...e, target: value } : e); }}><option value="">Choisir un objet…</option>{library.targets.filter(t => !t.retired || t.id === editor.target).map(target => <option key={target.id} value={target.id}>{targetKinds[target.kind] || target.kind} · {target.label}{target.retired ? ' · retiré' : ''}</option>)}</select></Field><Field id="document-link-role" label="Rôle de l’association (facultatif)"><input id="document-link-role" value={editor.role} maxLength={200} onChange={event => { const value = event.currentTarget.value; setEditor(e => e?.kind === 'link' ? { ...e, role: value } : e); }} /></Field><div className="form-grid"><Field id="document-link-from" label="Début de l’association"><input id="document-link-from" type="date" required value={editor.from} onChange={event => { const value = event.currentTarget.value; setEditor(e => e?.kind === 'link' ? { ...e, from: value } : e); }} /></Field><Field id="document-link-to" label="Fin de l’association exclue (facultative)"><input id="document-link-to" type="date" value={editor.to} onChange={event => { const value = event.currentTarget.value; setEditor(e => e?.kind === 'link' ? { ...e, to: value } : e); }} /></Field></div>{editor.record && <p>Pour changer d’objet, retirer cette association puis en créer une nouvelle. Son historique sera conservé.</p>}</> : <><Field id="document-description" label="Description du document"><textarea id="document-description" maxLength={10000} value={editor.description} onChange={event => { const value = event.currentTarget.value; setEditor(e => e && e.kind !== 'link' ? { ...e, description: value } : e); }} /></Field>{editor.kind === 'metadata' ? <Field id="document-status" label="Nouveau statut du document"><select id="document-status" value={editor.status} onChange={event => { const value = event.currentTarget.value; setEditor(e => e?.kind === 'metadata' ? { ...e, status: value, confirmed: false } : e); }}>{Object.entries(statusNames).map(([value, name]) => <option key={value} value={value}>{name}</option>)}</select></Field> : <p>Le choix du nouvel original s’ouvre après validation. Les anciens octets et leurs empreintes restent conservés.</p>}{editor.kind === 'metadata' && editor.status === 'deleted' && <><p>Ce retrait masque le document du classement actif. Ses originaux, versions et associations restent dans le coffre et ses sauvegardes.</p><label className="checkbox-label"><input id="document-delete-confirm" type="checkbox" checked={editor.confirmed} onChange={event => { const value = event.currentTarget.checked; setEditor(e => e?.kind === 'metadata' ? { ...e, confirmed: value } : e); }} />Confirmer le retrait du document du classement courant</label></>}</>}
        <Field id="document-reason" label="Motif de la modification documentaire"><textarea id="document-reason" required value={editor.reason} onChange={event => { const value = event.currentTarget.value; setEditor(e => e && ({ ...e, reason: value })); }} /></Field><button type="submit" className="button" disabled={staleEditor || !editor.reason.trim() || (editor.kind === 'metadata' && editor.status === 'deleted' && !editor.confirmed)}>{editor.kind === 'link' ? 'Enregistrer l’association documentaire' : editor.kind === 'replace' ? 'Choisir et enregistrer le nouvel original' : 'Enregistrer la description et le statut'}</button>
      </fieldset></form></EditorFrame>}
      {removal && !readOnly && <section ref={removalFocus} tabIndex={-1} className="card restore-review" aria-label="Retirer une association documentaire"><h2>Retirer une association documentaire</h2><p>Le document et ses autres associations restent conservés. L’ancienne association restera consultable dans l’historique.</p><form onSubmit={event => { event.preventDefault(); void mutate(() => api.removeDocumentLink(removal.id, removal.revision_id, removalReason.trim()), 'L’association a été retirée ; le document et ses autres associations sont conservés.'); }}><fieldset disabled={pending}><Field id="document-link-remove-reason" label="Motif du retrait de l’association"><textarea id="document-link-remove-reason" required value={removalReason} onChange={event => setRemovalReason(event.currentTarget.value)} /></Field><button type="submit" className="button danger" disabled={!removalReason.trim()}>Confirmer le retrait de l’association</button><button type="button" className="button secondary" onClick={() => setRemoval(null)}>Conserver l’association</button></fieldset></form></section>}
    </>}
  </div>;
}
