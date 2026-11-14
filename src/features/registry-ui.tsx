import { useEffect, useRef } from 'react';
import type { ReactNode } from 'react';
import type { Asset, Entity, RegistrySnapshot } from '../services/registry-types';

export interface ViewContext {
  personId: string | null;
  date: string;
  knowledgeSequence?: number | null;
}

export function today(): string {
  const date = new Date();
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

export function activeAt(from: string, to: string | null, date: string): boolean {
  return from <= date && (!to || date < to);
}

// This selects descriptive metadata only. Rights and every monetary calculation stay in Rust.
export function entityAt(registry: RegistrySnapshot, entity: Entity, date: string): Entity {
  const period = registry.profile_periods.find(item => item.target_kind === 'entity' && item.target_id === entity.id && activeAt(item.valid_from, item.valid_to, date));
  return period ? [...registry.entities, ...registry.profile_revisions].find(item => item.id === entity.id && item.revision_id === period.profile_revision_id) ?? entity : entity;
}

export function assetAt(registry: RegistrySnapshot, asset: Asset, date: string): Asset {
  const period = registry.profile_periods.find(item => item.target_kind === 'asset' && item.target_id === asset.id && activeAt(item.valid_from, item.valid_to, date));
  return period ? [...registry.assets, ...registry.asset_profile_revisions].find(item => item.id === asset.id && item.revision_id === period.profile_revision_id) ?? asset : asset;
}

export function Period({ from, to }: { from: string; to: string | null }) {
  return <span>Du {from}{to ? ` au ${to} (exclu)` : ', sans date de fin'}</span>;
}

export function Field({ label, id, children, hint }: { label: string; id: string; children: ReactNode; hint?: string }) {
  return <div className="registry-field"><label htmlFor={id}>{label}</label>{children}{hint && <p className="field-hint" id={`${id}-help`}>{hint}</p>}</div>;
}

export function RegistryNotice({ children, error = false }: { children: ReactNode; error?: boolean }) {
  return <div className={`notice ${error ? 'notice-error' : 'notice-info'}`} role={error ? 'alert' : 'status'}>{children}</div>;
}

export function EditorFrame({ title, children, onCancel, pending }: { title: string; children: ReactNode; onCancel: () => void; pending: boolean }) {
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    heading.current?.focus();
    return () => {
      requestAnimationFrame(() => {
        if (previous?.isConnected && !previous.matches(':disabled')) previous.focus();
      });
    };
  }, []);
  return <section className="card registry-editor" aria-label={title} aria-busy={pending}>
    <div className="section-title"><h2 ref={heading} tabIndex={-1}>{title}</h2><button type="button" className="button secondary small-button" disabled={pending} onClick={onCancel}>Fermer le formulaire</button></div>
    {children}
  </section>;
}
