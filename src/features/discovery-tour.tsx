import { useId, useLayoutEffect, useRef, useState } from 'react';
import './discovery-tour.css';

export type TourStepId = 'overview' | 'family' | 'property' | 'rentals' | 'bank' | 'metals' | 'company' | 'backup';
export type TourDestination = Exclude<TourStepId, 'company'> | 'companies';

interface TourStep {
  id: TourStepId;
  destination: TourDestination;
  title: string;
  description: string;
  instructions: string[];
  illustration: string;
  action: string;
}

const steps: TourStep[] = [
  {
    id: 'overview', destination: 'overview', title: 'Comprendre le tableau de bord',
    description: 'Le patrimoine brut rassemble les biens et les droits. Déduisez les dettes et les autres obligations pour obtenir le patrimoine net.',
    instructions: [
      'Dans « Périmètre de consultation », choisissez la famille entière ou une personne pour voir sa part.',
      'La répartition distingue notamment l’immobilier, les comptes, les espèces, les métaux et les participations. En cas de données manquantes, les sous-totaux connus et les alertes restent visibles.',
    ],
    illustration: 'Le choix du périmètre et les cartes Patrimoine brut, Dettes et obligations, Patrimoine net.', action: 'Ouvrir le tableau de bord',
  },
  {
    id: 'family', destination: 'family', title: 'Ajouter un membre de la famille',
    description: 'Une fiche personnelle vous permet de rattacher les biens et les comptes à la bonne personne.',
    instructions: [
      'Dans « Famille », cliquez sur « Ajouter un membre de la famille ». Saisissez le prénom et le nom. Le pays de résidence est facultatif.',
      'Cliquez sur « Enregistrer le membre », puis utilisez « Voir » dans la liste pour retrouver sa fiche.',
    ],
    illustration: 'L’ajout d’un membre et son accès depuis la liste des personnes.', action: 'Ouvrir la famille',
  },
  {
    id: 'property', destination: 'property', title: 'Ajouter un bien immobilier',
    description: 'Vous pouvez créer la fiche depuis « Immobilier et loyers » et compléter les autres informations plus tard.',
    instructions: [
      'Cliquez sur « Ajouter un bien immobilier ». Renseignez le nom, le type, le pays et l’état du bien. Indiquez la ville et l’estimation si vous les connaissez.',
      'Choisissez un propriétaire unique uniquement s’il détient la totalité des droits. Pour une propriété partagée, ouvrez le registre patrimonial depuis les options du bien après l’ajout.',
      'Enregistrez le bien puis cliquez sur « Voir » dans « Biens et unités » pour retrouver sa fiche.',
    ],
    illustration: 'Le formulaire d’un bien, le propriétaire et l’accès à la fiche enregistrée.', action: 'Ouvrir l’immobilier',
  },
  {
    id: 'rentals', destination: 'rentals', title: 'Gérer les loyers',
    description: 'Le bail décrit la location. Les échéances sont les sommes attendues. Les paiements correspondent à l’argent reçu.',
    instructions: [
      'Après avoir ajouté une unité à votre bien et créé la fiche du locataire, ouvrez « Baux et échéances ». Ajoutez le bail, puis ses conditions de loyer et de charges.',
      'Cliquez sur « Générer les échéances », choisissez le bail et la période à préparer.',
      'Dans « Paiements et dépenses », utilisez « Enregistrer un paiement » puis « Affecter un paiement » pour indiquer les échéances réglées.',
    ],
    illustration: 'Les actions pour créer le bail, générer les échéances et saisir le paiement reçu.', action: 'Ouvrir les baux et échéances',
  },
  {
    id: 'bank', destination: 'bank', title: 'Ajouter un compte bancaire',
    description: 'Le solde de départ correspond à l’argent sur le compte au premier jour du suivi.',
    instructions: [
      'Cliquez sur « Ajouter un compte bancaire ». Saisissez le nom du compte, son pays et son solde de départ dans la devise du compte. Choisissez son titulaire.',
      'Enregistrez le compte. Dans la liste, utilisez « Voir les mouvements » pour retrouver les entrées, les sorties et les virements.',
    ],
    illustration: 'La devise du solde et les actions de consultation du compte.', action: 'Ouvrir les comptes bancaires',
  },
  {
    id: 'metals', destination: 'metals', title: 'Ajouter de l’or ou de l’argent',
    description: 'Un lot rassemble des objets identiques. Le poids fin tient compte de la quantité, du poids par objet et de la pureté.',
    instructions: [
      'Cliquez sur « Ajouter un lot de métal ». Saisissez ses caractéristiques, son propriétaire et son lieu de conservation. Une pureté de 0,9 correspond à 90 % de métal fin.',
      'Renseignez un cours daté dans « Cours et primes ». Depuis le lot, cliquez sur « Voir la valeur » puis « Calculer le lot de métal » pour utiliser le cours applicable au jour choisi.',
    ],
    illustration: 'Le poids, la pureté et la consultation de la valeur d’un lot de métal.', action: 'Ouvrir l’or et l’argent',
  },
  {
    id: 'company', destination: 'companies', title: 'Gérer une société',
    description: 'La société a sa propre fiche, ses biens et ses dettes. Une participation relie la société à un associé et décrit les droits de celui-ci.',
    instructions: [
      'Ajoutez la société. Vous pouvez déclarer les associés connus depuis « Associés (facultatif) ».',
      'Pour une fiche existante, ouvrez « Fiches et participations » puis cliquez sur « Ajouter une participation ». Choisissez la société, l’associé et la date de début. Renseignez les parts connues.',
    ],
    illustration: 'La société et la section Associés et participations.', action: 'Ouvrir les sociétés',
  },
  {
    id: 'backup', destination: 'backup', title: 'Sauvegarder ses données',
    description: 'Une sauvegarde crée une copie du coffre. Une restauration remplace le coffre actuel par le contenu d’une archive après vérification et confirmation.',
    instructions: [
      '« Créer une sauvegarde complète » produit une archive ZIP avec l’historique et les documents originaux.',
      'Exportez un classeur XLSX pour consulter les données dans un tableur, ou une archive CSV intégrale pour conserver aussi les originaux et pouvoir reconstruire le coffre.',
      'Pour restaurer, choisissez une sauvegarde à analyser. Lisez le résumé avant de confirmer le remplacement. Vous pouvez aussi conserver une copie du coffre actuel.',
    ],
    illustration: 'Les trois sections Sauvegarder, Exporter et Restaurer.', action: 'Ouvrir la page Sauvegarde et export',
  },
];

const illustrations = import.meta.glob<string>('./discovery-screens/*.webp', { eager: true, import: 'default', query: '?url' });

export default function DiscoveryTour({ initialStep, onClose, onNavigate }: {
  initialStep: TourStepId;
  onClose: () => void;
  onNavigate: (destination: TourDestination) => void;
}) {
  const [index, setIndex] = useState(() => Math.max(0, steps.findIndex(step => step.id === initialStep)));
  const step = steps[index];
  const last = index === steps.length - 1;
  const label = useId();
  const dialog = useRef<HTMLDialogElement>(null);
  const body = useRef<HTMLDivElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const openedFrom = useRef<HTMLElement | null>(null);
  const navigating = useRef(false);
  const illustration = illustrations[`./discovery-screens/${step.id}.webp`];

  useLayoutEffect(() => {
    const modal = dialog.current;
    const active = document.activeElement;
    if (!openedFrom.current && active instanceof HTMLElement && !modal?.contains(active)) openedFrom.current = active;
    modal?.showModal();
    return () => {
      if (modal?.open) modal.close();
      requestAnimationFrame(() => {
        if (document.querySelector('dialog[open]')) return;
        const previous = openedFrom.current;
        if (!navigating.current && previous?.isConnected && previous !== document.body) previous.focus();
        else document.querySelector<HTMLElement>('main h1')?.focus();
      });
    };
  }, []);

  useLayoutEffect(() => {
    if (body.current) body.current.scrollTop = 0;
    heading.current?.focus({ preventScroll: true });
  }, [index]);

  return <dialog className="discovery-dialog" ref={dialog} aria-labelledby={label}
    onCancel={event => { event.preventDefault(); onClose(); }}
    onKeyDown={event => {
      if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
      if (event.key === 'ArrowLeft' && index > 0) {
        event.preventDefault(); setIndex(index - 1);
      } else if (event.key === 'ArrowRight' && !last) {
        event.preventDefault(); setIndex(index + 1);
      }
    }}>
    <header className="discovery-header">
      <div className="discovery-header-row">
        <h2 id={label}>Découvrir Patrimoine</h2>
        <button type="button" className="button secondary small-button" onClick={onClose}>Fermer le tour</button>
      </div>
      <div className="discovery-progress">
        <span>Étape {index + 1} sur {steps.length}</span>
        <progress value={index + 1} max={steps.length} aria-label="Avancement du tour" />
      </div>
    </header>
    <div className="discovery-body" ref={body}>
      <h3 ref={heading} tabIndex={-1}>{step.title}</h3>
      <p>{step.description}</p>
      <ol>{step.instructions.map(instruction => <li key={instruction}>{instruction}</li>)}</ol>
      {illustration && <figure>
        <img src={illustration} alt={step.illustration} decoding="async" />
        <figcaption><strong>Exemple fictif.</strong> Les encadrés indiquent les éléments à repérer.</figcaption>
      </figure>}
    </div>
    <footer className="discovery-footer">
      <button type="button" className="text-button discovery-destination" onClick={() => {
        navigating.current = true;
        onClose();
        onNavigate(step.destination);
      }}>{step.action}</button>
      <button type="button" className="button secondary" disabled={index === 0} onClick={() => setIndex(index - 1)}>Précédent</button>
      <button type="button" className="button" onClick={last ? onClose : () => setIndex(index + 1)}>{last ? 'Terminer' : 'Suivant'}</button>
    </footer>
  </dialog>;
}
