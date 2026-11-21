import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, it, vi } from 'vitest';
import App from './App';
import { api } from './services/api';
import { emptyRegistry } from './services/registry-types';
import { openModule } from './test/navigation';
vi.mock('./services/api', async importOriginal => {
 const actual = await importOriginal<typeof import('./services/api')>();
 return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key,vi.fn()])) };
});
vi.mock('./features/dashboard',()=>({ default:()=> <p>Tableau synthétique</p> }));
vi.mock('./features/financial-report',()=>({ default:()=> <p>Bilan synthétique</p> }));
vi.mock('./features/global-search',()=>({ default:()=> null }));
const alice = { id:'alice', revision_id:'alice-r5',kind:'person' as const,display_name:'Alice actuelle K5',first_name:null,last_name:null,birth_date:null,residence_country:'FR',preferred_currency:'EUR',activity:'',notes:'',archived:false };
const registry = { ...emptyRegistry,knowledge_sequence:5,entities:[alice],
 memberships:[{id:'member-alice',revision_id:'member-r1',person_id:alice.id,valid_from:'2000-01-01',valid_to:null,relationship:''}] };
const status = {generation_id:'vault',data_dir:'/local',schema_version:6,app_version:'0.1.1',document_count:0,mutation_sequence:5,country_count:4,currency_count:5,recovery_required:false,recovery_message:null,settings:{workspace_name:'Coffre de test',display_currency:'EUR' as const}};
function deferred<T>() { let resolve!:(value:T)=>void; let reject!:(reason:unknown)=>void; const promise=new Promise<T>((a,b)=>{resolve=a;reject=b}); return {promise,resolve,reject}; }
beforeEach(()=>{
 vi.resetAllMocks(); vi.mocked(api.getStatus).mockResolvedValue(status); vi.mocked(api.getSettings).mockResolvedValue(status.settings);
 vi.mocked(api.listRegistry).mockResolvedValue(registry);
 vi.mocked(api.listRegistryHistory).mockResolvedValue({latest_sequence:5,events:[4,2].map(sequence=>({sequence,recorded_at:'2026-11-01T00:00:00Z',object_id:'alice',revision_id:`r${sequence}`,kind:'entity',label:`Alice K${sequence}`,reason:'Historique',is_void:false}))});
});
it.each(['ancienne', 'récente'])('SHELL-02 garde le dernier K exact quand la réponse %s termine en premier',async first=>{
 const old=deferred<typeof registry>(),latest=deferred<typeof registry>();
 vi.mocked(api.listRegistryAt).mockImplementation(k=>k===4?old.promise:latest.promise);
 const user=userEvent.setup();render(<App/>);await screen.findByText('Coffre de test');
 await openModule(user,'Historique');
 await user.click(await screen.findByRole('button',{name:'Consulter la séquence 4'}));
 await user.click(screen.getByRole('button',{name:'Consulter la séquence 2'}));
 await user.click(screen.getByRole('button',{name:'Famille'}));
 expect(screen.queryByRole('table',{name:'Personnes, membres et tiers'})).not.toBeInTheDocument();
 const finishOld=()=>old.resolve({...registry,knowledge_sequence:4,entities:[{...alice,display_name:'Alice K4'}]});
 const finishLatest=()=>latest.resolve({...registry,knowledge_sequence:2,entities:[{...alice,display_name:'Alice K2'}]});
 if(first==='ancienne') {
   await act(async()=>finishOld());
   expect(screen.getByLabelText('Connaissances disponibles')).toHaveValue('2');
   expect(screen.getByLabelText('Connaissances disponibles')).toBeDisabled();
   expect(screen.queryByRole('table',{name:'Personnes, membres et tiers'})).not.toBeInTheDocument();
   await act(async()=>finishLatest());
 } else {
   await act(async()=>finishLatest());
   expect(within(screen.getByRole('table',{name:'Personnes, membres et tiers'})).getByText('Alice K2')).toBeVisible();
   await act(async()=>finishOld());
 }
 expect(within(screen.getByRole('table',{name:'Personnes, membres et tiers'})).getByText('Alice K2')).toBeVisible();
 expect(screen.queryByText('Alice K4')).not.toBeInTheDocument();
 expect(screen.getByLabelText('Connaissances disponibles')).toBeEnabled();
 expect(screen.getByRole('button',{name:'Créer une personne'})).toBeDisabled();
});
it.each(['calcul', 'sauvegarde'])('SHELL-01 conserve le blocage quand %s termine avant l’autre opération',async first=>{
 const calculation=deferred<never>(),saving=deferred<never>();
 vi.mocked(api.getConsolidation).mockReturnValue(calculation.promise);vi.mocked(api.saveAsset).mockReturnValue(saving.promise);
 const user=userEvent.setup();render(<App/>);await screen.findByText('Coffre de test');await openModule(user,'Patrimoine');
 await user.click(screen.getByRole('button',{name:'Créer un actif'}));
 await user.type(screen.getByLabelText('Nom de l’actif'),'Actif encore en sauvegarde');
 await user.selectOptions(screen.getByLabelText('Pays de l’actif'),'FR');
 await user.type(screen.getByLabelText('Catégorie du bien'),'autre');
 await user.type(screen.getByLabelText('Motif de l’enregistrement'),'Contrôle concurrence');
 await user.click(screen.getByRole('button',{name:'Contrôler les attributions'}));
 expect(screen.getByRole('button',{name:'Famille'})).toBeDisabled();
 await user.click(screen.getByRole('button',{name:'Créer l’enregistrement'}));
 await waitFor(()=>expect(api.saveAsset).toHaveBeenCalledTimes(1));
 const finishCalculation=()=>calculation.reject({message:'Fin contrôlée du calcul'});
 const finishSaving=()=>saving.reject({message:'Fin contrôlée de la sauvegarde'});
 await act(async()=>first==='calcul'?finishCalculation():finishSaving());
 expect(screen.getByRole('button',{name:'Famille'})).toBeDisabled();
 expect(screen.getByLabelText('Date de référence')).toBeDisabled();
 if(first==='calcul') expect(screen.getByRole('button',{name:'Enregistrement…'})).toBeDisabled();
 await act(async()=>first==='calcul'?finishSaving():finishCalculation());
 expect(screen.getByRole('button',{name:'Famille'})).toBeEnabled();
 expect(screen.getByLabelText('Date de référence')).toBeEnabled();
});
it('conserve le brouillon des paramètres après changement de page',async()=>{
 const user=userEvent.setup();render(<App/>);await screen.findByText('Coffre de test');await openModule(user,'Paramètres');
 const input=await screen.findByLabelText('Nom de l’espace familial');
 await user.clear(input);await user.type(input,'Brouillon à conserver');
 await openModule(user,'Famille');await openModule(user,'Paramètres');
 expect(await screen.findByLabelText('Nom de l’espace familial')).toHaveValue('Brouillon à conserver');expect(api.saveSettings).not.toHaveBeenCalled();
});

it('REG-03 garde une confirmation globale quand la relecture démonte le formulaire enregistré',async()=>{
 vi.mocked(api.saveAsset).mockResolvedValue({id:'asset',revision_id:'asset-r1',name:'Actif enregistré',country:'FR',category:'autre',notes:'',archived:false});
 const user=userEvent.setup();render(<App/>);await screen.findByText('Coffre de test');await openModule(user,'Patrimoine');
 await user.click(screen.getByRole('button',{name:'Créer un actif'}));
 await user.type(screen.getByLabelText('Nom de l’actif'),'Actif enregistré');
 await user.selectOptions(screen.getByLabelText('Pays de l’actif'),'FR');
 await user.type(screen.getByLabelText('Catégorie du bien'),'autre');
 await user.type(screen.getByLabelText('Motif de l’enregistrement'),'Création confirmée');
 vi.mocked(api.listRegistry).mockRejectedValue({message:'Lecture indisponible après le commit'});
 await user.click(screen.getByRole('button',{name:'Créer l’enregistrement'}));
 expect(await screen.findByText(/Les changements ont été enregistrés dans le coffre.*L’affichage n’a pas pu être actualisé/)).toHaveTextContent('Lecture indisponible après le commit');
 expect(screen.queryByRole('button',{name:'Créer l’enregistrement'})).not.toBeInTheDocument();
 expect(api.saveAsset).toHaveBeenCalledOnce();
 expect(screen.getByRole('button',{name:'Recharger le registre'})).toBeEnabled();
});

it('conserve la valeur normalisée après sauvegarde du brouillon et nouvelle navigation',async()=>{
 vi.mocked(api.saveSettings).mockResolvedValue({...status.settings,workspace_name:'Nom enregistré'});
 const user=userEvent.setup();render(<App/>);await screen.findByText('Coffre de test');await openModule(user,'Paramètres');
 const input=await screen.findByLabelText('Nom de l’espace familial');
 await user.clear(input);await user.type(input,'  Nom enregistré  ');
 await user.click(screen.getByRole('button',{name:'Enregistrer les paramètres'}));
 await screen.findByText('Les paramètres ont été enregistrés dans le coffre local.');
 expect(api.saveSettings).toHaveBeenCalledExactlyOnceWith({...status.settings,workspace_name:'Nom enregistré'});
 await openModule(user,'Famille');await openModule(user,'Paramètres');
 expect(await screen.findByLabelText('Nom de l’espace familial')).toHaveValue('Nom enregistré');
});


it('REG-03 conserve le succès des paramètres quand seul le journal échoue', async () => {
 const updated = { ...status.settings, workspace_name: 'Nom conservé' };
 vi.mocked(api.saveSettings).mockResolvedValue(updated);
 const user = userEvent.setup(); render(<App />); await screen.findByText('Coffre de test');
 await openModule(user,'Paramètres');
 const input = await screen.findByLabelText('Nom de l’espace familial');
 await user.clear(input); await user.type(input, updated.workspace_name);
 vi.mocked(api.listRegistryHistory).mockRejectedValueOnce({ message: 'Journal indisponible' });
 await user.click(screen.getByRole('button', { name: 'Enregistrer les paramètres' }));
 expect(await screen.findByText(/Les changements ont été enregistrés.*L’affichage n’a pas pu être actualisé/)).toHaveTextContent('Journal indisponible');
 expect(input).toHaveValue(updated.workspace_name);
 expect(api.saveSettings).toHaveBeenCalledOnce();
});
