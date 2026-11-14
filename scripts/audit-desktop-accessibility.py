#!/usr/bin/env python3
"""Audit axe-core dans WebKit natif sur une copie d’un coffre synthétique.

Le test charge une sauvegarde explicitement fournie, vérifie les fichiers puis
ouvre sa copie isolée. Il ne qualifie pas la restauration et ne soumet aucun
formulaire métier. Le moteur axe est une dépendance de développement locale.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time
import uuid
import zipfile


def file_hash(path):
    checksum = hashlib.sha256()
    with path.open('rb') as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b''):
            checksum.update(chunk)
    return checksum.hexdigest()


def private_file(path, content):
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    with path.open('xb') as target:
        target.write(content)
    path.chmod(0o600)


def load_synthetic_copy(backup, destination):
    with zipfile.ZipFile(backup) as source:
        manifest = json.loads(source.read('manifest.json'))
        generation = str(uuid.UUID(manifest['generation_id']))
        assert manifest['schema_version'] == 6 and manifest['format_version'] == 1
        root = destination / 'generations' / generation
        root.mkdir(parents=True, mode=0o700)
        for item in manifest['files']:
            path = item['path']
            raw = source.read(path)
            assert len(raw) == item['size_bytes'] and hashlib.sha256(raw).hexdigest() == item['sha256']
            if path == 'data/patrimoine.sqlite3':
                private_file(root / 'patrimoine.sqlite3', raw)
            elif path.startswith('documents/sha256/'):
                pieces = Path(path).parts
                assert len(pieces) == 4 and pieces[2] == pieces[3][:2] and len(pieces[3]) == 64
                assert all(character in '0123456789abcdef' for character in pieces[3])
                private_file(root / path, raw)
        private_file(destination / 'CURRENT', (generation + '\n').encode())
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('application', type=Path)
    parser.add_argument('--backup', required=True, type=Path)
    parser.add_argument('--axe', required=True, type=Path)
    parser.add_argument('--artifacts', required=True, type=Path)
    parser.add_argument('--driver-module', type=Path, default=Path(__file__).with_name('desktop-smoke.py'))
    parser.add_argument('--adapter', type=Path, default=Path(__file__).with_name('accessibility-audit.js'))
    parser.add_argument('--width', type=int, default=1280)
    parser.add_argument('--height', type=int, default=860)
    args = parser.parse_args()
    if args.width < 900 or args.height < 650:
        parser.error('La fenêtre qualifiée doit mesurer au moins 900 × 650 pixels.')
    args.artifacts.mkdir(parents=True, exist_ok=True)
    module_spec = importlib.util.spec_from_file_location('native_driver', args.driver_module)
    driver_module = importlib.util.module_from_spec(module_spec)
    module_spec.loader.exec_module(driver_module)
    driver = driver_module.WebDriver()
    process = None
    report = {'started_at': datetime.now(timezone.utc).isoformat(), 'status': 'failed', 'pages': [],
              'fixture': str(args.backup), 'application': str(args.application),
              'application_sha256': file_hash(args.application), 'fixture_sha256': file_hash(args.backup),
              'axe_sha256': file_hash(args.axe), 'adapter_sha256': file_hash(args.adapter),
              'requested_window': {'width': args.width, 'height': args.height},
              'method': 'Real Tauri/WebKitGTK rendering, axe-core WCAG 2.0/2.1 A/AA rules; no mocked IPC',
              'limit': 'Automated findings and selected keyboard checks only; no complete WCAG or screen-reader certification.'}
    with tempfile.TemporaryDirectory(prefix='patrimoine-accessibility-') as directory:
        vault = Path(directory) / 'vault'
        vault.mkdir(mode=0o700)
        manifest = load_synthetic_copy(args.backup, vault)
        report['source_knowledge_sequence'] = manifest['mutation_sequence']
        environment = {**os.environ, 'PATRIMOINE_DATA_DIR': str(vault)}
        with (args.artifacts / 'tauri-driver.log').open('w') as logfile:
            try:
                process = subprocess.Popen(['tauri-driver', '--port', '4444', '--native-port', '4445'], env=environment,
                                           stdout=logfile, stderr=subprocess.STDOUT, start_new_session=True)
                driver_module.wait_until(lambda: driver.request('GET', '/status', timeout=2), 'pilote natif disponible', timeout=20)
                driver.start(args.application)
                driver.command('POST', '/window/rect', {'width': args.width, 'height': args.height})
                driver_module.wait_until(lambda: driver.execute("return document.querySelector('h1')?.textContent==='Tableau de bord';"), 'fenêtre disponible', timeout=30)
                status = driver.invoke('get_status')
                report['initial_window'] = driver.execute('return {inner_width:window.innerWidth,inner_height:window.innerHeight};')
                assert status['mutation_sequence'] == manifest['mutation_sequence']
                axe_source = args.axe.read_text()
                adapter = args.adapter.read_text()

                def settle():
                    driver_module.wait_until(lambda: driver.execute("return ![...document.querySelectorAll('p[role=status]')].some(p=>/^(Chargement|Ouverture|Lecture des sources)/.test(p.innerText));"), 'vue chargée', timeout=60)
                    driver.command('POST', '/execute/async', {'script': 'const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));', 'args': []})

                def navigate(label):
                    # At the minimum window height, native WebDriver's implicit
                    # scroll can move the sidebar during its click. Settle the
                    # real scroll first, then keep an actual WebDriver click.
                    driver.execute("const button=[...document.querySelectorAll('nav button')].find(n=>n.textContent.includes(arguments[0]));button.scrollIntoView({block:'nearest',inline:'nearest',behavior:'instant'});", label)
                    driver.command('POST', '/execute/async', {'script': 'const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));', 'args': []})
                    driver_module.navigate(driver, label)
                    driver_module.wait_until(lambda: driver.execute("return document.querySelector('h1')?.textContent;") == label, 'page demandée affichée')

                def audit(label):
                    settle()
                    expanded = driver.execute("const nodes=[...document.querySelectorAll('details')];nodes.forEach(node=>node.open=true);return nodes.length;")
                    driver.execute(axe_source)
                    driver.execute(adapter)
                    result = driver.command('POST', '/execute/async', {'script': 'const done=arguments[arguments.length-1];window.auditPatrimoineAccessibility().then(r=>done({ok:true,result:r}),e=>done({ok:false,error:String(e)}));', 'args': []})
                    assert result['ok'], result
                    entry = {'page': label, 'expanded_details': expanded, **result['result']}
                    entry['window'] = driver.execute('return {inner_width:window.innerWidth,inner_height:window.innerHeight};')
                    entry['page_width'] = driver.execute('return {scroll_width:document.documentElement.scrollWidth,viewport_width:window.innerWidth};')
                    # axe cannot judge some decorative symbol glyphs. Keep its
                    # incomplete result verbatim, and record the actual DOM
                    # checks used to assess these individually without hiding
                    # any failed rule or generalising to other incomplete nodes.
                    entry['reviewed_incomplete'] = []
                    for item in entry['incomplete']:
                        for node in item['nodes']:
                            if item['id'] != 'color-contrast' or len(node['target']) != 1:
                                continue
                            assessment = driver.execute('''
                                const node=document.querySelector(arguments[0]);
                                if(!node || node.getAttribute('aria-hidden')!=='true')return null;
                                const button=node.closest('button.nav-item');
                                if(!button)return null;
                                const text=button.cloneNode(true);
                                text.querySelectorAll('[aria-hidden="true"]').forEach(n=>n.remove());
                                if(!text.textContent.trim())return null;
                                const style=getComputedStyle(node),labelStyle=getComputedStyle(button);
                                let background=button;
                                while(background && getComputedStyle(background).backgroundColor==='rgba(0, 0, 0, 0)')background=background.parentElement;
                                return {decorative_aria_hidden:true,adjacent_text_label:text.textContent.trim(),
                                  symbol_color:style.color,label_color:labelStyle.color,
                                  background_color:background?getComputedStyle(background).backgroundColor:null};
                            ''', node['target'][0])
                            if assessment:
                                entry['reviewed_incomplete'].append({'rule': item['id'], 'target': node['target'],
                                    'assessment': 'Decorative navigation symbol hidden from assistive technology; adjacent text supplies the button name and is checked separately by axe.',
                                    'dom': assessment})
                    report['pages'].append(entry)
                    print(json.dumps({'page': label, 'violations': [(item['id'], len(item['nodes'])) for item in entry['violations']], 'incomplete': len(entry['incomplete'])}), flush=True)

                for page in ['Tableau de bord', 'Famille', 'Patrimoine', 'Immobilier et loyers', 'Comptes bancaires', 'Espèces et devises', 'Or et argent', 'Sociétés', 'Dettes et financements', 'Documents', 'Historique', 'Sauvegarde et export', 'Paramètres']:
                    navigate(page)
                    audit(page)
                navigate('Famille')
                driver.click(driver_module.button('Créer une personne'))
                driver_module.wait_until(lambda: driver.execute("return document.activeElement?.matches('.registry-editor h2');"), 'titre du formulaire annoncé au clavier')
                audit('Formulaire personne')
                driver.click(driver_module.button('Fermer le formulaire'))
                driver_module.wait_until(lambda: driver.execute("return document.activeElement?.textContent==='Créer une personne';"), 'retour au bouton de création')
                navigate('Documents')
                driver.click(driver_module.button('Modifier la description ou le statut'))
                audit('Formulaire documentaire')
                driver.click(driver_module.button('Fermer le formulaire'))
                driver.click(driver_module.button('Rechercher dans le coffre'))
                settle()
                driver_module.wait_until(lambda: driver.execute("return document.activeElement?.id==='global-search-query';"), 'focus dans la recherche')
                audit('Recherche globale ouverte')
                search = driver.find('#global-search-query', 'css selector')
                driver.command('POST', f'/element/{search}/value', {'text': '\ue00c', 'value': ['\ue00c']})
                driver_module.wait_until(lambda: driver.execute("return document.activeElement?.classList.contains('global-search-toggle') && !document.querySelector('#global-search-panel');"), 'Échap ferme et rend le focus')
                navigate('Tableau de bord')
                settle()
                driver.execute("document.querySelector('.skip-link').focus();")
                assert driver.execute("return document.querySelector('.skip-link').getBoundingClientRect().top >= 0;")
                skip = driver.find('.skip-link', 'css selector')
                driver.command('POST', f'/element/{skip}/value', {'text': '\ue007', 'value': ['\ue007']})
                driver_module.wait_until(lambda: driver.execute("return document.activeElement?.id==='content';"), 'lien clavier vers le contenu principal')
                driver.command('POST', '/window/rect', {'width': 900, 'height': 650})
                audit('Tableau de bord · fenêtre minimale 900 × 650')
                assert driver.execute("return document.documentElement.scrollWidth <= window.innerWidth;"), 'Débordement horizontal global à la largeur minimale'
                driver.screenshot(args.artifacts / 'minimum-window.png')
                report['keyboard'] = {'skip_link_visible_and_enter_to_main': True, 'minimum_window_no_page_overflow': True, 'inline_editor_heading_focus': True, 'inline_editor_close_focus_return': True,
                                      'search_initial_focus': True, 'search_escape_and_focus_return': True}
                assert driver.invoke('get_status')['mutation_sequence'] == status['mutation_sequence']
                report['business_sequence_unchanged'] = True
                report['violation_count'] = sum(len(page['violations']) for page in report['pages'])
                report['page_overflow_count'] = sum(page['page_width']['scroll_width'] > page['page_width']['viewport_width'] for page in report['pages'])
                report['status'] = 'passed' if report['violation_count'] == 0 and report['page_overflow_count'] == 0 else 'violations'
            except Exception as error:
                report['error'] = f'{type(error).__name__}: {error}'
                if driver.session_id:
                    try:
                        (args.artifacts / 'failure.txt').write_text(driver.text())
                        driver.screenshot(args.artifacts / 'failure.png')
                    except Exception:
                        pass
            finally:
                try:
                    driver.close()
                finally:
                    if process:
                        try:
                            os.killpg(process.pid, signal.SIGTERM)
                            process.wait(timeout=5)
                        except ProcessLookupError:
                            pass
                        except subprocess.TimeoutExpired:
                            os.killpg(process.pid, signal.SIGKILL)
        report['completed_at'] = datetime.now(timezone.utc).isoformat()
        (args.artifacts / 'accessibility-result.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'status': report['status'], 'pages': len(report['pages']), 'violations': report.get('violation_count'), 'error': report.get('error')}, ensure_ascii=False))
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
