#!/usr/bin/env python3
"""Suit l’assistant en français dans le vrai WebKit de Patrimoine, hors ligne.

  python3 desktop-onboarding.py /app/Patrimoine.AppImage --artifacts /artifacts

Nécessite un écran X11 (Xvfb convient), WebKitWebDriver et tauri-driver.
Chaque scénario utilise un coffre temporaire et des données de test.
"""

import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
from importlib.machinery import SourceFileLoader
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import subprocess
import sys
import tempfile


smoke = SourceFileLoader("desktop_smoke", str(Path(__file__).with_name("desktop-smoke.py"))).load_module()

WELCOME = "Commencer la configuration"
SKIP_ALL = "Passer et accéder à l’application"
FINISH = "Accéder à mon patrimoine"


def visible_dialog(driver):
    return driver.execute("return !!document.querySelector('.onboarding-dialog[open]');")


def click_in_dialog(driver, xpath):
    # The footer stays in view while the wizard body scrolls independently.
    driver.execute("""
        const target = document.evaluate(arguments[0], document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue;
        (target?.closest('select') ?? target)?.scrollIntoView({block:'center', inline:'nearest', behavior:'instant'});
    """, xpath)
    try:
        driver.click(xpath)
    except RuntimeError as error:
        raise RuntimeError(f"Action dans l’assistant {xpath} : {error}") from error


def dialog_button(driver, label):
    xpath = f"//dialog[@open]//button[normalize-space(.)={json.dumps(label, ensure_ascii=False)}]"
    click_in_dialog(driver, xpath)


def control(driver, label):
    def find():
        return driver.execute("""
            const label = [...document.querySelectorAll('dialog[open] label')].find(
                label => label.textContent.trim() === arguments[0]);
            return label?.control && !label.control.matches(':disabled') ? label.control.id : null;
        """, label)
    return smoke.wait_until(find, f"champ de l’assistant {label}")


def fill(driver, label, text):
    element = control(driver, label)
    selector = f'[id={json.dumps(element)}]'
    driver.fill(selector, text)


def select(driver, label, value):
    element = control(driver, label)
    click_in_dialog(driver, f"//select[@id={json.dumps(element)}]/option[@value={json.dumps(value)}]")
    smoke.wait_until(lambda: driver.execute("return document.getElementById(arguments[0]).value;", element) == value,
                     f"sélection dans {label}")


def step(driver, number):
    smoke.wait_text(driver, f"Étape {number} sur 7")


def next_step(driver):
    dialog_button(driver, "Continuer")


def skip_step(driver):
    dialog_button(driver, "Passer cette étape")


def start_wizard(driver):
    smoke.wait_text(driver, WELCOME)
    step(driver, 1)
    dialog_button(driver, WELCOME)
    step(driver, 2)


def family(driver, first, last):
    fill(driver, "Prénom", first)
    fill(driver, "Nom", last)
    dialog_button(driver, "Enregistrer le membre")
    expected = f"{first} {last}"
    smoke.wait_text(driver, f"« {expected} » a été enregistré.")

    def saved_person():
        entities = driver.invoke("list_registry")["entities"]
        rows = [entity for entity in entities if entity["display_name"] == expected]
        smoke.require(len(rows) <= 1, f"Doublon de la personne {expected}")
        return rows[0]["id"] if rows else None

    return smoke.wait_until(saved_person, f"membre enregistré {expected}")


def property_form(driver, name, value, owner=None):
    fill(driver, "Nom du bien", name)
    select(driver, "Type de bien", "apartment")
    select(driver, "Pays du bien", "FR")
    fill(driver, "Ville (facultatif)", "Lyon")
    select(driver, "État du bien", "rented")
    fill(driver, "Valeur estimée (facultatif)", value)
    if owner:
        select(driver, "Propriétaire unique (facultatif)", owner)


def assert_sequence(driver, expected):
    actual = driver.invoke("get_status")["mutation_sequence"]
    smoke.require(actual == expected, f"Nombre d’enregistrements dans l’historique : {actual}, attendu {expected}")


def durable_configuration(driver):
    generation = driver.invoke("get_status")["generation_id"]
    raw = driver.invoke("read_onboarding_state", {"generationId": generation})
    return json.loads(raw) if raw else {}


@contextmanager
def session(application, artifacts, temporary, vault_name, profile_name=None, proof_prefix="onboarding"):
    profile = temporary / "profiles" / (profile_name or vault_name)
    environment = os.environ.copy()
    for key, subdir in (("XDG_DATA_HOME", "data"), ("XDG_CACHE_HOME", "cache"), ("XDG_CONFIG_HOME", "config")):
        directory = profile / subdir
        directory.mkdir(parents=True, exist_ok=True)
        environment[key] = str(directory)
    environment["PATRIMOINE_DATA_DIR"] = str(temporary / "vaults" / vault_name)
    with socket.socket() as service, socket.socket() as native:
        service.bind(('127.0.0.1', 0))
        native.bind(('127.0.0.1', 0))
        port = service.getsockname()[1]
        native_port = native.getsockname()[1]
    driver = smoke.WebDriver(port)
    with (artifacts / f"{proof_prefix}-driver.log").open("a", encoding="utf-8") as log:
        binary = shutil.which("tauri-driver")
        smoke.require(binary, "tauri-driver absent du PATH")
        process = subprocess.Popen([binary, "--port", str(port), "--native-port", str(native_port)],
                                   env=environment, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            smoke.wait_until(lambda: driver.request("GET", "/status", timeout=2), "démarrage de tauri-driver", timeout=20)
            driver.start(application)
            smoke.wait_until(lambda: driver.execute("return !!window.__TAURI_INTERNALS__ && document.querySelector('h1')?.textContent === 'Tableau de bord';"),
                             "fenêtre de Patrimoine prête", timeout=25)
            smoke.wait_text(driver, "Accessible", timeout=30)
            yield driver
        except Exception:
            if driver.session_id:
                try:
                    (artifacts / f"{proof_prefix}-failure.txt").write_text(driver.text(), encoding="utf-8")
                    driver.screenshot(artifacts / f"{proof_prefix}-failure.png")
                except Exception:
                    pass
            raise
        finally:
            try:
                driver.close()
            finally:
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=5)


def scenarios(application, previous_application, artifacts, temporary, results):
    def launch(vault, profile=None, executable=application):
        return session(executable, artifacts, temporary, vault, profile)

    def passed(name):
        results.append({"name": name, "status": "passed"})
        print(name, flush=True)

    with launch("skipped") as driver:
        assert_sequence(driver, 0)
        step(driver, 1)
        driver.screenshot(artifacts / "onboarding-welcome.png")
        dialog_button(driver, SKIP_ALL)
        smoke.wait_until(lambda: not visible_dialog(driver), "assistant passé")
        smoke.require(durable_configuration(driver).get("disposition") == "skipped", "Le choix de passer l’assistant reste à écrire")
        smoke.require(not driver.invoke("list_registry")["entities"], "Le premier lancement a ajouté des personnes")
        assert_sequence(driver, 0)
    with launch("skipped") as driver:
        smoke.require(not visible_dialog(driver), "L’assistant ignoré réapparaît au démarrage")
        assert_sequence(driver, 0)
    passed("new_empty_vault_and_skip_persists_on_restart")

    with launch("single") as driver:
        start_wizard(driver)
        person = family(driver, "Lina", "Exemple")
        next_step(driver)
        step(driver, 3)
        dialog_button(driver, "Précédent")
        step(driver, 2)
        smoke.wait_text(driver, "Lina Exemple")
        next_step(driver)
        step(driver, 3)
        dialog_button(driver, "Quitter et reprendre plus tard")
        smoke.wait_until(lambda: not visible_dialog(driver), "assistant mis en pause")
    with launch("single") as driver:
        smoke.require(not visible_dialog(driver), "L’assistant mis en pause s’ouvre sans demande")
        smoke.navigate(driver, "Paramètres")
        driver.click(smoke.button("Reprendre la configuration"))
        for number in range(3, 7):
            step(driver, number)
            skip_step(driver)
        step(driver, 7)
        smoke.wait_text(driver, "Lina Exemple")
        dialog_button(driver, FINISH)
        smoke.wait_until(lambda: not visible_dialog(driver), "fin de l’assistant")
        assert_sequence(driver, 1)
        smoke.navigate(driver, "Famille")
        smoke.wait_text(driver, "Lina Exemple")
        smoke.require(driver.invoke("list_registry")["memberships"][0]["person_id"] == person,
                      "La personne a été enregistrée sans son appartenance à la famille")
    passed("one_step_then_pause_manual_resume_and_member_visible_in_family")

    with launch("all") as driver:
        start_wizard(driver)
        person = family(driver, "Camille", "Exemple")
        family(driver, "Alex", "Exemple")
        next_step(driver)
        step(driver, 3)
        property_form(driver, "Appartement Exemple", "-12", person)
        dialog_button(driver, "Enregistrer le bien")
        smoke.wait_text(driver, "La cible ou le signe de la valorisation est invalide.")
        assert_sequence(driver, 2)
        smoke.require(not driver.invoke("list_registry")["assets"], "Le bien invalide a laissé un actif orphelin")
        driver.screenshot(artifacts / "onboarding-invalid.png")
        fill(driver, "Valeur estimée (facultatif)", "180000")
        smoke.wait_until(lambda: durable_configuration(driver).get("drafts", {}).get("property", {}).get("values", {}).get("amount") == "180000",
                         "coffre temporaire prêt à reprendre la saisie")
        generation = driver.invoke("get_status")["generation_id"]
        first_session = driver.session_id
    passed("invalid_later_entry_keeps_previously_saved_members")

    with launch("all") as driver:
        smoke.require(driver.session_id != first_session, "Le test n’a pas lancé de nouvelle fenêtre")
        smoke.require(driver.invoke("get_status")["generation_id"] == generation, "Le coffre de reprise a changé")
        step(driver, 3)
        name = control(driver, "Nom du bien")
        smoke.require(driver.execute("return document.getElementById(arguments[0]).value;", name) == "Appartement Exemple",
                      "La saisie immobilière non validée n’a pas repris après la fermeture")
        assert_sequence(driver, 2)
        driver.screenshot(artifacts / "onboarding-resumed.png")
        dialog_button(driver, "Enregistrer le bien")
        smoke.wait_until(lambda: driver.invoke("get_status")["mutation_sequence"] == 3, "bien ajouté une fois")
        next_step(driver)
        step(driver, 4)
        fill(driver, "Banque ou nom du compte", "Banque Exemple")
        select(driver, "Pays du compte", "FR")
        fill(driver, "Solde de départ", "1250,50")
        select(driver, "Devise du solde", "USD")
        select(driver, "Propriétaire unique (100 %)", person)
        dialog_button(driver, "Enregistrer le compte")
        smoke.wait_text(driver, "« Banque Exemple » a été enregistré.")
        assert_sequence(driver, 4)
        dialog_button(driver, "Caisse d’espèces")
        fill(driver, "Nom de la caisse", "Caisse Exemple")
        select(driver, "Pays de conservation", "FR")
        fill(driver, "Lieu de conservation", "Coffre Exemple")
        fill(driver, "Solde de départ", "0")
        select(driver, "Propriétaire unique (100 %)", person)
        dialog_button(driver, "Enregistrer la caisse")
        smoke.wait_text(driver, "« Caisse Exemple » a été enregistré.")
        assert_sequence(driver, 5)
        next_step(driver)

        step(driver, 5)
        select(driver, "Métal", "gold")
        select(driver, "Forme", "coin")
        fill(driver, "Nom du lot", "Pièces Exemple")
        fill(driver, "Quantité", "2")
        fill(driver, "Poids d’un objet", "10")
        fill(driver, "Pureté", "0,9")
        select(driver, "Propriétaire unique (100 %)", person)
        select(driver, "Pays de conservation", "FR")
        fill(driver, "Lieu de conservation", "Coffre Exemple")
        dialog_button(driver, "Enregistrer le lot")
        smoke.wait_text(driver, "« Pièces Exemple » a été enregistré.")
        assert_sequence(driver, 6)
        next_step(driver)

        step(driver, 6)
        fill(driver, "Nom de la société", "Atelier Exemple")
        select(driver, "Pays de la société", "FR")
        fill(driver, "Activité (facultatif)", "Conseil")
        click_in_dialog(driver, "//dialog[@open]//summary[normalize-space(.)='Associés (facultatif)']")
        dialog_button(driver, "Ajouter un associé")
        select(driver, "Personne ou société", person)
        dialog_button(driver, "Enregistrer la société")
        smoke.wait_text(driver, "« Atelier Exemple » a été enregistré.")
        assert_sequence(driver, 7)
        next_step(driver)

        step(driver, 7)
        for name in ("Camille Exemple", "Alex Exemple", "Appartement Exemple", "Banque Exemple", "Caisse Exemple", "Pièces Exemple", "Atelier Exemple"):
            smoke.wait_text(driver, name)
        driver.screenshot(artifacts / "onboarding-summary.png")
        dialog_button(driver, FINISH)
        smoke.wait_until(lambda: not visible_dialog(driver), "assistant terminé")
        registry = driver.invoke("list_registry")
        smoke.require(len(registry["memberships"]) == 2 and len(registry["assets"]) == 4 and len(registry["entities"]) == 3,
                      "Des écritures ont été perdues ou dupliquées pendant la reprise")
        smoke.require(registry["exchange_rates"] == [] and registry["participations"][0]["economic_share"] is None,
                      "L’assistant a ajouté un taux ou une part inconnue")
    passed("process_close_and_resume_draft_without_duplicates_all_steps_completed")

    with launch("all", "existing-clean-webview") as driver:
        smoke.require(not visible_dialog(driver), "Un coffre utilisé ouvre l’assistant sans demande")
        smoke.navigate(driver, "Famille")
        smoke.wait_text(driver, "Camille Exemple")
        assert_sequence(driver, 7)
    passed("existing_used_vault_has_no_forced_wizard_with_new_webview_profile")

    if previous_application:
        with launch("previous", executable=previous_application) as driver:
            driver.invoke("save_settings", {"settings": {"workspace_name": "Coffre 0.1.2 Exemple", "display_currency": "EUR"}})
            person = driver.invoke("save_entity", {"input": {
                "id": None, "expected_revision_id": None, "reason": "Données de qualification",
                "kind": "person", "display_name": "Famille 0.1.2 Exemple", "first_name": None,
                "last_name": None, "birth_date": None, "residence_country": None,
                "preferred_currency": "EUR", "activity": "", "notes": "", "archived": False,
            }})
            driver.invoke("save_membership", {"input": {
                "id": None, "expected_revision_id": None, "reason": "Données de qualification",
                "person_id": person["id"], "valid_from": "2025-01-01", "valid_to": None, "relationship": "",
            }})
            original_registry = driver.invoke("list_registry")
            original = driver.invoke("get_status")
        with launch("previous") as driver:
            current = driver.invoke("get_status")
            smoke.require(not visible_dialog(driver), "La mise à jour d’un coffre utilisé impose l’assistant")
            smoke.require(current["generation_id"] == original["generation_id"] and current["settings"] == original["settings"],
                          "La mise à jour a remplacé le coffre ou ses réglages")
            smoke.require(driver.invoke("list_registry") == original_registry, "La version mise à jour ne relit pas la famille sauvegardée")
            assert_sequence(driver, original["mutation_sequence"])
        passed("used_previous_application_vault_is_preserved_without_wizard")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("application", type=Path)
    parser.add_argument("--previous-application", type=Path)
    parser.add_argument("--artifacts", type=Path, required=True)
    args = parser.parse_args()
    application = args.application.resolve(strict=True)
    previous = args.previous_application.resolve(strict=True) if args.previous_application else None
    artifacts = args.artifacts.resolve()
    artifacts.mkdir(parents=True, exist_ok=True)
    report = {"status": "failed", "application": str(application), "steps": [],
              "started_at": datetime.now(timezone.utc).isoformat()}
    try:
        with tempfile.TemporaryDirectory(prefix="patrimoine-assistant-") as directory:
            scenarios(application, previous, artifacts, Path(directory), report["steps"])
        report["status"] = "passed"
    except Exception as error:
        report["error"] = f"{type(error).__name__}: {error}"
    report["completed_at"] = datetime.now(timezone.utc).isoformat()
    result = json.dumps(report, ensure_ascii=False, indent=2)
    (artifacts / "onboarding-result.json").write_text(result + "\n", encoding="utf-8")
    print(result)
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
