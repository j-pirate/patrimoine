#!/usr/bin/env python3
"""Teste le tour illustré en réseau coupé dans le vrai WebKit de Patrimoine.

  python3 desktop-discovery.py /app/Patrimoine.AppImage --artifacts /artifacts

Le répertoire des preuves ne contient que des captures de données fictives.
"""

import argparse
from contextlib import closing
from datetime import datetime, timezone
from importlib.machinery import SourceFileLoader
import json
from pathlib import Path
import sqlite3
import sys
import tempfile


onboarding = SourceFileLoader("desktop_onboarding", str(Path(__file__).with_name("desktop-onboarding.py"))).load_module()
smoke = onboarding.smoke
TITLES = (
    "Comprendre le tableau de bord",
    "Ajouter un membre de la famille",
    "Ajouter un bien immobilier",
    "Gérer les loyers",
    "Ajouter un compte bancaire",
    "Ajouter de l’or ou de l’argent",
    "Gérer une société",
    "Sauvegarder ses données",
)


def current_step(driver):
    return driver.execute("""
        const dialog = document.querySelector('.discovery-dialog[open]');
        if (!dialog) return null;
        const bar = dialog.querySelector('progress');
        return {
            title: dialog.querySelector('h3')?.textContent?.trim(),
            progress: bar && [bar.value, bar.max],
            images: [...dialog.querySelectorAll('img')].map(image => {
                const src = new URL(image.currentSrc || image.src, location.href);
                return { src: src.href, local: src.protocol === 'data:' || (src.protocol === location.protocol && src.host === location.host),
                    complete: image.complete, width: image.naturalWidth, height: image.naturalHeight,
                    alt: image.alt, caption: image.closest('figure')?.querySelector('figcaption')?.textContent?.trim() ?? '' };
            }),
        };
    """)


def step(driver, number):
    def loaded():
        state = current_step(driver)
        if state and state.get("title") == TITLES[number - 1] and state["images"] and all(
            image["complete"] and image["width"] > 0 for image in state["images"]
        ):
            return state
        return None

    state = smoke.wait_until(loaded, f"illustration décodée à l’étape {number} : {TITLES[number - 1]}")
    smoke.require(state["progress"] == [number, len(TITLES)], "La progression n’identifie pas l’étape affichée")
    for image in state["images"]:
        smoke.require(image["local"] and image["width"] >= 600 and image["height"] >= 120,
                      f"Illustration absente du paquet ou insuffisamment lisible à l’étape {number}")
        smoke.require(image["alt"] and "exemple" in image["caption"].lower(),
                      f"Texte alternatif ou légende de données fictives manquant à l’étape {number}")
    return state


def press(driver, key):
    smoke.xdotool("key", "--clearmodifiers", key)
    driver.command("POST", "/execute/async", {
        "script": "const done = arguments[arguments.length-1]; requestAnimationFrame(() => requestAnimationFrame(() => done(true)));",
        "args": [],
    })


def focused(driver):
    return driver.execute("""
        const element = document.activeElement;
        return element && { tag: element.tagName, name: element.textContent.trim(),
            inDialog: !!element.closest('.discovery-dialog[open]') };
    """)


def keyboard_button(driver, label):
    for _ in range(12):
        active = focused(driver)
        smoke.require(active and active["inDialog"], "Le clavier a quitté le tutoriel alors qu’il est ouvert")
        if active["tag"] == "BUTTON" and active["name"] == label:
            press(driver, "Return")
            return
        press(driver, "Tab")
    raise RuntimeError(f"Le bouton {label} est inaccessible au clavier dans le tutoriel")


def app_state(driver):
    status = driver.invoke("get_status")
    root = Path(status["data_dir"])
    database = root / "generations" / status["generation_id"] / "patrimoine.sqlite3"
    with closing(sqlite3.connect(database.as_uri() + "?mode=ro", uri=True)) as connection:
        backups = connection.execute("SELECT count(*) FROM backup_runs").fetchone()[0]
    return {"status": status, "registry": driver.invoke("list_registry"), "backup_runs": backups,
            "staging": sorted(child.name for child in (root / "staging").iterdir())}


def scenarios(application, artifacts, temporary, results):
    def launch(vault):
        return onboarding.session(application, artifacts, temporary, vault, proof_prefix="discovery")

    def passed(name, **details):
        results.append({"name": name, "status": "passed", **details})
        print(name, flush=True)

    with launch("discovery") as driver:
        onboarding.step(driver, 1)
        onboarding.dialog_button(driver, onboarding.SKIP_ALL)
        smoke.wait_until(lambda: not onboarding.visible_dialog(driver), "assistant fermé avant l’aide")
        baseline = app_state(driver)

        smoke.navigate(driver, "Aide")
        driver.click(smoke.button("Découvrir Patrimoine"))
        first = step(driver, 1)
        smoke.require(focused(driver) == {"tag": "H3", "name": TITLES[0], "inDialog": True},
                      "Le titre du tutoriel ne reçoit pas le focus")
        driver.screenshot(artifacts / "discovery-dashboard.png")
        keyboard_button(driver, "Suivant")
        step(driver, 2)
        press(driver, "Right")
        step(driver, 3)
        press(driver, "Left")
        step(driver, 2)
        keyboard_button(driver, "Précédent")
        step(driver, 1)
        press(driver, "Escape")
        smoke.wait_until(lambda: current_step(driver) is None, "fermeture du tutoriel au clavier")
        smoke.wait_until(lambda: focused(driver)["name"] == "Découvrir Patrimoine", "retour du focus au lien d’aide")
        passed("help_launch_keyboard_next_previous_escape_and_focus")

        driver.click(smoke.button("Découvrir Patrimoine"))
        illustration_urls = {image["src"] for image in first["images"]}
        for number in range(1, len(TITLES)):
            step(driver, number)
            onboarding.dialog_button(driver, "Suivant")
            following = step(driver, number + 1)
            illustration_urls.update(image["src"] for image in following["images"])
        smoke.require(len(illustration_urls) >= len(TITLES), "Des étapes différentes partagent une illustration générique")
        driver.screenshot(artifacts / "discovery-backup.png")
        onboarding.dialog_button(driver, "Terminer")
        smoke.wait_until(lambda: current_step(driver) is None, "fin du tour de présentation")
        smoke.navigate(driver, "Sauvegarde et export")
        driver.click(smoke.button("Aide sur cette rubrique"))
        step(driver, 8)
        onboarding.dialog_button(driver, "Ouvrir la page Sauvegarde et export")
        smoke.wait_until(lambda: current_step(driver) is None, "rubrique de sauvegarde ouverte sans dialogue fichier")
        smoke.wait_text(driver, "Créer une sauvegarde complète")
        smoke.require(app_state(driver) == baseline, "Le tour de sauvegarde a modifié le coffre")
        passed("all_tutorial_images_decode_from_local_package_and_demo_has_no_writes",
               tutorials=len(TITLES), distinct_packaged_illustrations=len(illustration_urls))

        smoke.navigate(driver, "Famille")
        driver.click(smoke.button("Aide sur cette rubrique"))
        step(driver, 2)
        onboarding.dialog_button(driver, "Fermer le tour")
        smoke.wait_until(lambda: current_step(driver) is None, "aide Famille fermée")
        smoke.navigate(driver, "Immobilier et loyers")
        driver.click(smoke.button("Paiements et dépenses"))
        driver.click(smoke.button("Aide sur les baux et loyers"))
        step(driver, 4)
        onboarding.dialog_button(driver, "Ouvrir les baux et échéances")
        smoke.wait_until(lambda: current_step(driver) is None, "navigation du tutoriel vers l’immobilier")
        active_tab = driver.execute("return document.querySelector('[aria-label=\"Sections immobilières\"] button[aria-pressed=true]')?.textContent;")
        smoke.require(active_tab == "Baux et échéances", "Le lien du tutoriel n’accède pas à la section Baux")
        smoke.wait_text(driver, "Ajouter un bail")
        driver.screenshot(artifacts / "discovery-rental-target.png")
        smoke.require(app_state(driver) == baseline, "L’aide contextuelle a modifié le coffre")
        passed("contextual_family_and_rentals_help_reaches_the_actual_leases_tab")

    with launch("from-wizard") as driver:
        onboarding.start_wizard(driver)
        for number in range(2, 7):
            onboarding.step(driver, number)
            onboarding.skip_step(driver)
        onboarding.step(driver, 7)
        baseline = app_state(driver)
        onboarding.dialog_button(driver, "Découvrir Patrimoine")
        step(driver, 1)
        smoke.require(not onboarding.visible_dialog(driver), "Les deux fenêtres d’aide restent ouvertes ensemble")
        smoke.require(onboarding.durable_configuration(driver).get("disposition") == "completed",
                      "Le lancement du tour a laissé l’assistant actif")
        onboarding.dialog_button(driver, "Fermer le tour")
        smoke.wait_until(lambda: current_step(driver) is None, "tour lancé par l’assistant refermé")
        smoke.require(app_state(driver) == baseline, "La démonstration lancée depuis l’assistant a ajouté des données")
        passed("tour_is_optional_after_onboarding_and_preserves_empty_vault")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("application", type=Path)
    parser.add_argument("--artifacts", type=Path, required=True)
    args = parser.parse_args()
    application = args.application.resolve(strict=True)
    artifacts = args.artifacts.resolve()
    artifacts.mkdir(parents=True, exist_ok=True)
    report = {"status": "failed", "application": str(application), "steps": [],
              "started_at": datetime.now(timezone.utc).isoformat()}
    try:
        with tempfile.TemporaryDirectory(prefix="patrimoine-decouverte-") as directory:
            scenarios(application, artifacts, Path(directory), report["steps"])
        report["status"] = "passed"
    except Exception as error:
        report["error"] = f"{type(error).__name__}: {error}"
    report["completed_at"] = datetime.now(timezone.utc).isoformat()
    result = json.dumps(report, ensure_ascii=False, indent=2)
    (artifacts / "discovery-result.json").write_text(result + "\n", encoding="utf-8")
    print(result)
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
