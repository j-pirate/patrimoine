#!/usr/bin/env python3
"""Smoke test du binaire desktop réel, sans dépendance Python externe.

Exemple (image de test équipée de WebKitWebDriver, Xvfb et tauri-driver) :
  xvfb-run -a dbus-run-session python3 desktop-smoke.py /app/patrimoine \
    --artifacts /artifacts --native-dialogs

Sans --native-dialogs : IPC réel, rendu, paramètres, registre, taux et historique.
Avec --native-dialogs : import/export d'un original, sauvegarde et restauration
par les dialogues GTK, avec sauvegarde préalable. Toujours sur un coffre neuf.
"""

import argparse
import base64
import hashlib
import http.client
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
from datetime import datetime, timezone
from fractions import Fraction
import zipfile


ELEMENT_KEY = "element-6066-11e4-a52e-4f735466cecf"


class WebDriver:
    def __init__(self, port=4444):
        self.port = port
        self.session_id = None

    def request(self, method, path, body=None, timeout=35):
        connection = http.client.HTTPConnection("127.0.0.1", self.port, timeout=timeout)
        try:
            payload = None if body is None else json.dumps(body).encode("utf-8")
            connection.request(method, path, payload, {"Content-Type": "application/json"})
            response = connection.getresponse()
            raw = response.read()
            parsed = json.loads(raw) if raw else {"value": None}
            value = parsed.get("value")
            if response.status >= 400 or isinstance(value, dict) and value.get("error"):
                raise RuntimeError(f"WebDriver {method} {path}: HTTP {response.status}: {value}")
            return value
        finally:
            connection.close()

    def command(self, method, suffix, body=None):
        if not self.session_id:
            raise RuntimeError("Session WebDriver absente")
        return self.request(method, f"/session/{self.session_id}{suffix}", body)

    def start(self, application):
        value = self.request("POST", "/session", {
            "capabilities": {"alwaysMatch": {"tauri:options": {"application": str(application)}}}
        })
        self.session_id = value["sessionId"]
        self.command("POST", "/timeouts", {"implicit": 0, "pageLoad": 30000, "script": 30000})

    def execute(self, script, *args):
        return self.command("POST", "/execute/sync", {"script": script, "args": list(args)})

    def invoke(self, command, args=None):
        result = self.command("POST", "/execute/async", {
            "script": """
                const done = arguments[arguments.length - 1];
                const internal = window.__TAURI_INTERNALS__;
                if (!internal || typeof internal.invoke !== 'function') {
                  done({ok: false, error: 'IPC Tauri indisponible'}); return;
                }
                internal.invoke(arguments[0], arguments[1]).then(
                  value => done({ok: true, value}),
                  error => done({ok: false, error})
                );
            """,
            "args": [command, args or {}],
        })
        if not result or not result.get("ok"):
            raise RuntimeError(f"IPC {command}: {result}")
        return result.get("value")

    def find(self, value, using="xpath"):
        return self.command("POST", "/element", {"using": using, "value": value})[ELEMENT_KEY]

    def click(self, xpath):
        self.command("POST", "/execute/async", {
            "script": "const done=arguments[arguments.length-1]; requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));", "args": [],
        })
        def enabled_element():
            element = self.find(xpath)
            return element if self.command("GET", f"/element/{element}/enabled") else None
        element = wait_until(enabled_element, f"élément cliquable : {xpath}")
        # WebKitWebDriver may scroll a button flush with the viewport bottom.
        # Keep the target away from the scrollbar before the real pointer click.
        self.command("POST", "/execute/async", {
            "script": """
                const done = arguments[arguments.length - 1];
                const target = document.evaluate(arguments[0], document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue;
                const visibleTarget = target?.closest('select') ?? target;
                const rect = visibleTarget?.getBoundingClientRect();
                if (rect && (rect.top < 24 || rect.bottom > innerHeight - 24 ||
                             rect.left < 0 || rect.right > innerWidth - 24)) {
                  visibleTarget.scrollIntoView({block: 'center', inline: 'nearest', behavior: 'instant'});
                }
                requestAnimationFrame(() => requestAnimationFrame(() => done(true)));
            """,
            "args": [xpath],
        })
        self.command("POST", f"/element/{element}/click", {})

    def text(self):
        return self.execute("return document.body.innerText;")

    def fill(self, element_id, text):
        element = self.find(element_id, using="css selector")
        self.command("POST", f"/element/{element}/clear", {})
        wait_until(lambda: self.execute("return document.querySelector(arguments[0]).value;", element_id) == "",
                   f"champ effacé : {element_id}")
        # WebKit peut acquitter une rafale sendKeys avant tous les événements input.
        # Des commandes séparées et la vérification DOM gardent une saisie utilisateur réelle.
        for index, character in enumerate(text, start=1):
            self.command("POST", f"/element/{element}/value", {"text": character, "value": [character]})
            expected = text[:index]
            wait_until(lambda: self.execute("return document.querySelector(arguments[0]).value;", element_id) == expected,
                       f"saisie du champ {element_id}, caractère {index}")
            # Le DOM peut refléter la touche avant le rendu contrôlé React.
            # Attendre deux images évite d'envoyer la suivante sur un état qui
            # sera encore remplacé ; aucune valeur n'est injectée dans React.
            settled = self.command("POST", "/execute/async", {
                "script": "const selector=arguments[0], done=arguments[arguments.length-1]; requestAnimationFrame(() => requestAnimationFrame(() => done(document.querySelector(selector).value)));",
                "args": [element_id],
            })
            require(settled == expected, f"Saisie non stabilisée dans {element_id} : attendu {expected!r}, obtenu {settled!r}")
        wait_until(lambda: self.execute("return document.querySelector(arguments[0]).value;", element_id) == text,
                   f"valeur complète du champ {element_id}")

    def fill_date(self, selector, value):
        # Native date inputs have platform-dependent keyboard formats. Emit the
        # real DOM input/change events; application handlers and IPC stay real.
        actual = self.command("POST", "/execute/async", {
            "script": """
                const node=document.querySelector(arguments[0]), value=arguments[1],
                      done=arguments[arguments.length-1];
                Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(node,value);
                node.dispatchEvent(new Event('input',{bubbles:true}));
                node.dispatchEvent(new Event('change',{bubbles:true}));
                requestAnimationFrame(()=>requestAnimationFrame(()=>done(node.value)));
            """, "args": [selector, value],
        })
        require(actual == value, f"Date non stabilisée dans {selector}: {actual!r}")

    def screenshot(self, destination):
        content = self.command("GET", "/screenshot")
        destination.write_bytes(base64.b64decode(content, validate=True))

    def capture_section(self, selector, destination):
        self.command("POST", "/execute/async", {
            "script": "const node=document.querySelector(arguments[0]), done=arguments[arguments.length-1]; for(let p=node;p;p=p.parentElement)p.style.scrollBehavior='auto'; node.scrollIntoView({block:'start',behavior:'auto'}); requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));",
            "args": [selector],
        })
        self.screenshot(destination)

    def close(self):
        if self.session_id:
            try:
                self.request("DELETE", f"/session/{self.session_id}", timeout=5)
            finally:
                self.session_id = None


def wait_until(callback, description, timeout=15):
    deadline = time.monotonic() + timeout
    last_error = None
    while time.monotonic() < deadline:
        try:
            value = callback()
            if value:
                return value
        except (OSError, RuntimeError, ValueError, KeyError) as error:
            last_error = str(error)
        time.sleep(0.15)
    raise RuntimeError(f"Délai dépassé : {description}. Dernier diagnostic : {last_error}")


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def button(label):
    # Les libellés utilisés ici sont des constantes internes, jamais des données.
    return f"//button[normalize-space(.)={json.dumps(label, ensure_ascii=False)}]"


def navigate(driver, label):
    driver.click(f"//nav//button[contains(., {json.dumps(label, ensure_ascii=False)})]")


def wait_text(driver, text, timeout=15):
    return wait_until(lambda: text in driver.text(), f"texte affiché : {text}", timeout)


def save_settings_ui(driver, workspace_name, currency="CHF"):
    navigate(driver, "Paramètres")
    wait_until(lambda: driver.find("#workspace-name", "css selector"), "formulaire des paramètres")
    driver.fill("#workspace-name", workspace_name)
    driver.click(f"//select[@id='display-currency']/option[.='{currency}']")
    wait_until(lambda: driver.execute("return document.querySelector('#display-currency').value;") == currency,
               "sélection de la devise")
    require(driver.execute("return document.querySelector('#workspace-name').value;") == workspace_name,
            "Le changement de devise a modifié le nom saisi")
    driver.click(button("Enregistrer les paramètres"))
    wait_text(driver, "Les paramètres ont été enregistrés dans le coffre local.")
    actual = driver.invoke("get_settings")
    require(actual == {"workspace_name": workspace_name, "display_currency": currency},
            f"Paramètres non persistés par IPC : {actual}")
    wait_until(lambda: not driver.execute("return document.querySelector('nav button').disabled;"),
               "fin de l'enregistrement des paramètres")
    return actual


def xdotool(*args, timeout=5, check=True):
    return subprocess.run(["xdotool", *args], text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, timeout=timeout, check=check)


def dialog_window(title):
    # Avec Xvfb sans gestionnaire de fenêtres, windowfocus fonctionne sans _NET_ACTIVE_WINDOW.
    import re
    result = xdotool("search", "--onlyvisible", "--name", f"^{re.escape(title)}$", check=False)
    return result.stdout.splitlines()[0] if result.returncode == 0 and result.stdout.strip() else None


def choose_native_file(driver, button_label, dialog_title, path):
    """Le thread X11 débloque aussi un click WebDriver attendant la fermeture native."""
    failures = []

    def select_file():
        try:
            window = wait_until(lambda: dialog_window(dialog_title), f"dialogue natif {dialog_title}")
            xdotool("windowfocus", "--sync", window)
            xdotool("key", "--clearmodifiers", "ctrl+l")
            # GTK peut ne sélectionner que le nom sans extension dans un dialogue
            # d'enregistrement : remplacer tout le champ, extension comprise.
            xdotool("key", "--clearmodifiers", "ctrl+a")
            xdotool("type", "--clearmodifiers", "--delay", "1", "--", str(path))
            xdotool("key", "--clearmodifiers", "Return")
            time.sleep(0.3)
            if dialog_window(dialog_title):
                xdotool("key", "--clearmodifiers", "Return")
            wait_until(lambda: not dialog_window(dialog_title), f"fermeture du dialogue {dialog_title}")
        except Exception as error:  # rapport conservé puis remonté sur le thread principal
            failures.append(error)

    worker = threading.Thread(target=select_file, daemon=True)
    worker.start()
    driver.click(button(button_label))
    worker.join(timeout=32)
    require(not worker.is_alive(), f"Automatisation du dialogue bloquée : {dialog_title}")
    if failures:
        raise RuntimeError(f"Dialogue {dialog_title} : {failures[0]}")


def canonical_registry(snapshot):
    # Plusieurs révisions historiques d'un profil partagent l'identité de la
    # personne. Ne jamais les écraser en indexant seulement sur l'identifiant.
    return {key: sorted(value, key=lambda row: (row['id'], row.get('revision_id', '')))
            if isinstance(value, list) else value
            for key, value in snapshot.items()}


def wait_operation_complete(driver):
    wait_until(lambda: not driver.execute("return document.querySelector('nav button').disabled;"),
               "fin de l’écriture du registre")


def consolidate(driver, person_id, knowledge_sequence=None, date=None):
    date = date or driver.execute("return document.querySelector('#global-date').value;")
    return driver.invoke("get_consolidation", {"request": {
        "context": "person" if person_id else "family", "person_id": person_id,
        "date": date, "knowledge_sequence": knowledge_sequence,
    }})


def registry_scenarios(driver, artifacts, steps):
    """Jeu synthétique créé par le véritable IPC, avec premiers formulaires UI."""
    date = driver.execute("return document.querySelector('#global-date').value;")
    initial = driver.invoke("list_registry")
    require(all(not value for value in initial.values() if isinstance(value, list)),
            "Le coffre neuf contient déjà des données patrimoniales")
    navigate(driver, "Famille")
    driver.click(button("Créer une personne"))
    driver.fill("#registry-display_name", "Alice du test desktop")
    driver.fill("#registry-first_name", "Alice")
    driver.fill("#registry-reason", "Jeu synthétique de validation desktop")
    driver.click(button("Créer l’enregistrement"))
    wait_text(driver, "L’enregistrement a été créé dans le registre local.")
    wait_operation_complete(driver)
    alice = next(entity for entity in driver.invoke("list_registry")["entities"]
                 if entity["display_name"] == "Alice du test desktop")
    steps.append({"name": "person_created_through_registry_ui", "status": "passed", "id": alice["id"]})

    def save(record_kind, **fields):
        return driver.invoke("save_" + record_kind, {"input": {
            "id": None, "expected_revision_id": None,
            "reason": "Jeu synthétique de validation desktop", **fields,
        }})

    def entity(name, kind="person"):
        return save("entity", kind=kind, display_name=name, first_name=None, last_name=None,
                    birth_date=None, residence_country="FR", preferred_currency="EUR",
                    activity="", notes="Jeu de test sans donnée réelle", archived=False)

    bob = entity("Benoît du test desktop")
    outside = entity("Tiers du test desktop")
    for person in (alice, bob):
        save("membership", person_id=person["id"], valid_from="2020-01-01", valid_to=None,
             relationship="Membre synthétique")

    # Recharger l’application : les insertions IPC externes aux formulaires doivent
    # être relues, sans injecter de faux état React pour rendre les tests verts.
    driver.command("POST", "/refresh", {})
    wait_until(lambda: driver.execute("return !!document.querySelector('#global-context') && !document.querySelector('#global-context').disabled;"),
               "relecture du registre à l’ouverture")
    navigate(driver, "Patrimoine")
    driver.click(button("Créer un actif"))
    driver.fill("#registry-name", "Bien partagé du test")
    driver.click("//select[@id='registry-country']/option[@value='FR']")
    driver.fill("#registry-category", "immobilier")
    driver.fill("#registry-reason", "Actif synthétique de copropriété")
    driver.click(button("Créer l’enregistrement"))
    wait_text(driver, "L’enregistrement a été créé dans le registre local.")
    wait_operation_complete(driver)
    asset = next(item for item in driver.invoke("list_registry")["assets"] if item["name"] == "Bien partagé du test")
    driver.click(button("Ajouter une estimation"))
    driver.click(f"//select[@id='registry-target_id']/option[@value='{asset['id']}']")
    driver.fill("#registry-amount", "400000")
    driver.fill("#registry-source", "Estimation synthétique du test desktop")
    driver.fill("#registry-reason", "Estimation synthétique datée")
    driver.click(button("Créer l’enregistrement"))
    wait_text(driver, "L’enregistrement a été créé dans le registre local.")
    wait_operation_complete(driver)

    def ratio(num, den):
        return {"num": str(num), "den": str(den)}

    for owner, share in ((alice, ratio(1, 2)), (bob, ratio(3, 10)), (outside, ratio(1, 5))):
        save("ownership", asset_id=asset["id"], owner_entity_id=owner["id"],
             legal_share=share, economic_share=share, valid_from="2020-01-01", valid_to=None)
    expected = ((None, "320000.00"), (alice["id"], "200000.00"), (bob["id"], "120000.00"))
    for person_id, amount in expected:
        result = consolidate(driver, person_id)
        require(result["gross_eur"] == amount, f"Copropriété mal attribuée : {result}")
    steps.append({"name": "joint_property_50_30_20_exact_attributions", "status": "passed"})

    # Chaîne sur deux sociétés, chacune entièrement attribuée entre un membre et
    # le tiers. Une estimation indicative des parts en mode détaillé reste exclue.
    company_x = entity("Société X du test", "company")
    company_y = entity("Société Y du test", "company")
    for company in (company_x, company_y):
        save("company_policy", company_id=company["id"], mode="detailed",
             coverage_complete=True, liability_qualified=False, valid_from="2020-01-01", valid_to=None)
    for owner, company, share in ((alice, company_x, ratio(3, 5)), (outside, company_x, ratio(2, 5)),
                                  (company_x, company_y, ratio(1, 2)), (outside, company_y, ratio(1, 2))):
        save("participation", owner_entity_id=owner["id"], company_id=company["id"],
             legal_share=share, economic_share=share, voting_share=share,
             valid_from="2020-01-01", valid_to=None)
    indirect = save("asset", name="Actif indirect du test", country="AE", category="autre",
                    notes="Jeu synthétique", archived=False)
    save("ownership", asset_id=indirect["id"], owner_entity_id=company_y["id"],
         legal_share=ratio(1, 1), economic_share=ratio(1, 1), valid_from="2020-01-01", valid_to=None)
    save("valuation", target_kind="asset", target_id=indirect["id"], amount="100000", currency="EUR",
         effective_on="2020-01-01", source="Jeu synthétique")
    save("valuation", target_kind="company", target_id=company_x["id"], amount="999999", currency="EUR",
         effective_on="2020-01-01", source="Estimation indicative exclue du mode détaillé")
    family = consolidate(driver, None)
    require(family["gross_eur"] == "350000.00", f"Chaîne 60 % × 50 % ou double compte incorrect : {family}")
    require(consolidate(driver, alice["id"])["gross_eur"] == "230000.00", "Attribution indirecte individuelle incorrecte")
    steps.append({"name": "indirect_company_chain_without_double_counting", "status": "passed"})

    driver.command("POST", "/refresh", {})
    wait_until(lambda: driver.execute("return !!document.querySelector('#global-context') && !document.querySelector('#global-context').disabled;"),
               "rafraîchissement du registre avant contrôle UI")
    navigate(driver, "Patrimoine")
    driver.click(button("Contrôler les attributions"))
    wait_text(driver, "350000.00 EUR")
    require("ne présente pas un patrimoine net exhaustif" in driver.text(),
            "Le contrôle n’indique pas les limites du périmètre de phase 3")
    driver.execute("document.querySelector('.consolidation-check').scrollIntoView({block: 'start'});")
    driver.screenshot(artifacts / "registry.png")
    steps.append({"name": "real_consolidation_control_ui", "status": "passed", "gross_eur": family["gross_eur"]})
    require(date == driver.execute("return document.querySelector('#global-date').value;"), "Date globale modifiée implicitement")
    return driver.invoke("list_registry")


def fx_scenarios(driver, artifacts, steps):
    """Cours manuels réels, exactitude, correction et connaissance historique."""
    date = driver.execute("return document.querySelector('#global-date').value;")
    snapshot = driver.invoke("list_registry")
    alice = next(entity for entity in snapshot["entities"]
                 if entity["display_name"] == "Alice du test desktop")
    navigate(driver, "Espèces et devises")
    driver.click(button("Ajouter un taux"))
    driver.click("//select[@id='rate-base']/option[@value='TND']")
    driver.click("//select[@id='rate-quote']/option[@value='EUR']")
    driver.fill("#rate-value", "0,3")
    driver.fill("#rate-source", "Cours manuel synthétique")
    driver.fill("#rate-reason", "Cours de validation desktop sans fournisseur externe")
    driver.click(button("Enregistrer le taux"))
    wait_operation_complete(driver)
    rate = wait_until(lambda: next((row for row in driver.invoke("list_registry")["exchange_rates"]
                                   if row["base_currency"] == "TND" and row["quote_currency"] == "EUR"), None),
                      "taux manuel TND vers EUR enregistré")
    require(rate["rate"] == "0.3" and rate["effective_on"] == date,
            f"Le taux saisi dans le formulaire n'a pas été conservé exactement : {rate}")
    steps.append({"name": "manual_exchange_rate_created_through_ui", "status": "passed",
                  "rate_id": rate["id"], "rate": rate["rate"]})

    def save(record_kind, **fields):
        return driver.invoke("save_" + record_kind, {"input": {
            "id": None, "expected_revision_id": None,
            "reason": "Jeu synthétique de validation multidevise", **fields,
        }})

    foreign = save("asset", name="Actif TND du test", country="TN", category="autre",
                   notes="Montant d'origine conservé en TND", archived=False)
    save("ownership", asset_id=foreign["id"], owner_entity_id=alice["id"],
         legal_share={"num": "1", "den": "1"}, economic_share={"num": "1", "den": "1"},
         valid_from="2020-01-01", valid_to=None)
    save("valuation", target_kind="asset", target_id=foreign["id"], amount="1000", currency="TND",
         effective_on="2020-01-01", source="Estimation synthétique en monnaie d'origine")
    first = consolidate(driver, None)
    require(first["gross_eur"] == "350300.00", f"Conversion TND familiale incorrecte : {first}")

    driver.fill("#fx-amount", "1000")
    driver.click(button("Vérifier la conversion"))
    wait_text(driver, "300.00 EUR")
    wait_text(driver, "Cours direct")
    driver.execute("document.querySelector('.fx-result').scrollIntoView({block: 'start'});")
    driver.screenshot(artifacts / "exchange-preview.png")
    steps.append({"name": "conversion_preview_uses_real_rust_result", "status": "passed"})

    def convert(amount, base, quote, as_of=date, sequence=None):
        return driver.invoke("convert_currency", {"query": {
            "amount": amount, "from_currency": base, "to_currency": quote,
            "date": as_of, "knowledge_sequence": sequence,
        }})

    direct = convert("1000", "TND", "EUR")
    require(direct["status"] == "complete" and direct["method"] == "direct"
            and direct["amount_converted"] == "300.00"
            and direct["exact_rate"] == {"num": "3", "den": "10"},
            f"Conversion directe non exacte : {direct}")
    inverse = convert("300", "EUR", "TND")
    require(inverse["status"] == "complete" and inverse["method"] == "inverse"
            and inverse["amount_converted"] == "1000.000", f"Conversion inverse incorrecte : {inverse}")
    missing = convert("1000", "TND", "EUR", "2019-12-31")
    require(missing["status"] == "non_calculable" and missing["amount_converted"] is None,
            f"Une date sans taux a reçu une conversion inventée : {missing}")
    steps.append({"name": "exact_direct_inverse_and_missing_historical_rate", "status": "passed"})

    old_sequence = driver.invoke("list_registry")["knowledge_sequence"]
    corrected = driver.invoke("save_exchange_rate", {"input": {
        "id": rate["id"], "expected_revision_id": rate["revision_id"],
        "reason": "Correction synthétique du taux, ancienne connaissance conservée",
        "base_currency": "TND", "quote_currency": "EUR", "rate": "0,31",
        "effective_on": rate["effective_on"], "source": "Cours manuel corrigé", "notes": "Test de révision",
    }})
    require(corrected["id"] == rate["id"] and corrected["revision_id"] != rate["revision_id"],
            "La correction n'a pas conservé l'identité du cours et créé une révision")
    require(consolidate(driver, None)["gross_eur"] == "350310.00", "Le taux corrigé ne valorise pas le stock courant")
    require(consolidate(driver, None, old_sequence)["gross_eur"] == "350300.00",
            "La correction du taux a écrasé la connaissance historique")
    previous = driver.invoke("list_registry_at", {"knowledgeSequence": old_sequence})
    prior_rate = next(row for row in previous["exchange_rates"] if row["id"] == rate["id"])
    require(prior_rate["rate"] == "0.3" and prior_rate["revision_id"] == rate["revision_id"],
            "Le cours original n'est plus disponible à son ancienne borne de connaissance")
    steps.append({"name": "rate_correction_preserves_prior_knowledge_and_amounts", "status": "passed",
                  "knowledge_sequence": old_sequence, "before_eur": "350300.00", "after_eur": "350310.00"})

    save("exchange_rate", base_currency="CHF", quote_currency="EUR", rate="1.24",
         effective_on="2020-01-01", source="Cours ancien synthétique", notes="Test du pivot EUR")
    cross = convert("1000", "TND", "CHF")
    require(cross["status"] == "complete" and cross["method"] == "cross"
            and cross["amount_converted"] == "250.00" and len(cross["legs"]) == 2,
            f"Conversion croisée non exacte : {cross}")
    require(cross["oldest_rate_date"] == "2020-01-01" and cross["age_days"] > 30 and cross["warnings"],
            "L'ancienneté du plus ancien cours croisé n'est pas exposée")
    steps.append({"name": "cross_rate_retains_each_source_and_oldest_date", "status": "passed"})

    # La relecture est réalisée par le frontend et ses commandes réelles, sans
    # remplacer le state React par un tableau synthétique.
    driver.command("POST", "/refresh", {})
    wait_until(lambda: driver.execute("return !!document.querySelector('#global-knowledge') && !document.querySelector('#global-knowledge').disabled;"),
               "sélecteur de connaissance disponible")
    navigate(driver, "Espèces et devises")
    require(driver.command("GET", f"/element/{driver.find(button('Ajouter un taux'))}/enabled"),
            "Le registre actuel ne propose pas l'ajout de cours")
    driver.click(f"//select[@id='global-knowledge']/option[@value='{old_sequence}']")
    wait_until(lambda: "lecture seule" in driver.text().lower(), "mode de connaissance historique en lecture seule")
    require(driver.execute("return Array.from(document.querySelectorAll('button')).filter(b => b.textContent.trim() === 'Ajouter un taux').every(b => b.disabled);"),
            "Un cours historique pourrait être modifié depuis une vue figée")
    navigate(driver, "Patrimoine")
    driver.click(button("Contrôler les attributions"))
    wait_text(driver, "350300.00 EUR")
    driver.execute("document.querySelector('.consolidation-check').scrollIntoView({block: 'start'});")
    driver.screenshot(artifacts / "historical-fx.png")
    navigate(driver, "Historique")
    wait_until(lambda: str(old_sequence) in driver.text(), "journal de connaissance visible")
    steps.append({"name": "historical_ui_is_read_only_and_reconstructs_prior_exchange_rate", "status": "passed"})
    driver.click("//select[@id='global-knowledge']/option[@value='current']")
    wait_until(lambda: driver.execute("return document.querySelector('#global-knowledge').value;") == "current",
               "retour aux connaissances actuelles")
    navigate(driver, "Espèces et devises")
    wait_until(lambda: driver.command("GET", f"/element/{driver.find(button('Ajouter un taux'))}/enabled"),
               "réactivation de la saisie des cours actuels")
    wait_text(driver, "0.31 EUR")
    driver.screenshot(artifacts / "exchange-rates.png")
    return driver.invoke("list_registry"), {
        "rate_id": rate["id"], "current_revision_id": corrected["revision_id"],
        "old_revision_id": rate["revision_id"], "old_sequence": old_sequence,
        "current_total": "350310.00", "old_total": "350300.00", "date": date,
    }


def profile_scenarios(driver, artifacts, steps):
    """Des périodes pointent des révisions exactes, même après une correction."""
    snapshot = driver.invoke("list_registry")
    person = next(row for row in snapshot["entities"]
                  if row["display_name"] == "Alice du test desktop")
    asset = next(row for row in snapshot["assets"] if row["name"] == "Actif TND du test")
    profiles = []
    for kind, original, changes, collection in (
        ("entity", person, {"display_name": "Alice du test desktop — profil corrigé", "residence_country": "TN"},
         "profile_revisions"),
        ("asset", asset, {"name": "Actif TND du test — profil corrigé", "country": "FR"},
         "asset_profile_revisions"),
    ):
        fields = {key: value for key, value in original.items() if key not in ("id", "revision_id")}
        corrected = driver.invoke("save_" + kind, {"input": {
            **fields, **changes, "id": original["id"],
            "expected_revision_id": original["revision_id"],
            "reason": "Correction synthétique du profil " + kind,
        }})
        require(corrected["id"] == original["id"]
                and corrected["revision_id"] != original["revision_id"],
                f"La correction du profil {kind} n'a pas créé une révision stable")
        for revision_id, valid_from, valid_to in (
            (original["revision_id"], "2020-01-01", "2024-01-01"),
            (corrected["revision_id"], "2024-01-01", None),
        ):
            driver.invoke("save_profile_period", {"input": {
                "id": None, "expected_revision_id": None,
                "reason": "Période synthétique du profil " + kind,
                "target_kind": kind, "target_id": original["id"],
                "profile_revision_id": revision_id, "valid_from": valid_from, "valid_to": valid_to,
            }})
        profiles.append({"target_id": original["id"], "collection": collection,
                         "old_revision_id": original["revision_id"],
                         "current_revision_id": corrected["revision_id"]})

    snapshot = driver.invoke("list_registry")
    for profile in profiles:
        revisions = {row["revision_id"] for row in snapshot[profile["collection"]]
                     if row["id"] == profile["target_id"]}
        require({profile["old_revision_id"], profile["current_revision_id"]} <= revisions,
                "Une correction a effacé une ancienne révision de profil du snapshot")
        references = {row["profile_revision_id"] for row in snapshot["profile_periods"]
                      if row["target_id"] == profile["target_id"]}
        require(references == {profile["old_revision_id"], profile["current_revision_id"]},
                "Les périodes n'ont pas conservé leurs deux révisions exactes")
    history = driver.invoke("list_registry_history")
    require(history["latest_sequence"] == snapshot["knowledge_sequence"],
            "Le journal n'expose pas la dernière séquence de connaissance")
    require(all(any(event["revision_id"] == profile["current_revision_id"]
                    for event in history["events"]) for profile in profiles),
            "Le journal ne contient pas les corrections des deux types de profil")
    driver.command("POST", "/refresh", {})
    wait_until(lambda: driver.execute("return !!document.querySelector('#global-knowledge') && !document.querySelector('#global-knowledge').disabled;"),
               "relecture des profils et du journal")
    navigate(driver, "Historique")
    wait_text(driver, "Période synthétique du profil entity")
    wait_text(driver, "Période synthétique du profil asset")
    driver.screenshot(artifacts / "profile-history.png")
    steps.append({"name": "exact_entity_and_asset_profile_revisions_and_history", "status": "passed"})
    # Comparer les valeurs réellement disponibles aux dates par défaut du
    # formulaire, puis vérifier le rendu du résultat Rust par le bouton UI.
    start = driver.execute("return document.querySelector('#variation-from').value;")
    end = driver.execute("return document.querySelector('#variation-to').value;")
    variation = driver.invoke("explain_variation", {"request": {
        "context": "family", "person_id": None, "from_date": start,
        "to_date": end, "knowledge_sequence": None,
    }})
    # Le bien partagé n'a de valeur qu'aujourd'hui ; la chaîne indirecte vaut
    # déjà 30 000 EUR depuis 2020. Le cours TND n'existe qu'aujourd'hui.
    expected_delta = "0.00" if start == end else "320310.00"
    require(variation["delta_eur"] == expected_delta,
            f"Variation du sous-total enregistré incorrecte : {variation}")
    require(variation["closing"]["gross_eur"] == "350310.00",
            "Le pont n'utilise pas le sous-total final courant")
    if start != end:
        require(variation["status"] == "partial"
                and any(row["amount_eur"] is None for row in variation["steps"]),
                "Les références initiales manquantes sont devenues des effets nuls")
    driver.click(button("Expliquer la variation"))
    wait_text(driver, f"{expected_delta} EUR")
    wait_text(driver, "Résidu non expliqué")
    if start != end:
        wait_text(driver, "Non attribuable")
    driver.execute("document.querySelector('.variation-result').scrollIntoView({block: 'start'});")
    driver.screenshot(artifacts / "variation.png")
    steps.append({"name": "variation_ui_uses_rust_subtotals_and_marks_missing_references", "status": "passed",
                  "delta_eur": expected_delta, "from_date": start, "to_date": end})
    return snapshot, profiles


def property_rental_scenarios(driver, artifacts, steps):
    """Immobilier et loyers réels, conservant les oracles de phases 3/4."""
    registry = driver.invoke("list_registry")
    asset = next(a for a in registry["assets"] if a["name"] == "Bien partagé du test")
    alice = next(e for e in registry["entities"] if e["first_name"] == "Alice")
    tenant = next(e for e in registry["entities"] if e["display_name"] == "Tiers du test desktop")

    def save(record_kind, **fields):
        return driver.invoke("save_" + record_kind, {"input": {
            "id": None, "expected_revision_id": None,
            "reason": "Validation immobilière synthétique", **fields,
        }})

    navigate(driver, "Immobilier et loyers")
    driver.click(button("Ajouter une fiche immobilière"))
    driver.click(f"//select[@id='property-asset_id']/option[@value='{asset['id']}']")
    driver.click("//select[@id='property-property_type']/option[@value='building']")
    driver.fill("#property-city", "Lyon")
    driver.fill("#property-address", "Adresse synthétique du test")
    driver.fill("#property-surface_m2", "125,50")
    driver.fill_date("#property-valid_from", "2020-01-01")
    driver.fill("#property-reason", "Fiche synthétique sans deuxième actif")
    driver.click(button("Enregistrer la fiche immobilière"))
    wait_operation_complete(driver)
    properties = wait_until(lambda: driver.invoke("list_properties_at")["properties"], "fiche immobilière persistée")
    require(properties[0]["asset_id"] == asset["id"] and properties[0]["surface_m2"] == "125.5",
            "La fiche n'est pas rattachée à l'actif original avec surface exacte")
    steps.append({"name": "property_details_created_through_ui_without_duplicate_asset", "status": "passed"})

    save("property_event", asset_id=asset["id"], kind="acquisition", effective_on="2020-01-01",
         price="300000", fees="10000", currency="EUR", notes="Coût distinct de l'estimation")
    units = [save("rental_unit", property_asset_id=asset["id"], reference=f"LOT-{index}",
                  name=f"Unité technique {index}", surface_m2="40", valid_from="2020-01-01",
                  valid_to=None, available_for_rent=True, notes="Aucune valeur patrimoniale propre")
             for index in range(1, 4)]
    save("valuation", target_kind="asset", target_id=asset["id"], amount="400000", currency="EUR",
         effective_on="2023-01-01", source="Estimation historique synthétique")
    require(consolidate(driver, None)["gross_eur"] == "350310.00", "Les unités ont doublé la valeur du bien")
    p_report = driver.invoke("get_property_report", {"request": {
        "asset_id": asset["id"], "date": "2023-12-31", "knowledge_sequence": None,
    }})
    require(p_report["acquisition_cost_eur"] == "310000.00"
            and p_report["estimated_value_eur"] == "400000.00"
            and p_report["unrealized_gain_eur"] == "90000.00"
            and p_report["debt_remaining_eur"] is None,
            f"Coût, estimation, gain ou dette supposée incorrects : {p_report}")
    steps.append({"name": "property_cost_estimate_and_units_are_distinct", "status": "passed"})

    lease = save("lease", unit_id=units[0]["id"], tenant_entity_id=tenant["id"],
                 valid_from="2023-01-01", valid_to="2024-01-01", notes="Bail synthétique annuel")
    save("lease_terms", lease_id=lease["id"], rent="1000", charges="0", currency="EUR",
         frequency="monthly", due_day=5, deposit_required="2000", valid_from="2023-01-01",
         valid_to="2024-01-01", notes="Dépôt contractuel séparé du revenu")
    generation = {"lease_id": lease["id"], "from_date": "2023-01-01", "to_date": "2024-01-01",
                  "expected_sequence": driver.invoke("list_rentals_at")["knowledge_sequence"],
                  "reason": "Génération synthétique annuelle"}
    first = driver.invoke("generate_rent_dues", {"input": generation})
    require(len(first["created"]) == 12, f"Nombre d'échéances incorrect : {first}")
    generation["expected_sequence"] = first["knowledge_sequence"]
    second = driver.invoke("generate_rent_dues", {"input": generation})
    require(not second["created"] and second["existing_count"] == 12
            and second["knowledge_sequence"] == first["knowledge_sequence"],
            "La génération répétée a dupliqué des échéances ou créé une mutation inutile")
    march = next(d for d in first["created"] if d["period_start"] == "2023-03-01")
    for amount, paid_on in [("400", "2023-03-05"), ("300", "2023-03-20"), ("300", "2023-04-20")]:
        payment = save("rent_payment", lease_id=lease["id"], kind="receipt", amount=amount,
                       currency="EUR", effective_on=paid_on, original_payment_id=None,
                       recipient_entity_id=alice["id"], reference=f"TEST-{paid_on}",
                       unlinked_reason="Compte bancaire non encore suivi ; validation synthétique")
        save("rent_allocation", payment_id=payment["id"], due_id=march["id"],
             component="rent", amount=amount, currency="EUR", effective_on=paid_on)
    save("rental_expense", asset_id=asset["id"], kind="operating", amount="2000", currency="EUR",
         effective_on="2023-06-01", paid_on="2023-06-01", recoverable=False,
         notes="Exploitation annuelle synthétique, hors capital et financement")

    def rental_report(start, end, observation):
        request = {"context": "property", "person_id": None, "asset_id": asset["id"], "country": None,
                   "from_date": start, "to_date": end, "date": observation, "knowledge_sequence": None}
        return request, driver.invoke("get_rental_report", {"request": request})

    _, march_report = rental_report("2023-03-01", "2023-04-01", "2023-03-31")
    march_status = next(row for row in march_report["dues"] if row["due_id"] == march["id"])
    require(march_report["rent_received_eur"] == "700.00"
            and march_status["outstanding_rent"] == "300" and march_status["historical_late"],
            f"Paiements partiels et retard au 31 mars incorrects : {march_report}")
    _, april_report = rental_report("2023-03-01", "2023-05-01", "2023-04-30")
    settled = next(row for row in april_report["dues"] if row["due_id"] == march["id"])
    require(settled["outstanding_rent"] == "0" and settled["settled_on"] == "2023-04-20"
            and settled["historical_late"], "Le règlement tardif a effacé l'ancien retard ou modifié la date de règlement")
    request, report = rental_report("2023-01-01", "2024-01-01", "2023-12-31")
    require(report["contractual_annual_rent_eur"] == "12000.00"
            and report["rent_due_eur"] == "12000.00" and report["rent_received_eur"] == "1000.00"
            and report["operating_expenses_eur"] == "2000.00" and report["noi_eur"] == "10000.00"
            and report["gross_yield_percent"] == "3.00" and report["net_yield_percent"] == "2.50"
            and report["cash_flow_after_financing_eur"] is None,
            f"Indicateurs locatifs, annualisation ou limite de financement incorrects : {report}")
    require(report["occupation"]["occupied_unit_days"] == 365
            and report["occupation"]["available_unit_days"] == 1095,
            f"L'occupation contractuelle ne distingue pas les unités vacantes : {report['occupation']}")
    steps.append({"name": "idempotent_dues_partial_payments_late_history_and_noi", "status": "passed",
                  "rent_due_eur": report["rent_due_eur"], "noi_eur": report["noi_eur"]})

    sold = save("asset", name="Bien cédé du test", country="FR", category="real_estate", notes="Cession synthétique", archived=False)
    save("property", asset_id=sold["id"], property_type="house", city="", address="", surface_m2=None,
         status="vacant", valid_from="2020-01-01", valid_to=None, notes="Historique conservé après cession")
    save("property_event", asset_id=sold["id"], kind="acquisition", effective_on="2020-01-01",
         price="100000", fees="10000", currency="EUR", notes="Source synthétique")
    save("ownership", asset_id=sold["id"], owner_entity_id=alice["id"],
         legal_share={"num": "1", "den": "1"}, economic_share={"num": "1", "den": "1"},
         valid_from="2020-01-01", valid_to=None)
    save("valuation", target_kind="asset", target_id=sold["id"], amount="150000", currency="EUR",
         effective_on="2020-01-01", source="Estimation synthétique avant cession")
    before_sale = driver.invoke("list_properties_at")["knowledge_sequence"]
    driver.invoke("dispose_property", {"input": {"asset_id": sold["id"], "effective_on": "2024-07-01",
                  "price": "160000", "fees": "1000", "currency": "EUR", "expected_sequence": before_sale,
                  "reason": "Cession complète synthétique avec clôture des droits"}})
    after_sale = driver.invoke("get_property_report", {"request": {
        "asset_id": sold["id"], "date": "2024-07-01", "knowledge_sequence": None}})
    prior_sale = driver.invoke("get_property_report", {"request": {
        "asset_id": sold["id"], "date": "2024-07-01", "knowledge_sequence": before_sale}})
    require(after_sale["status"] == "sold" and after_sale["estimated_value_eur"] is None
            and prior_sale["estimated_value_eur"] == "150000.00",
            "La cession a conservé un stock courant ou écrasé la connaissance précédente")
    require(consolidate(driver, None)["gross_eur"] == "350310.00", "Le bien cédé reste dans le patrimoine courant")
    steps.append({"name": "atomic_property_disposal_preserves_old_knowledge_without_sale_cash", "status": "passed"})

    driver.command("POST", "/refresh", {})
    wait_until(lambda: driver.execute("return !!document.querySelector('#global-knowledge') && !document.querySelector('#global-knowledge').disabled;"), "relecture après fixtures immobilières")
    navigate(driver, "Immobilier et loyers")
    driver.click(button("Corriger la cession"))
    before_correction = driver.invoke("list_properties_at")["knowledge_sequence"]
    driver.fill_date("#lifecycle-effective_on", "2024-08-01")
    driver.fill("#lifecycle-price", "161000")
    driver.fill("#lifecycle-reason", "Correction synthétique de la date et du prix de vente")
    driver.click("//input[@id='lifecycle-disposal-confirm']")
    driver.click(button("Confirmer la correction de cession"))
    wait_operation_complete(driver)
    current_sale = driver.invoke("get_property_report", {"request": {
        "asset_id": sold["id"], "date": "2024-07-15", "knowledge_sequence": None}})
    old_sale = driver.invoke("get_property_report", {"request": {
        "asset_id": sold["id"], "date": "2024-07-15", "knowledge_sequence": before_correction}})
    require(current_sale["estimated_value_eur"] == "150000.00" and old_sale["status"] == "sold",
            "La correction UI de cession n'a pas réparé les droits ou a effacé K antérieur")
    steps.append({"name": "disposal_date_correction_through_ui_updates_rights_atomically", "status": "passed"})
    driver.click(button("Indicateurs locatifs"))
    wait_text(driver, "Calculer les indicateurs locatifs")
    driver.click(f"//select[@id='rental-property-filter']/option[@value='{asset['id']}']")
    driver.click("//input[@id='rental-report-whole-property']")
    driver.fill_date("#rental-report-from", "2023-01-01")
    driver.fill_date("#rental-report-to", "2024-01-01")
    driver.click(button("Calculer les indicateurs locatifs"))
    wait_text(driver, "10000.00 EUR")
    driver.screenshot(artifacts / "rental-indicators.png")
    steps.append({"name": "rental_indicators_render_real_rust_report", "status": "passed"})
    return driver.invoke("list_registry"), {
        "properties": driver.invoke("list_properties_at"), "rentals": driver.invoke("list_rentals_at"),
        "report_request": request, "report": report,
    }


def financial_scenarios(driver, artifacts, steps, phase5):
    """Real IPC sources and UI writes; exact expectations calculated independently."""
    registry = driver.invoke("list_registry")
    alice = next(e for e in registry["entities"] if e["first_name"] == "Alice")
    bob = next(e for e in registry["entities"] if e["display_name"] == "Benoît du test desktop")
    outside = next(e for e in registry["entities"] if e["display_name"] == "Tiers du test desktop")
    date = driver.execute("return document.querySelector('#global-date').value;")
    one = {"num": "1", "den": "1"}

    def data(**fields):
        return {"id": None, "expected_revision_id": None, "reason": "Validation financière synthétique", **fields}

    def save(record_kind, **fields):
        return driver.invoke("save_" + record_kind, {"input": data(**fields)})

    def refresh():
        driver.command("POST", "/refresh", {})
        wait_until(lambda: driver.execute("return !!document.querySelector('#global-knowledge') && !document.querySelector('#global-knowledge').disabled;"), "relecture des sources financières")

    def choose(selector, value):
        driver.click(f"//select[@id='{selector}']/option[@value='{value}']")

    assets = []
    for name, country in [("Banque A du test", "FR"), ("Banque B du test", "FR"), ("Caisse TND du test", "TN")]:
        asset = save("asset", name=name, country=country, category="liquidités", notes="Solde issu du journal", archived=False)
        save("ownership", asset_id=asset["id"], owner_entity_id=alice["id"], legal_share=one,
             economic_share=one, valid_from="2020-01-01", valid_to=None)
        assets.append(asset)
    refresh()
    navigate(driver, "Comptes bancaires")
    driver.click(button("Ajouter un compte bancaire"))
    choose("treasury-asset_id", assets[0]["id"])
    choose("treasury-legal_owner_entity_id", alice["id"])
    driver.fill_date("#treasury-tracking_start", "2020-01-01")
    driver.fill("#treasury-bank_name", "Banque synthétique")
    driver.fill("#treasury-reference", "TEST-A")
    driver.fill("#treasury-reason", "Ouverture du suivi bancaire du test")
    driver.click(button("Enregistrer la trésorerie"))
    wait_operation_complete(driver)
    a = next(x for x in driver.invoke("list_treasury_at")["accounts"] if x["asset_id"] == assets[0]["id"])

    def report(account, knowledge=None):
        return driver.invoke("get_treasury_report", {"request": {"account_id": account["id"], "date": date, "knowledge_sequence": knowledge}})

    require(report(a)["balance_original"] is None, "Le compte sans ouverture a reçu un zéro implicite")
    driver.click(button("Enregistrer l’ouverture"))
    driver.fill("#treasury-label", "Ouverture confirmée")
    driver.fill("#treasury-line-amount-0", "100")
    driver.fill("#treasury-reason", "Solde initial confirmé du test")
    driver.click(button("Enregistrer la trésorerie"))
    wait_operation_complete(driver)
    require(report(a)["balance_original"] == "100.00", "L'ouverture UI n'a pas créé le solde exact")
    steps.append({"name": "bank_account_and_explicit_opening_through_ui", "status": "passed"})

    b, cash = [save("treasury_account", asset_id=asset["id"], kind=kind, currency=currency,
                    tracking_start="2020-01-01", legal_owner_entity_id=alice["id"],
                    bank_name="Banque synthétique" if kind == "bank" else "", reference="TEST-B" if kind == "bank" else "",
                    country=asset["country"], storage_location="Coffre synthétique" if kind == "cash" else "", notes="", archived=False)
               for asset, kind, currency in [(assets[1], "bank", "EUR"), (assets[2], "cash", "TND")]]

    def transaction(kind, effective_on, entries, label="Journal synthétique"):
        return data(kind=kind, effective_on=effective_on, label=label, reference="TEST", notes="",
                    entries=[{"line_id": f"line-{index}", "account_id": account["id"], "amount": amount, "role": role}
                             for index, (account, amount, role) in enumerate(entries)])

    def journal(kind, effective_on, entries):
        return driver.invoke("save_treasury_transaction", {"input": transaction(kind, effective_on, entries)})

    for account in [b, cash]:
        journal("opening", "2020-01-01", [(account, "0", "opening")])
    journal("receipt", "2020-01-02", [(a, "30", "principal")])
    journal("payment", "2020-01-03", [(a, "-20", "principal")])
    save("account_reconciliation", account_id=a["id"], effective_on="2020-01-03", statement_balance="100",
         statement_reference="RELEVE-TEST", document_id=None, notes="Écart conservé sans ajustement")
    reconciliation = report(a)
    require(reconciliation["balance_original"] == "110.00"
            and reconciliation["reconciliations"][0]["difference"] == "-10.00", "Le relevé a modifié le journal ou son écart")
    journal("transfer", "2020-01-04", [(a, "-50", "principal"), (b, "50", "principal")])
    journal("fx", "2020-01-05", [(a, "-100", "principal"), (cash, "330", "principal"), (a, "-2", "fee")])
    a_report, cash_report = report(a), report(cash)
    require(a_report["balance_original"] == "-42.00" and a_report["overdraft"]
            and report(b)["balance_original"] == "50.00" and cash_report["balance_original"] == "330.000",
            "Le transfert, la conversion ou les frais ont modifié les mauvais soldes")
    executed = cash_report["executed_fx"][0]
    require(executed["exact_rate"] == {"num": "33", "den": "10"}
            and executed["fees"] == [{"currency": "EUR", "amount": {"num": "2", "den": "1"}}],
            f"Taux exécuté ou frais mélangés : {executed}")
    steps.append({"name": "journal_transfer_fx_fees_overdraft_and_non_mutating_reconciliation", "status": "passed",
                  "bank_a_eur": "-42.00", "cash_tnd": "330.000", "executed_rate": executed["exact_rate"]})

    def loan(name, creditor, debtor, principal):
        contract = save("debt_contract", name=name, currency="EUR", tracking_start="2020-01-01", notes="", archived=False)
        parties = [{"line_id": side, "side": side, "entity_id": entity["id"], "legal_share": one,
                    "economic_share": one, "liability_kind": "single", "notes": ""}
                   for side, entity in [("creditor", creditor), ("debtor", debtor)]]
        save("debt_terms", debt_id=contract["id"], valid_from="2020-01-01", valid_to=None,
             principal_status="complete", interest_rate=None, interest_convention="Non calculé", parties=parties, notes="")
        save("debt_movement", debt_id=contract["id"], kind="opening", effective_on="2020-01-01",
             principal_change=principal, interest_paid="0", fees_paid="0", unlinked_reason="Ouverture historique, sans flux nouveau", notes="")
        return contract

    external = loan("Prêt bancaire du test", outside, alice, "100000")
    internal = loan("Prêt intrafamilial du test", alice, bob, "10000")
    schedule = save("debt_schedule", debt_id=external["id"], due_on="2020-01-06", expected_principal="1000",
                    expected_interest="200", expected_fees="10", status="planned", notes="Prévision distincte du flux")
    repayment = driver.invoke("post_debt_movement", {"input": {
        "expected_sequence": driver.invoke("list_debts_at")["knowledge_sequence"],
        "movement": data(debt_id=external["id"], kind="principal_repayment", effective_on="2020-01-06",
                         principal_change="-1000", interest_paid="200", fees_paid="10", unlinked_reason="Journal lié", notes=""),
        "transaction": transaction("loan", "2020-01-06", [(b, "-1210", "principal")]),
        "existing_transaction_revision_id": None,
        "links": [{"line_id": "line-0", "side": "debtor", "component": component, "amount": amount}
                  for component, amount in [("principal", "-1000"), ("interest", "-200"), ("fee", "-10")]],
    }})
    save("debt_payment_allocation", movement_id=repayment["id"], schedule_id=schedule["id"], effective_on="2020-01-06",
         principal="1000", interest="200", fees="10")
    save("debt_guarantee", debt_id=external["id"], guarantor_entity_id=bob["id"], limit_amount="50000",
         valid_from="2020-01-01", valid_to=None, notes="Garantie non appelée")
    debt_request = {"debt_id": external["id"], "date": date, "knowledge_sequence": None}
    debt_report = driver.invoke("get_debt_report", {"request": debt_request})
    require(debt_report["balance"]["principal_original"] == "99000.00"
            and debt_report["balance"]["interest_paid_original"] == "200.00"
            and debt_report["balance"]["fees_paid_original"] == "10.00"
            and debt_report["schedules"][0]["remaining_principal"] == "0.00"
            and len(debt_report["guarantees"]) == 1 and report(b)["balance_original"] == "-1160.00",
            f"Principal, flux total, échéancier ou garantie incorrects : {debt_report}")
    steps.append({"name": "loan_principal_interest_fees_atomic_cash_and_uncalled_guarantee", "status": "passed"})

    rentals = driver.invoke("list_rentals_at")
    lease = rentals["leases"][0]
    january = next(d for d in rentals["dues"] if d["lease_id"] == lease["id"] and d["period_start"] == "2023-01-01")
    save("rent_claim", due_id=january["id"], parties=[{"entity_id": alice["id"], "economic_share": one}],
         valid_from="2023-01-01", valid_to=None, represented_by_debt_id=None, notes="Créancier déclaré indépendamment du titre immobilier")
    old_k = driver.invoke("list_treasury_at")["knowledge_sequence"]
    refresh()
    navigate(driver, "Comptes bancaires")
    driver.click(button("Loyers et dépôts"))
    driver.click(button("Enregistrer un mouvement de dépôt"))
    choose("posting-lease_id", lease["id"])
    choose("posting-recipient_entity_id", alice["id"])
    driver.fill("#posting-amount", "2000")
    driver.fill_date("#posting-effective_on", "2023-06-01")
    choose("posting-cash-account", b["id"])
    driver.fill("#posting-notes", "Dépôt distinct du loyer")
    driver.fill("#posting-reason", "Dépôt reçu et banque liés atomiquement")
    driver.click(button("Enregistrer la source et la trésorerie ensemble"))
    wait_operation_complete(driver)
    require(report(b)["balance_original"] == "840.00", "Le dépôt UI n'a pas augmenté la banque du montant reçu")
    transactions_before = len(driver.invoke("list_treasury_at")["transactions"])
    driver.invoke("post_deposit", {"input": {
        "expected_sequence": driver.invoke("list_treasury_at")["knowledge_sequence"],
        "movement": data(lease_id=lease["id"], kind="applied_to_due", amount="-300", currency="EUR",
                         effective_on="2023-06-02", recipient_entity_id=alice["id"], due_id=january["id"], component="rent", notes="Compensation sans banque"),
        "cash": None,
    }})
    require(len(driver.invoke("list_treasury_at")["transactions"]) == transactions_before
            and report(b)["balance_original"] == "840.00", "La compensation a créé un faux flux bancaire")
    driver.invoke("post_deposit", {"input": {
        "expected_sequence": driver.invoke("list_treasury_at")["knowledge_sequence"],
        "movement": data(lease_id=lease["id"], kind="returned", amount="-500", currency="EUR",
                         effective_on="2023-06-03", recipient_entity_id=alice["id"], due_id=None, component="", notes="Restitution partielle"),
        "cash": {"transaction": transaction("payment", "2023-06-03", [(b, "-500", "principal")]),
                 "existing_transaction_revision_id": None, "line_id": "line-0"},
    }})
    require(report(b)["balance_original"] == "340.00" and report(b, old_k)["balance_original"] == "-1160.00",
            "Dépôt/restitution incorrects ou ancienne connaissance écrasée")
    phase5["report"] = driver.invoke("get_rental_report", {"request": phase5["report_request"]})
    january_status = next(d for d in phase5["report"]["dues"] if d["due_id"] == january["id"])
    require(january_status["outstanding_rent"] == "700" and phase5["report"]["rent_received_eur"] == "1000.00",
            "La compensation n'a pas réglé l'échéance ou a été transformée en loyer reçu")
    steps.append({"name": "deposit_ui_receipt_noncash_settlement_return_and_old_knowledge", "status": "passed",
                  "remaining_deposit_eur": "1200.00", "bank_b_eur": "340.00", "january_unpaid_eur": "700"})

    refresh()
    navigate(driver, "Patrimoine")
    driver.click("//summary[normalize-space(.)='Déclarations de couverture et droits sur les loyers']")
    driver.click(button("Déclarer la couverture du bilan"))
    for name in ["assets_complete", "debts_complete", "rentals_complete"]:
        driver.click(f"//input[@id='finance-{name}']")
    driver.fill_date("#finance-valid_from", "2020-01-01")
    driver.fill("#finance-reason", "Déclaration synthétique pour contrôler les sources manquantes")
    driver.click(button("Enregistrer la déclaration financière"))
    wait_operation_complete(driver)
    financial_request = {"context": "family", "person_id": None, "date": date, "knowledge_sequence": None}
    financial_report = driver.invoke("get_financial_report", {"request": financial_request})
    require(financial_report["coverage_confirmed"] and financial_report["net_eur"] is None
            and financial_report["status"] == "partial", "La déclaration a masqué les sources locatives manquantes")
    elimination = next(e for e in financial_report["eliminations"] if e["object_id"] == internal["id"])
    require(elimination["eliminated_eur"] == "10000.00", "Le prêt intrafamilial n'est pas éliminé à parts égales")
    require(any(p["kind"] == "overdraft" and p["amount_eur"] == "42.00" for p in financial_report["positions"]),
            "Le découvert n'est pas un passif positif")
    driver.click(button("Calculer le bilan financier"))
    wait_text(driver, "Couverture ou sources incomplètes")
    driver.execute("document.querySelector('.financial-result').scrollIntoView({block:'start'});")
    driver.screenshot(artifacts / "financial-report.png")
    navigate(driver, "Comptes bancaires")
    choose("treasury-report-account", a["id"])
    driver.click(button("Calculer le solde"))
    wait_text(driver, "-42.00 EUR")
    driver.screenshot(artifacts / "treasury-report.png")
    steps.append({"name": "financial_coverage_ui_internal_elimination_and_partial_net", "status": "passed",
                  "internal_elimination_eur": elimination["eliminated_eur"], "net_eur": None})
    phase5["properties"] = driver.invoke("list_properties_at")
    phase5["rentals"] = driver.invoke("list_rentals_at")
    state = {"debt_request": debt_request, "debt_report": debt_report, "financial_request": financial_request,
             "financial_report": financial_report, "accounts": [a, b, cash], "date": date, "old_sequence": old_k,
             "old_treasury": driver.invoke("list_treasury_at", {"knowledgeSequence": old_k})}
    (artifacts / "financial-oracles.json").write_text(json.dumps(state, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return driver.invoke("list_registry"), state


def metals_company_scenarios(driver, artifacts, steps, phase5, phase6, fx_state):
    """Physical quantities, source prices and company payments through real IPC."""
    registry = driver.invoke("list_registry")
    alice = next(e for e in registry["entities"] if e["first_name"] == "Alice")
    bob = next(e for e in registry["entities"] if e["display_name"] == "Benoît du test desktop")
    outside = next(e for e in registry["entities"] if e["display_name"] == "Tiers du test desktop")
    one = {"num": "1", "den": "1"}
    date = phase6["date"]

    def data(**fields):
        return {"id": None, "expected_revision_id": None, "reason": "Validation métaux et sociétés synthétique", **fields}

    def save(record_kind, **fields):
        return driver.invoke("save_" + record_kind, {"input": data(**fields)})

    def refresh():
        driver.command("POST", "/refresh", {})
        wait_until(lambda: driver.execute("return !!document.querySelector('#global-knowledge') && !document.querySelector('#global-knowledge').disabled;"), "relecture des métaux et sociétés")

    def choose(selector, value):
        driver.click(f"//select[@id='{selector}']/option[@value='{value}']")

    def exact(value):
        return Fraction(int(value["num"]), int(value["den"]))

    def metal_report(knowledge=None):
        return driver.invoke("get_metal_report", {"request": {"lot_id": lot["id"], "date": date, "knowledge_sequence": knowledge}})

    asset = save("asset", name="Lot physique du test", country="FR", category="metal", notes="Stock exact sans double estimation", archived=False)
    right = save("ownership", asset_id=asset["id"], owner_entity_id=alice["id"], legal_share=one, economic_share=one, valid_from="2020-01-01", valid_to=None)
    save("valuation", target_kind="asset", target_id=asset["id"], amount="999999", currency="EUR", effective_on="2020-01-01", source="Estimation générique à remplacer par le lot")
    refresh()
    navigate(driver, "Or et argent")
    driver.click(button("Créer un lot de métal"))
    choose("metal-asset_id", asset["id"])
    driver.fill("#metal-designation", "Deux pièces du test")
    driver.fill("#metal-unit_weight", "1")
    choose("metal-weight_unit", "troy_oz")
    driver.fill("#metal-purity", "0,9")
    driver.fill_date("#metal-tracking_start", "2020-01-01")
    choose("metal-legal_owner_entity_id", alice["id"])
    driver.fill("#metal-storage_location", "Coffre synthétique français")
    driver.fill("#metal-reason", "Lot identifié sans nouvelle valeur générique")
    driver.click(button("Enregistrer la source métal"))
    wait_operation_complete(driver)
    lot = next(l for l in driver.invoke("list_metals_at")["lots"] if l["asset_id"] == asset["id"])
    require(metal_report()["quantity_remaining"] is None, "Le lot sans ouverture a reçu un stock implicite")
    driver.click(button("Confirmer le stock d’ouverture"))
    driver.fill("#metal-quantity_change", "2")
    driver.fill("#metal-cost_amount", "2400")
    driver.fill("#metal-reason", "Deux pièces historiquement acquises, sans nouveau débit")
    driver.click(button("Enregistrer la source métal"))
    wait_operation_complete(driver)
    driver.click(button("Cours et primes"))
    driver.click(button("Ajouter un cours métal"))
    driver.fill_date("#metal-effective_on", "2020-01-01")
    driver.fill("#metal-price", "60")
    driver.fill("#metal-source", "Cours synthétique en EUR par gramme fin")
    driver.fill("#metal-reason", "Cotation manuelle datée du test")
    driver.click(button("Enregistrer la source métal"))
    wait_operation_complete(driver)
    before = metal_report()
    require(exact(before["exact_fine_weight_g"]) == Fraction("55.98625824")
            and before["intrinsic_eur"] == "3359.18", f"O10/O12 poids ou valeur inexacts : {before}")
    require(consolidate(driver, None)["gross_eur"] == "353669.18", "Le cours métal ne remplace pas l'estimation générique")
    old_k = driver.invoke("list_metals_at")["knowledge_sequence"]
    old_metals = driver.invoke("list_metals_at", {"knowledgeSequence": old_k})
    quote = before["quote"]
    corrected = {k:v for k,v in quote.items() if k not in ["revision_id"]}
    corrected.update(expected_revision_id=quote["revision_id"], reason="Correction motivée du cours synthétique", price="61")
    driver.invoke("save_metal_quote", {"input": corrected})
    require(metal_report(old_k)["intrinsic_eur"] == "3359.18" and metal_report()["intrinsic_eur"] == "3415.16", "La correction du cours a écrasé l'ancienne connaissance")
    steps.append({"name": "metal_lot_opening_and_quote_ui_exact_troy_weight_and_old_price", "status": "passed", "fine_weight_g": "55.98625824", "intrinsic_before_eur": "3359.18"})

    bank = phase6["accounts"][1]
    sale = driver.invoke("post_metal_movement", {"input": {
        "expected_sequence": driver.invoke("list_metals_at")["knowledge_sequence"],
        "movement": data(lot_id=lot["id"], kind="sale", effective_on="2024-03-01", quantity_change="-1", cost_amount=None,
                         trade_amount="1800", trade_fees="10", trade_currency="EUR", cash_entity_id=alice["id"], counterparty_entity_id=outside["id"],
                         country="FR", storage_location=lot["storage_location"], unlinked_reason="Journal lié", notes="Vente d'une des deux pièces"),
        "transaction": data(kind="asset_trade", effective_on="2024-03-01", label="Vente métal synthétique", reference="TEST-METAL", notes="",
                            entries=[{"line_id": "metal-sale-line", "account_id": bank["id"], "amount": "1790", "role": "principal"}]),
        "existing_transaction_revision_id": None, "links": [{"line_id": "metal-sale-line", "amount": "1790"}],
    }})
    after_sale = metal_report()
    require(exact(after_sale["exact_quantity"]) == 1 and exact(after_sale["exact_fine_weight_g"]) == Fraction("27.99312912")
            and after_sale["cost_remaining_original"] == "1200.00" and after_sale["realized_gain_eur"] == "590.00",
            f"Vente partielle ou coût proportionnel incorrects : {after_sale}")
    journals = len(driver.invoke("list_treasury_at")["transactions"])
    save("metal_movement", lot_id=lot["id"], kind="storage_transfer", effective_on="2024-04-01", quantity_change="0", cost_amount=None,
         trade_amount=None, trade_fees="0", trade_currency=None, cash_entity_id="", counterparty_entity_id=None, country="CH", storage_location="Coffre synthétique suisse", unlinked_reason="", notes="Même lot, nouveau lieu")
    family_before = exact(consolidate(driver, None)["exact_gross"])
    alice_before = exact(consolidate(driver, alice["id"])["exact_gross"])
    bob_before = exact(consolidate(driver, bob["id"])["exact_gross"])
    close = {k:v for k,v in right.items() if k != "revision_id"}
    close.update(expected_revision_id=right["revision_id"], reason="Clôture pour transfert interne", valid_to="2024-05-01")
    driver.invoke("transfer_metal_ownership", {"input": {"expected_sequence": driver.invoke("list_metals_at")["knowledge_sequence"], "lot_id": lot["id"], "effective_on": "2024-05-01", "reason": "Transfert familial sans gain ni nouvelle quantité", "ownerships": [close, data(asset_id=asset["id"], owner_entity_id=bob["id"], legal_share=one, economic_share=one, valid_from="2024-05-01", valid_to=None)]}})
    require(exact(consolidate(driver, None)["exact_gross"]) == family_before
            and alice_before - exact(consolidate(driver, alice["id"])["exact_gross"]) == Fraction("1707.58087632")
            and exact(consolidate(driver, bob["id"])["exact_gross"]) - bob_before == Fraction("1707.58087632"), "Le transfert de droits a créé un gain ou perdu un stock")
    require(len(driver.invoke("list_treasury_at")["transactions"]) == journals and metal_report()["country"] == "CH", "Le transfert de lieu ou de droits a créé du cash")
    steps.append({"name": "partial_metal_sale_single_cash_proportional_cost_storage_and_owner_transfer", "status": "passed", "net_receipt_eur": "1790.00", "remaining_cost_eur": "1200.00", "remaining_intrinsic_eur": "1707.58"})

    company = save("entity", kind="company", display_name="Société de pilotage du test", first_name=None, last_name=None, birth_date=None, residence_country="FR", preferred_currency="EUR", activity="Activité synthétique", notes="", archived=False)
    save("participation", company_id=company["id"], owner_entity_id=alice["id"], legal_share=one, economic_share=one, voting_share=one, valid_from="2020-01-01", valid_to=None)
    save("company_policy", company_id=company["id"], mode="detailed", coverage_complete=True, liability_qualified=True, valid_from="2020-01-01", valid_to=None)
    co_asset = save("asset", name="Banque société du test", country="FR", category="bank", notes="", archived=False)
    save("ownership", asset_id=co_asset["id"], owner_entity_id=company["id"], legal_share=one, economic_share=one, valid_from="2020-01-01", valid_to=None)
    co_account = save("treasury_account", asset_id=co_asset["id"], kind="bank", currency="EUR", tracking_start="2020-01-01", legal_owner_entity_id=company["id"], bank_name="Banque société synthétique", reference="CO-TEST", country="FR", storage_location="", notes="", archived=False)
    save("treasury_transaction", kind="opening", effective_on="2020-01-01", label="Ouverture société", reference="", notes="Stock historique", entries=[{"line_id": "co-opening", "account_id": co_account["id"], "amount": "1000", "role": "opening"}])
    money_before = driver.invoke("get_financial_report", {"request": phase6["financial_request"]})
    refresh()
    navigate(driver, "Sociétés")
    driver.click(button("Indicateurs et dividendes"))
    driver.click(button("Ajouter une observation de société"))
    choose("company-company_id", company["id"])
    choose("company-frequency", "annual")
    driver.fill_date("#company-period_start", "2023-01-01")
    driver.fill_date("#company-period_end", "2024-01-01")
    for field, value in [("revenue", "120000"), ("operating_expenses", "80000"), ("operating_result", "40000"), ("net_result", "30000"), ("indicative_cash", "999999")]:
        driver.fill("#company-" + field, value)
    driver.fill("#company-reason", "Observation de gestion sans écriture de banque")
    driver.click(button("Enregistrer l’observation de société"))
    wait_operation_complete(driver)
    money_after = driver.invoke("get_financial_report", {"request": phase6["financial_request"]})
    require(money_before["known_assets_eur"] == money_after["known_assets_eur"] and money_before["known_liabilities_eur"] == money_after["known_liabilities_eur"], "Un indicateur de société a été ajouté au patrimoine")
    declaration = save("company_distribution", company_id=company["id"], beneficiary_entity_id=alice["id"], kind="declared", effective_on="2024-05-01", amount="300", currency="EUR", related_declaration_id=None, unlinked_reason="", notes="Décision distincte de son paiement", cash_links=[])
    driver.invoke("post_company_distribution", {"input": {
        "expected_sequence": driver.invoke("list_companies_at")["knowledge_sequence"],
        "distribution": data(company_id=company["id"], beneficiary_entity_id=alice["id"], kind="paid", effective_on="2024-06-01", amount="300", currency="EUR", related_declaration_id=declaration["id"], unlinked_reason="Deux faces suivies", notes="Paiement interne réel", cash_links=[]),
        "transaction": data(kind="dividend", effective_on="2024-06-01", label="Dividende synthétique", reference="DIV-TEST", notes="Deux lignes, une source", entries=[{"line_id": "dividend-payer", "account_id": co_account["id"], "amount": "-300", "role": "principal"}, {"line_id": "dividend-beneficiary", "account_id": bank["id"], "amount": "300", "role": "principal"}]),
        "existing_transaction_revision_id": None, "links": [{"side": "payer", "line_id": "dividend-payer", "amount": "-300"}, {"side": "beneficiary", "line_id": "dividend-beneficiary", "amount": "300"}],
    }})
    money_final = driver.invoke("get_financial_report", {"request": phase6["financial_request"]})
    require(money_final["known_assets_eur"] == money_after["known_assets_eur"], "Le dividende familial a créé un gain")
    company_request = {"company_id": company["id"], "date": date, "frequency": "annual", "knowledge_sequence": None}
    co_report = driver.invoke("get_company_report", {"request": company_request})
    require(co_report["financial_periods"][0]["calculated_operating_result"] == "40000.00"
            and any(d["distribution"]["kind"] == "paid" and d["payer_cash_complete"] and d["beneficiary_cash_complete"] for d in co_report["distributions"]), "Indicateurs ou rapprochements des dividendes incorrects")
    steps.append({"name": "company_period_ui_indicators_excluded_and_two_sided_dividend_no_family_gain", "status": "passed"})
    refresh()
    navigate(driver, "Sociétés")
    driver.click(button("Indicateurs et dividendes"))
    choose("company-filter", company["id"])
    driver.click(button("Consulter une société"))
    driver.click(button("Calculer le contrôle de société"))
    wait_text(driver, "40000.00 EUR")
    driver.execute("document.documentElement.style.scrollBehavior='auto'; document.querySelector('.company-result').scrollIntoView({block:'start'});")
    driver.screenshot(artifacts / "company-report.png")
    navigate(driver, "Or et argent")
    choose("global-knowledge", str(old_k))
    wait_operation_complete(driver)
    choose("metal-filter", lot["id"])
    driver.click(button("Contrôler un lot"))
    driver.click(button("Calculer le lot de métal"))
    wait_text(driver, "3359.18 EUR")
    require("Lecture seule historique" in driver.text(), "La consultation métal à K n'est pas en lecture seule")
    choose("global-knowledge", "current")
    wait_operation_complete(driver)
    choose("metal-filter", lot["id"])
    driver.click(button("Contrôler un lot"))
    driver.click(button("Calculer le lot de métal"))
    wait_text(driver, "1707.58 EUR")
    driver.execute("document.documentElement.style.scrollBehavior='auto'; document.querySelector('.metal-report-result').scrollIntoView({block:'start'});")
    driver.screenshot(artifacts / "metal-report.png")
    steps.append({"name": "company_and_metal_rust_reports_ui_with_historical_read_only", "status": "passed"})
    fx_state["current_total"] = consolidate(driver, None)["gross_eur"]
    phase6["financial_report"] = money_final
    phase6["expected_balances"] = ["-42.00", "2430.00", "330.000"]
    phase5["properties"] = driver.invoke("list_properties_at")
    phase5["rentals"] = driver.invoke("list_rentals_at")
    state = {"lot_id": lot["id"], "old_sequence": old_k, "old_metals": old_metals, "company_request": company_request,
             "metal_request": {"lot_id": lot["id"], "date": date, "knowledge_sequence": None}, "co_account": co_account,
             "sale_id": sale["id"], "family_gross_eur": fx_state["current_total"], "old_metal_report": metal_report(old_k)}
    (artifacts / "metal-company-oracles.json").write_text(json.dumps(state, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return driver.invoke("list_registry"), state


def document_library_scenarios(driver, temporary, artifacts, steps, source, content, phase7):
    """Versioned originals, shared links, logical removal and old K in real UI."""
    library = driver.invoke("list_document_library_at")
    original = library["documents"][0]
    old_k = library["knowledge_sequence"]
    old_library = driver.invoke("list_document_library_at", {"knowledgeSequence": old_k})
    registry = driver.invoke("list_registry")
    alice = next(e for e in registry["entities"] if e["first_name"] == "Alice")
    driver.click(button("Modifier la description ou le statut"))
    driver.fill("#document-description", "Pièce familiale synthétique")
    driver.fill("#document-reason", "Description documentaire vérifiée")
    driver.click(button("Enregistrer la description et le statut"))
    wait_operation_complete(driver)
    for target, role in [(alice["id"], "FAMILLE-TEST"), (phase7["lot_id"], "METAL-TEST")]:
        driver.click(button("Associer un objet"))
        driver.click(f"//select[@id='document-link-target']/option[@value='{target}']")
        driver.fill("#document-link-role", role)
        driver.fill("#document-reason", "Association explicite au même original")
        driver.click(button("Enregistrer l’association documentaire"))
        wait_operation_complete(driver)
    linked = driver.invoke("list_document_library_at")
    require(len(linked["links"]) == 2, "Les associations multiples n'ont pas été conservées")
    driver.click("//tr[td[contains(., 'FAMILLE-TEST')]]//button[normalize-space(.)='Retirer l’association']")
    driver.fill("#document-link-remove-reason", "Retrait d'une association sans retirer la pièce")
    driver.click(button("Confirmer le retrait de l’association"))
    wait_operation_complete(driver)
    remaining = driver.invoke("list_document_library_at")
    require(len(remaining["documents"]) == 1 and len(remaining["links"]) == 1
            and remaining["links"][0]["target_object_id"] == phase7["lot_id"], "Le retrait a perdu la pièce ou son autre association")
    replacement = temporary / "piece-remplacee.txt"
    replacement_bytes = "Nouvel original synthétique, caractères accentués.\n".encode("utf-8")
    replacement.write_bytes(replacement_bytes)
    driver.click(button("Remplacer l’original"))
    driver.fill("#document-reason", "Nouvelle version sans écraser l'original historique")
    choose_native_file(driver, "Choisir et enregistrer le nouvel original", "Remplacer l’original du document", replacement)
    wait_operation_complete(driver)
    versioned = driver.invoke("list_document_library_at")
    current = next(d for d in versioned["documents"] if d["id"] == original["id"])
    require(current["sha256"] == hashlib.sha256(replacement_bytes).hexdigest()
            and any(v["id"] == original["id"] and v["sha256"] == original["sha256"] for v in versioned["versions"]), "Le remplacement a perdu les anciens octets ou l'identité logique")
    driver.click("//summary[normalize-space(.)='Versions antérieures de ce document']")
    old_copy = artifacts / "document-original-historique.txt"
    choose_native_file(driver, "Exporter cette version", "Exporter une copie du document", old_copy)
    wait_text(driver, "a été exportée.")
    require(old_copy.read_bytes() == content, "L'export de la version historique n'a pas retrouvé les octets originaux")
    # A second logical document reuses the historical blob, then leaves the
    # active classification through an explicit confirmation.
    choose_native_file(driver, "Importer un document", "Importer un document", source)
    wait_text(driver, "a été importé dans le coffre.")
    wait_operation_complete(driver)
    imported = driver.invoke("list_document_library_at")
    second = next(d for d in imported["documents"] if d["id"] != original["id"])
    require(second["sha256"] == original["sha256"], "Le contenu identique n'a pas gardé le même hash")
    section = "//section[div/div/h2[normalize-space(.)='piece-test.txt']]"
    driver.click(section + "//button[normalize-space(.)='Supprimer le document']")
    driver.fill("#document-reason", "Retrait logique confirmé, historique conservé")
    require(not driver.command("GET", f"/element/{driver.find(button('Enregistrer la description et le statut'))}/enabled"), "Le retrait n'exige pas sa confirmation")
    driver.click("//input[@id='document-delete-confirm']")
    driver.click(button("Enregistrer la description et le statut"))
    wait_operation_complete(driver)
    removed = driver.invoke("list_document_library_at")
    require(next(d for d in removed["documents"] if d["id"] == second["id"])["status"] == "deleted"
            and len(driver.invoke("list_documents")) == 1, "Le retrait logique a perdu l'historique ou gardé le classement actif")
    # Exact adverse text is created through the real typed command; textareas
    # normalize CRLF on input, so the IPC fixture preserves those original bytes.
    current = next(d for d in removed["documents"] if d["id"] == original["id"])
    literal = '=HYPERLINK("https://invalid.example","Texte littéral")\r\nAccents é, guillemets " et virgule,\t0\n_x0000_'
    driver.invoke("save_document_metadata", {"input": {"id": current["id"], "expected_revision_id": current["revision_id"], "reason": "Oracle CSV et XLSX sans formule active", "description": literal, "status": "active", "confirmed": False}})
    company = driver.invoke("get_company_report", {"request": phase7["company_request"]})["company"]
    driver.invoke("save_company_financial_period", {"input": {"id": None, "expected_revision_id": None, "reason": "Oracle de précision et de NULL", "company_id": company["id"], "period_start": "2024-01-01", "period_end": "2024-02-01", "frequency": "monthly", "currency": "EUR", "revenue": "12345678901234567890.1234", "operating_expenses": None, "operating_result": None, "net_result": "0", "indicative_cash": None, "indicative_debt": None, "important_receivables": None, "dividends_declared": None, "indicative_global_value": None, "notes": ""}})
    driver.command("POST", "/refresh", {})
    wait_until(lambda: driver.execute("return !!document.querySelector('#global-knowledge') && !document.querySelector('#global-knowledge').disabled;"), "historique documentaire rechargé")
    navigate(driver, "Documents")
    driver.click(f"//select[@id='global-knowledge']/option[@value='{old_k}']")
    wait_operation_complete(driver)
    wait_text(driver, "Lecture seule historique")
    import_button = wait_until(lambda: driver.find(button("Importer un document")),
                               "bibliothèque documentaire historique chargée")
    require(not driver.command("GET", f"/element/{import_button}/enabled"), "La bibliothèque historique permettrait un nouvel import")
    require(canonical_registry(driver.invoke("list_document_library_at", {"knowledgeSequence": old_k})) == canonical_registry(old_library), "L'ancienne bibliothèque a changé")
    driver.capture_section(".registry-workspace", artifacts / "documents-historical.png")
    driver.click("//select[@id='global-knowledge']/option[@value='current']")
    wait_operation_complete(driver)
    wait_until(lambda: driver.command("GET", f"/element/{driver.find(button('Importer un document'))}/enabled"), "retour au classement documentaire actuel")
    driver.capture_section(".registry-workspace", artifacts / "documents-current.png")
    steps.append({"name": "document_library_ui_versions_shared_links_logical_removal_and_historical_K", "status": "passed"})
    return {"old_sequence": old_k, "old_library": old_library, "original": original, "active_id": original["id"], "deleted_id": second["id"], "literal_description": literal}


def csv_portability_scenarios(driver, artifacts, steps, source_library, snapshots, registry, phase5, phase6, phase7, references):
    """A separate reconstruction, with source-prefix equality and provenance."""
    navigate(driver, "Sauvegarde et export")
    source_status = driver.invoke("get_status")
    source_k = source_status["mutation_sequence"]
    csv_archive = artifacts / "desktop-export-csv.zip"
    workbook = artifacts / "desktop-export.xlsx"
    choose_native_file(driver, "Exporter une archive CSV intégrale", "Exporter une archive CSV intégrale", csv_archive)
    wait_text(driver, "L’archive CSV intégrale a été créée", timeout=60)
    choose_native_file(driver, "Exporter un classeur XLSX complet", "Exporter un classeur XLSX complet", workbook)
    wait_text(driver, "Le classeur XLSX complet a été créé", timeout=60)
    require(csv_archive.is_file() and workbook.is_file(), "Un des exports complets est absent")
    require(driver.invoke("get_status")["mutation_sequence"] == source_k, "Un export a modifié les données source")
    with zipfile.ZipFile(csv_archive) as z:
        manifest = json.loads(z.read("manifest.json"))
        require(manifest["archive_kind"] == "structured_csv" and manifest["mutation_sequence"] == source_k, "Le CSV ne décrit pas le snapshot source")
        require("data/patrimoine.sqlite3" not in z.namelist() and len([n for n in z.namelist() if n.startswith("tables/")]) == 62, "L'archive CSV ne contient pas ses 62 tables indépendantes")
    with zipfile.ZipFile(workbook) as z:
        require("xl/workbook.xml" in z.namelist() and not any("vbaProject" in n for n in z.namelist()), "Classeur OOXML incomplet ou actif")
    steps.append({"name": "whole_vault_csv_and_xlsx_exports_through_native_dialogs", "status": "passed", "source_sequence": source_k})
    choose_native_file(driver, "Choisir une archive CSV à analyser", "Analyser une archive CSV Patrimoine", csv_archive)
    wait_text(driver, "Vérifier avant de réimporter", timeout=60)
    require(driver.invoke("get_status")["generation_id"] == source_status["generation_id"]
            and driver.invoke("get_status")["mutation_sequence"] == source_k, "L'analyse CSV a modifié le coffre actif")
    driver.click(button("Annuler"))
    require(driver.invoke("get_status")["generation_id"] == source_status["generation_id"], "Annuler le CSV a changé la génération")
    choose_native_file(driver, "Choisir une archive CSV à analyser", "Analyser une archive CSV Patrimoine", csv_archive)
    wait_text(driver, "Vérifier avant de réimporter", timeout=60)
    safety = artifacts / "desktop-before-csv-import.zip"
    require(driver.execute("return document.querySelector('.checkbox-label input').checked;"), "La sauvegarde préalable CSV n'est pas proposée par défaut")
    choose_native_file(driver, "Confirmer le remplacement du coffre", "Sauvegarder l’état actuel avant restauration", safety)
    wait_text(driver, "L’archive CSV a été réimportée.", timeout=60)
    after = driver.invoke("get_status")
    require(after["generation_id"] != source_status["generation_id"] and after["mutation_sequence"] == source_k + 1,
            "Le CSV n'a pas activé une candidate avec exactement un événement de provenance")
    prefix_commands = {"list_registry_at": registry, "list_document_library_at": source_library, **snapshots}
    for command, expected in prefix_commands.items():
        actual = driver.invoke(command, {"knowledgeSequence": source_k})
        require(canonical_registry(actual) == canonical_registry(expected), f"Le préfixe source a changé après réimport CSV : {command}")
    csv_reports = {}
    for name, (command, request, expected) in references.items():
        actual = driver.invoke(command, {"request": {**request, "knowledge_sequence": source_k}})
        require(actual == expected, f"Le calcul source {name} diffère après réimport CSV")
        csv_reports[name] = actual
    for command, expected in prefix_commands.items():
        now = driver.invoke(command)
        require({k:v for k,v in canonical_registry(now).items() if k != "knowledge_sequence"}
                == {k:v for k,v in canonical_registry(expected).items() if k != "knowledge_sequence"}, f"Les données courantes ont changé hors provenance : {command}")
    (artifacts / "csv-roundtrip-reports.json").write_text(json.dumps({"source_sequence": source_k, "import_sequence": after["mutation_sequence"], "reports": csv_reports}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    steps.append({"name": "csv_inspect_cancel_confirm_new_generation_and_all_source_K_reports", "status": "passed"})
    result = artifacts / "desktop-csv-roundtrip-backup.zip"
    choose_native_file(driver, "Créer une sauvegarde complète", "Enregistrer une sauvegarde complète", result)
    wait_text(driver, "La sauvegarde complète a été créée.", timeout=60)
    require(result.is_file() and safety.is_file(), "Une sauvegarde du réimport CSV manque")
    steps.append({"name": "csv_reconstructed_vault_complete_backup_for_independent_comparison", "status": "passed"})


def native_scenarios(driver, temporary, artifacts, steps, registry_baseline, fx_state, phase5, phase6, phase7):
    require(shutil.which("xdotool"), "xdotool requis pour --native-dialogs")
    source = temporary / "piece-test.txt"
    content = "Pièce synthétique du test desktop Patrimoine.\nAucune donnée réelle.\n".encode("utf-8")
    source.write_bytes(content)
    choose_native_file(driver, "Importer un document", "Importer un document", source)
    wait_text(driver, "a été importé dans le coffre.")
    documents = driver.invoke("list_documents")
    require(len(documents) == 1, f"Nombre de documents inattendu : {len(documents)}")
    require(documents[0]["sha256"] == hashlib.sha256(content).hexdigest(), "SHA-256 import différent")
    require(documents[0]["original_name"] == source.name, "Nom original perdu")
    steps.append({"name": "native_document_import", "status": "passed", "sha256": documents[0]["sha256"]})

    copied = artifacts / "document-export.txt"
    choose_native_file(driver, "Exporter une copie", "Exporter une copie du document", copied)
    wait_text(driver, "a été exportée.")
    require(copied.read_bytes() == content, "La copie exportée diffère de l'original")
    steps.append({"name": "native_document_export", "status": "passed"})
    phase8 = document_library_scenarios(driver, temporary, artifacts, steps, source, content, phase7)

    navigate(driver, "Sauvegarde et export")
    archive = artifacts / "desktop-backup.zip"
    # L'import documentaire est une mutation réelle postérieure au scénario
    # familial. Comparer la restauration au snapshot effectivement sauvegardé,
    # séquence de connaissance comprise, et vérifier auparavant que l'import
    # n'a changé aucun enregistrement du registre patrimonial.
    registry_at_backup = driver.invoke("list_registry")
    before_records = {key: value for key, value in canonical_registry(registry_baseline).items()
                      if key != "knowledge_sequence"}
    backup_records = {key: value for key, value in canonical_registry(registry_at_backup).items()
                      if key != "knowledge_sequence"}
    require(before_records == backup_records, "L'import documentaire a modifié les enregistrements patrimoniaux")
    require(registry_at_backup["knowledge_sequence"] > registry_baseline["knowledge_sequence"],
            "L'import documentaire n'a pas été enregistré dans l'ordre de connaissance")
    properties_at_backup = driver.invoke("list_properties_at")
    rentals_at_backup = driver.invoke("list_rentals_at")
    library_at_backup = driver.invoke("list_document_library_at")
    financial_snapshots = {command: driver.invoke(command) for command in ["list_treasury_at", "list_debts_at", "list_finance_at", "list_metals_at", "list_companies_at"]}
    metal_at_backup = driver.invoke("get_metal_report", {"request": phase7["metal_request"]})
    company_at_backup = driver.invoke("get_company_report", {"request": phase7["company_request"]})
    # Importing the original increments K. FX traces in positions carry that K,
    # so restore must reproduce the report actually captured by the archive.
    financial_at_backup = driver.invoke("get_financial_report", {"request": phase6["financial_request"]})
    references = {
        "financial": ("get_financial_report", phase6["financial_request"], financial_at_backup),
        "metal": ("get_metal_report", phase7["metal_request"], metal_at_backup),
        "company": ("get_company_report", phase7["company_request"], company_at_backup),
        "rental": ("get_rental_report", phase5["report_request"], driver.invoke("get_rental_report", {"request": phase5["report_request"]})),
        "debt": ("get_debt_report", phase6["debt_request"], driver.invoke("get_debt_report", {"request": phase6["debt_request"]})),
    }
    for index, account in enumerate([*phase6["accounts"], phase7["co_account"]]):
        request = {"account_id": account["id"], "date": phase6["date"], "knowledge_sequence": None}
        references[f"treasury-{index}"] = ("get_treasury_report", request, driver.invoke("get_treasury_report", {"request": request}))
    for field in ["net_eur", "known_assets_eur", "known_liabilities_eur", "known_net_eur", "eliminations"]:
        require(financial_at_backup[field] == phase6["financial_report"][field],
                f"L'import documentaire a changé le calcul financier : {field}")
    for before, after in [(phase5["properties"], properties_at_backup), (phase5["rentals"], rentals_at_backup)]:
        require({k:v for k,v in canonical_registry(before).items() if k != "knowledge_sequence"}
                == {k:v for k,v in canonical_registry(after).items() if k != "knowledge_sequence"},
                "L’import documentaire a modifié les sources immobilières ou locatives")
    choose_native_file(driver, "Créer une sauvegarde complète", "Enregistrer une sauvegarde complète", archive)
    wait_text(driver, "La sauvegarde complète a été créée.")
    require(archive.is_file(), "Archive native absente")
    with zipfile.ZipFile(archive) as opened:
        require(opened.testzip() is None, "Archive ZIP corrompue")
        require("manifest.json" in opened.namelist(), "Manifeste absent")
    steps.append({"name": "native_complete_backup", "status": "passed", "path": str(archive)})

    save_settings_ui(driver, "État courant avant restauration", "TND")
    current = driver.invoke("get_status")
    navigate(driver, "Sauvegarde et export")
    choose_native_file(driver, "Choisir une sauvegarde à analyser", "Analyser une sauvegarde Patrimoine", archive)
    wait_text(driver, "Vérifier avant de restaurer")
    require("Coffre du test desktop" in driver.text(), "Le résumé ne présente pas l'espace sauvegardé")
    inspected = driver.invoke("get_status")
    require(current["generation_id"] == inspected["generation_id"], "L'analyse a changé la génération")
    require(current["mutation_sequence"] == inspected["mutation_sequence"], "L'analyse a modifié le coffre")
    require(driver.execute("return document.querySelector('.checkbox-label input').checked;"),
            "La sauvegarde préalable n'est pas cochée par défaut")
    driver.click(button("Annuler"))
    require("Vérifier avant de restaurer" not in driver.text(), "Le résumé reste affiché après annulation")
    require(driver.invoke("get_status")["generation_id"] == current["generation_id"],
            "L'annulation a changé le coffre")
    steps.append({"name": "native_inspect_and_cancel_without_mutation", "status": "passed"})

    choose_native_file(driver, "Choisir une sauvegarde à analyser", "Analyser une sauvegarde Patrimoine", archive)
    wait_text(driver, "Vérifier avant de restaurer")
    safety = artifacts / "desktop-before-restore.zip"
    restore_started = time.monotonic()
    choose_native_file(driver, "Confirmer le remplacement du coffre",
                       "Sauvegarder l’état actuel avant restauration", safety)
    wait_text(driver, "La sauvegarde a été restaurée.", timeout=60)
    restored = driver.invoke("get_status")
    require(restored["settings"]["workspace_name"] == "Coffre du test desktop", "Nom sauvegardé non restauré")
    require(restored["settings"]["display_currency"] == "CHF", "Devise sauvegardée non restaurée")
    require(restored["generation_id"] != current["generation_id"], "La restauration n'a pas activé de génération")
    require(canonical_registry(driver.invoke("list_document_library_at")) == canonical_registry(library_at_backup), "Les documents, originaux, versions ou associations n'ont pas été restaurés")
    require(safety.is_file(), "La sauvegarde préalable choisie est absente")
    with zipfile.ZipFile(safety) as opened:
        require(opened.testzip() is None, "ZIP préalable corrompu")
    steps.append({"name": "native_confirm_restore_with_safety_backup", "status": "passed", "elapsed_seconds": round(time.monotonic() - restore_started, 2)})
    restored_registry = driver.invoke("list_registry")
    require(canonical_registry(restored_registry) == canonical_registry(registry_at_backup),
            "Les identités, révisions, droits ou valorisations du registre ont changé après restauration")
    require(consolidate(driver, None)["gross_eur"] == fx_state["current_total"],
            "Les attributions familiales ne sont pas reproduites après restauration")
    steps.append({"name": "registry_ids_rights_and_consolidation_after_restore", "status": "passed"})
    require(consolidate(driver, None, fx_state["old_sequence"], fx_state["date"])["gross_eur"] == fx_state["old_total"],
            "Le résultat historique du change n'est pas retrouvé après restauration")
    historical = driver.invoke("list_registry_at", {"knowledgeSequence": fx_state["old_sequence"]})
    old_rate = next(row for row in historical["exchange_rates"] if row["id"] == fx_state["rate_id"])
    new_rate = next(row for row in restored_registry["exchange_rates"] if row["id"] == fx_state["rate_id"])
    require(old_rate["revision_id"] == fx_state["old_revision_id"]
            and new_rate["revision_id"] == fx_state["current_revision_id"],
            "Les révisions actuelle et historique du cours ne sont pas conservées après restauration")
    steps.append({"name": "exchange_rates_and_historical_results_survive_restore", "status": "passed"})
    for profile in fx_state["profiles"]:
        revisions = {row["revision_id"] for row in restored_registry[profile["collection"]]
                     if row["id"] == profile["target_id"]}
        require({profile["old_revision_id"], profile["current_revision_id"]} <= revisions,
                "La restauration a perdu une révision historique de profil")
    steps.append({"name": "all_entity_and_asset_profile_versions_survive_restore", "status": "passed"})

    require(canonical_registry(driver.invoke("list_properties_at")) == canonical_registry(properties_at_backup),
            "Les fiches, unités ou cessions diffèrent après restauration")
    require(canonical_registry(driver.invoke("list_rentals_at")) == canonical_registry(rentals_at_backup),
            "Les baux, termes historiques, échéances, paiements ou affectations diffèrent après restauration")
    restored_rent = driver.invoke("get_rental_report", {"request": phase5["report_request"]})
    for field in ["rent_due_eur", "rent_received_eur", "outstanding_rent_eur", "noi_eur", "gross_yield_percent", "net_yield_percent"]:
        require(restored_rent[field] == phase5["report"][field], f"Indicateur {field} différent après restauration")
    steps.append({"name": "properties_leases_allocations_and_rental_indicators_survive_restore", "status": "passed"})
    for command, before in financial_snapshots.items():
        require(canonical_registry(driver.invoke(command)) == canonical_registry(before),
                f"Sources financières ou révisions différentes après restauration : {command}")
    require(canonical_registry(driver.invoke("list_treasury_at", {"knowledgeSequence": phase6["old_sequence"]}))
            == canonical_registry(phase6["old_treasury"]), "La restauration a modifié l'ancien journal")
    restored_debt = driver.invoke("get_debt_report", {"request": phase6["debt_request"]})
    require(restored_debt["balance"] == phase6["debt_report"]["balance"], "Le prêt a changé après restauration")
    restored_finance = driver.invoke("get_financial_report", {"request": phase6["financial_request"]})
    (artifacts / "financial-restore-comparison.json").write_text(json.dumps({"before": financial_at_backup, "after": restored_finance}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    for field in ["net_eur", "known_assets_eur", "known_liabilities_eur", "known_net_eur", "eliminations", "positions"]:
        require(restored_finance[field] == financial_at_backup[field], f"Bilan financier différent après restauration : {field}")
    for account, expected in zip(phase6["accounts"], phase6["expected_balances"]):
        restored_balance = driver.invoke("get_treasury_report", {"request": {"account_id": account["id"], "date": phase6["date"], "knowledge_sequence": None}})
        require(restored_balance["balance_original"] == expected, "Solde financier différent après restauration")
    steps.append({"name": "financial_sources_cash_links_debts_deposits_and_old_knowledge_survive_restore", "status": "passed"})
    restored_metal = driver.invoke("get_metal_report", {"request": phase7["metal_request"]})
    restored_company = driver.invoke("get_company_report", {"request": phase7["company_request"]})
    require(restored_metal == metal_at_backup and restored_company == company_at_backup, "Les contrôles métal ou société diffèrent après restauration")
    require(canonical_registry(driver.invoke("list_metals_at", {"knowledgeSequence": phase7["old_sequence"]})) == canonical_registry(phase7["old_metals"]), "L'ancienne cotation ou le stock historique a été perdu")
    old_request = {**phase7["metal_request"], "knowledge_sequence": phase7["old_sequence"]}
    require(driver.invoke("get_metal_report", {"request": old_request}) == phase7["old_metal_report"], "L'ancien calcul métal n'est pas reproduit")
    (artifacts / "metal-company-restore-comparison.json").write_text(json.dumps({"metal_before": metal_at_backup, "metal_after": restored_metal, "company_before": company_at_backup, "company_after": restored_company}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    steps.append({"name": "metal_quantities_prices_costs_transfers_and_company_distributions_survive_restore", "status": "passed"})
    require(canonical_registry(driver.invoke("list_document_library_at", {"knowledgeSequence": phase8["old_sequence"]})) == canonical_registry(phase8["old_library"]), "L'ancienne bibliothèque documentaire n'est pas reproduite après restauration")
    all_snapshots = {"list_properties_at": properties_at_backup, "list_rentals_at": rentals_at_backup, **financial_snapshots}
    csv_portability_scenarios(driver, artifacts, steps, library_at_backup, all_snapshots, registry_at_backup, phase5, phase6, phase7, references)


def dashboard_scenarios(driver, artifacts, steps, phase6, phase7):
    """Read-only dashboards on the rich restored fixture, with rational oracles."""
    from fractions import Fraction

    before = driver.invoke("get_status")
    references = json.loads((artifacts / "csv-roundtrip-reports.json").read_text())
    source_k = references["source_sequence"]
    date = phase6["date"]
    request = {"context": "family", "entity_id": None, "date": date,
               "from_date": date[:4] + "-01-01", "interval": "monthly", "knowledge_sequence": source_k,
               "filters": {"country": None, "currency": None, "category": None, "owner_entity_id": None, "query": ""}}

    def ratio(value):
        return Fraction(int(value["num"]), int(value["den"]))

    def reconcile(report):
        positions = report["positions"]
        assets = sum((ratio(p["exact_eur"]) for p in positions if p["side"] == "asset" and p["exact_eur"] is not None), Fraction(0))
        liabilities = sum((ratio(p["exact_eur"]) for p in positions if p["side"] == "liability" and p["exact_eur"] is not None), Fraction(0))
        require(assets == ratio(report["totals"]["exact_known_assets"])
                and liabilities == ratio(report["totals"]["exact_known_liabilities"]), "Les positions du tableau de bord ne réconcilient pas les totaux exacts")
        for key in ["by_category", "by_country", "by_currency"]:
            require(sum((ratio(b["exact_assets"]) for b in report[key]), Fraction(0)) == assets
                    and sum((ratio(b["exact_liabilities"]) for b in report[key]), Fraction(0)) == liabilities,
                    f"La répartition {key} ne réconcilie pas le bilan exact")

    family = driver.invoke("get_dashboard", {"request": request})
    require(family["financial"] == references["reports"]["financial"], "Le tableau de bord a modifié le bilan financier source")
    require(family["totals"]["net_eur"] is None and family["totals"]["known_assets_eur"] == "355949.88"
            and family["totals"]["known_liabilities_eur"] == "100242.00", "Le tableau de bord a masqué les sources manquantes ou changé les sous-totaux")
    require(family["history"][0]["date"] == request["from_date"] and family["history"][-1]["date"] == date
            and family["history"][-1]["totals"]["net_eur"] is None
            and all(p["totals"]["net_eur"] is None for p in family["history"] if p["status"] != "complete"), "La courbe invente une couverture complète")
    require(ratio(family["history"][-1]["totals"]["exact_known_net"]) == ratio(family["totals"]["exact_known_net"]), "Le dernier point historique diffère du bilan courant")
    reconcile(family)
    require(any(p["side"] == "contingent" for p in family["positions"]), "La garantie hors bilan a disparu")
    navigate(driver, "Tableau de bord")
    driver.click("//select[@id='global-context']/option[@value='family']")
    driver.click(f"//select[@id='global-knowledge']/option[@value='{source_k}']")
    wait_until(lambda: driver.execute("return document.querySelector('.dashboard-results')?.dataset.knowledgeSequence;") == str(source_k), "tableau de bord historique chargé", timeout=60)
    require(driver.execute("return document.querySelector('[aria-label=\"Patrimoine net\"] strong').dataset.value;") == "unknown", "L'interface présente un patrimoine net complet fictif")
    wait_text(driver, "355\u202f949,88 EUR")
    driver.capture_section(".dashboard-total-grid", artifacts / "dashboard-family.png")
    driver.execute("document.querySelector('.dashboard-chart-table summary').click();")
    wait_text(driver, "Historique des positions calculées en EUR")
    require(driver.execute("return document.querySelector('.dashboard-chart-table').querySelectorAll('tbody tr').length;") == len(family["history"]), "Le tableau accessible n'expose pas tous les points réels")
    steps.append({"name": "dashboard_family_exact_residual_allocations_partial_net_and_historical_chart", "status": "passed", "source_sequence": source_k, "points": len(family["history"])})

    filtered_request = {**request, "filters": {**request["filters"], "category": "or"}}
    filtered = driver.invoke("get_dashboard", {"request": filtered_request})
    reconcile(filtered)
    require(filtered["positions"] and all(p["category"] == "or" for p in filtered["positions"])
            and filtered["totals"]["known_assets_eur"] == "1707.58" and filtered["totals"]["net_eur"] is None,
            "Le filtre métal a doublé une estimation générique ou masqué l'incomplétude")
    driver.click("//select[@id='dashboard-category']/option[@value='or']")
    driver.click(button("Actualiser le tableau de bord"))
    wait_until(lambda: driver.execute("return !!document.querySelector('.dashboard-results') && !document.querySelector('.dashboard-loading');"), "filtre or calculé", timeout=60)
    wait_text(driver, "1\u202f707,58 EUR")
    export_path = artifacts / "dashboard-filtered-report.json"
    choose_native_file(driver, "Exporter le rapport filtré", "Exporter le rapport filtré", export_path)
    wait_text(driver, "Le rapport filtré a été exporté en JSON", timeout=60)
    envelope = json.loads(export_path.read_text())
    require(envelope["report_kind"] == "patrimoine_filtered_report" and envelope["schema_version"] == 6
            and envelope["report"]["request"] == filtered["request"], "Le rapport exporté ne conserve pas son contexte filtré à K fixe")
    require(envelope["report"]["positions"] == filtered["positions"] and envelope["report"]["financial"] is None
            and envelope["report"]["rental"] is None and envelope["report"]["variation"] is None, "L'export filtré inclut un détail financier non filtré")
    driver.capture_section(".dashboard-total-grid", artifacts / "dashboard-filtered.png")
    steps.append({"name": "dashboard_gold_filter_and_native_filtered_report_export", "status": "passed"})

    member = family["members"][0]
    person_request = {**request, "context": "person", "entity_id": member["entity_id"]}
    person = driver.invoke("get_dashboard", {"request": person_request})
    reconcile(person)
    require(person["totals"] == member["totals"], "La vue individuelle et la répartition par membre divergent")
    driver.click("//select[@id='dashboard-category']/option[@value='']")
    driver.click(f"//select[@id='global-context']/option[@value='{member['entity_id']}']")
    wait_until(lambda: driver.execute("return !!document.querySelector('.dashboard-results') && !document.querySelector('.dashboard-loading');"), "bilan individuel chargé", timeout=60)
    require(driver.execute("return document.querySelector('[aria-label=\"Patrimoine net\"] strong').dataset.value;") == (person["totals"]["net_eur"] or "unknown"), "La carte individuelle diffère du moteur")
    driver.click("//select[@id='global-context']/option[@value='family']")
    wait_until(lambda: driver.execute("return !!document.querySelector('.dashboard-results') && !document.querySelector('.dashboard-loading');"), "périmètre familial retrouvé", timeout=60)
    company_id = phase7["company_request"]["company_id"]
    driver.fill_date("#dashboard-from-date", "2023-01-01")
    driver.click(f"//select[@id='dashboard-company']/option[@value='{company_id}']")
    wait_text(driver, "Pilotage de la société", timeout=60)
    company_request = {**request, "context": "company", "entity_id": company_id, "from_date": "2023-01-01"}
    company = driver.invoke("get_dashboard", {"request": company_request})
    require(company["financial"] is None and company["totals"] is None and not company["history"], "Le pilotage société invente un net par addition d'indicateurs")
    require(any(p["period"]["revenue"] == "12345678901234567890.1234" for p in company["company"]["financial_periods"]), "Une observation précise de société a été arrondie")
    require(driver.execute("return document.querySelectorAll('.dashboard-total-grid').length;") == 0, "La vue société affiche des cartes de patrimoine fictives")
    wait_text(driver, "Comptes de la société")
    driver.capture_section(".dashboard-company-panel", artifacts / "dashboard-company.png")
    steps.append({"name": "dashboard_member_and_company_views_reuse_recorded_reports_without_fake_company_net", "status": "passed"})

    driver.click(button("Rechercher dans le coffre"))
    wait_text(driver, "Saisir au moins deux caractères.", timeout=30)
    driver.fill("#global-search-query", phase7["lot_id"])
    wait_text(driver, "1 résultat.")
    driver.click(button("Ouvrir le module Or et argent"))
    require(driver.execute("return document.querySelector('h1').textContent;") == "Or et argent", "La recherche n'ouvre pas le module de l'objet")
    driver.click(button("Rechercher dans le coffre"))
    wait_text(driver, "Saisir au moins deux caractères.", timeout=30)
    driver.fill("#global-search-query", phase6["accounts"][2]["id"])
    wait_text(driver, "1 résultat.")
    driver.click(button("Ouvrir le module Espèces et devises"))
    wait_until(lambda: driver.execute("return [...document.querySelectorAll('h2')].some(h=>h.textContent==='Caisses physiques');"), "la recherche ouvre directement les caisses physiques", timeout=30)
    require(driver.execute("return !!document.querySelector('#treasury-filter option[value=\"'+arguments[0]+'\"]');", phase6["accounts"][2]["id"]), "La caisse recherchée manque dans le module ouvert")
    require(driver.invoke("get_status")["mutation_sequence"] == before["mutation_sequence"]
            and driver.invoke("get_status")["generation_id"] == before["generation_id"], "La consultation ou l'export de rapport a modifié le coffre")
    steps.append({"name": "global_search_at_source_knowledge_opens_real_business_module_read_only", "status": "passed"})
    (artifacts / "dashboard-oracles.json").write_text(json.dumps({"source_sequence": source_k, "active_sequence": before["mutation_sequence"], "family": family, "filtered": filtered, "member": person, "company": company}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("application", type=Path)
    parser.add_argument("--artifacts", type=Path, default=Path("/artifacts"))
    parser.add_argument("--native-dialogs", action="store_true")
    args = parser.parse_args()
    application = args.application.resolve()
    require(application.is_file(), f"Binaire absent : {application}")
    args.artifacts.mkdir(parents=True, exist_ok=True)
    artifacts = args.artifacts.resolve()
    driver_binary = shutil.which("tauri-driver")
    require(driver_binary, "tauri-driver absent du PATH")
    report = {"started_at": datetime.now(timezone.utc).isoformat(), "application": str(application),
              "status": "failed", "steps": [], "native_dialogs_requested": args.native_dialogs}
    steps = report["steps"]
    driver = WebDriver()
    process = None
    success = False
    with tempfile.TemporaryDirectory(prefix="patrimoine-desktop-") as directory:
        temporary = Path(directory)
        environment = os.environ.copy()
        environment["PATRIMOINE_DATA_DIR"] = str(temporary / "coffre")
        with (artifacts / "tauri-driver.log").open("w", encoding="utf-8") as logfile:
            try:
                process = subprocess.Popen([driver_binary, "--port", "4444", "--native-port", "4445"],
                                           env=environment, stdout=logfile, stderr=subprocess.STDOUT,
                                           start_new_session=True)
                wait_until(lambda: driver.request("GET", "/status", timeout=2), "démarrage de tauri-driver", timeout=20)
                driver.start(application)
                wait_until(lambda: driver.execute("return !!window.__TAURI_INTERNALS__ && document.querySelector('h1')?.textContent === 'Tableau de bord';"),
                           "rendu de la fenêtre Tauri", timeout=25)
                require(driver.execute("return document.documentElement.lang;") == "fr", "Interface non déclarée en français")
                status = driver.invoke("get_status")
                require(status["schema_version"] == 6, f"Schéma inattendu : {status}")
                require(status["country_count"] == 4 and status["currency_count"] == 5,
                        f"Référentiels initiaux inattendus : {status}")
                require(status["document_count"] == 0, "Le coffre neuf contient déjà des documents")
                wait_text(driver, "Accessible")
                steps.append({"name": "real_desktop_render_and_ipc", "status": "passed",
                              "schema_version": status["schema_version"], "countries": status["country_count"],
                              "currencies": status["currency_count"]})
                settings = save_settings_ui(driver, "Coffre du test desktop")
                steps.append({"name": "settings_saved_through_ui_verified_by_ipc", "status": "passed", "settings": settings})
                registry_baseline = registry_scenarios(driver, artifacts, steps)
                registry_baseline, fx_state = fx_scenarios(driver, artifacts, steps)
                registry_baseline, fx_state["profiles"] = profile_scenarios(driver, artifacts, steps)
                registry_baseline, phase5 = property_rental_scenarios(driver, artifacts, steps)
                registry_baseline, phase6 = financial_scenarios(driver, artifacts, steps, phase5)
                registry_baseline, phase7 = metals_company_scenarios(driver, artifacts, steps, phase5, phase6, fx_state)
                navigate(driver, "Documents")
                wait_text(driver, "Votre espace documentaire est vide")
                require(driver.invoke("list_documents") == [], "Le module documentaire neuf n'est pas vide")
                steps.append({"name": "empty_documents_ui_and_ipc", "status": "passed"})
                if args.native_dialogs:
                    native_scenarios(driver, temporary, artifacts, steps, registry_baseline, fx_state, phase5, phase6, phase7)
                    dashboard_scenarios(driver, artifacts, steps, phase6, phase7)
                else:
                    steps.append({"name": "native_document_backup_restore_dialogs", "status": "unrun",
                                  "reason": "Activer --native-dialogs pour les parcours GTK avec xdotool."})
                driver.screenshot(artifacts / "desktop.png")
                steps.append({"name": "desktop_screenshot", "status": "passed", "path": str(artifacts / "desktop.png")})
                report["status"] = "passed"
                success = True
            except Exception as error:
                report["error"] = f"{type(error).__name__}: {error}"
                if driver.session_id:
                    try:
                        (artifacts / "desktop-failure.txt").write_text(driver.text(), encoding="utf-8")
                    except Exception:
                        pass
                    try:
                        driver.screenshot(artifacts / "desktop-failure.png")
                    except Exception as screenshot_error:
                        report["screenshot_error"] = str(screenshot_error)
            finally:
                try:
                    driver.close()
                except Exception as close_error:
                    report["cleanup_warning"] = str(close_error)
                if process:
                    try:
                        os.killpg(process.pid, signal.SIGTERM)
                    except ProcessLookupError:
                        pass
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait(timeout=5)
    report["completed_at"] = datetime.now(timezone.utc).isoformat()
    (artifacts / "desktop-result.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())
