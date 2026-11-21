use patrimoine_domain::*;

#[test]
fn actual_loan_cash_without_matching_entries_cannot_certify_a_net() {
    for (opening, kind, principal, interest, fees) in [
        ("100000", "principal_repayment", "-1000", "200", "10"),
        ("0", "drawdown", "1000", "0", "0"),
    ] {
        let mut s = Sources::complete();
        s.bank(Some("10000"));
        s.loan(opening, "outside", "a");
        s.debts.movements.push(DebtMovement {
            id: "actual-loan-cash".into(),
            revision_id: "actual-loan-cash-r".into(),
            debt_id: "loan".into(),
            kind: kind.into(),
            effective_on: "2024-03-10".into(),
            principal_change: principal.into(),
            interest_paid: interest.into(),
            fees_paid: fees.into(),
            unlinked_reason: "Flux déclaré sans rapprochement bancaire".into(),
            notes: String::new(),
        });
        let report = s.report(None, "2024-03-31");
        assert_eq!(report.net_eur, None);
        assert_eq!(report.status, "partial");
        assert_eq!(report.known_assets_eur, "10000.00");
        assert!(report
            .warnings
            .iter()
            .any(|w| w.code == "unlinked_debt_cash"));
        let repayment = kind == "principal_repayment";
        s.cash(
            "actual-loan-bank",
            "loan",
            if repayment { "-1210" } else { "1000" },
            "2024-03-10",
            "principal",
        );
        for (component, amount) in if repayment {
            vec![("principal", "-1000"), ("interest", "-200"), ("fee", "-10")]
        } else {
            vec![("principal", "1000")]
        } {
            s.link(
                &format!("matched-{component}"),
                "debt_movement",
                "actual-loan-cash",
                "actual-loan-bank",
                amount,
            );
            let link = s.treasury.cash_links.last_mut().unwrap();
            link.component = component.into();
            link.side = "debtor".into();
        }
        let matched = s.report(None, "2024-03-31");
        assert_eq!(
            matched.net_eur.as_deref(),
            Some(if repayment { "-90210.00" } else { "10000.00" })
        );
        assert!(!matched
            .warnings
            .iter()
            .any(|w| w.code == "unlinked_debt_cash"));
    }
}

#[test]
fn paid_rental_expense_needs_its_actual_debit_before_a_complete_net() {
    let mut s = Sources::complete();
    s.bank(Some("10000"));
    s.rental("outside");
    s.rentals.expenses.push(RentalExpense {
        id: "operating-cost".into(),
        revision_id: "operating-cost-r".into(),
        asset_id: "home".into(),
        kind: "operating".into(),
        amount: "2000".into(),
        currency: "EUR".into(),
        effective_on: "2024-03-10".into(),
        paid_on: Some("2024-03-10".into()),
        recoverable: false,
        notes: String::new(),
    });
    let report = s.report(None, "2024-03-31");
    assert_eq!(report.net_eur, None);
    assert!(report
        .warnings
        .iter()
        .any(|w| w.code == "unlinked_rental_expense_cash"));
    assert_eq!(report.known_assets_eur, "11000.00");
    s.cash(
        "operating-cash",
        "payment",
        "-2000",
        "2024-03-10",
        "principal",
    );
    s.link(
        "operating-link",
        "rental_expense",
        "operating-cost",
        "operating-cash",
        "-2000",
    );
    let matched = s.report(None, "2024-03-31");
    assert_eq!(matched.net_eur.as_deref(), Some("9000.00"));
    assert!(!matched
        .warnings
        .iter()
        .any(|w| w.code == "unlinked_rental_expense_cash"));
}

fn share(n: &str, d: &str) -> Fraction {
    Fraction {
        num: n.into(),
        den: d.into(),
    }
}

fn entity(id: &str, kind: &str) -> Entity {
    Entity {
        id: id.into(),
        revision_id: format!("{id}-r"),
        kind: kind.into(),
        display_name: id.into(),
        first_name: None,
        last_name: None,
        birth_date: None,
        residence_country: Some("FR".into()),
        preferred_currency: "EUR".into(),
        activity: String::new(),
        notes: String::new(),
        archived: false,
    }
}

struct Sources {
    registry: RegistrySnapshot,
    properties: PropertySnapshot,
    rentals: RentalSnapshot,
    treasury: TreasurySnapshot,
    debts: DebtSnapshot,
    finance: FinanceSnapshot,
}

impl Sources {
    fn complete() -> Self {
        Self {
            registry: RegistrySnapshot {
                entities: vec![
                    entity("a", "person"),
                    entity("b", "person"),
                    entity("outside", "person"),
                ],
                memberships: ["a", "b"]
                    .into_iter()
                    .map(|id| Membership {
                        id: format!("member-{id}"),
                        revision_id: format!("member-{id}-r"),
                        person_id: id.into(),
                        valid_from: "2024-01-01".into(),
                        valid_to: None,
                        relationship: String::new(),
                    })
                    .collect(),
                ..Default::default()
            },
            properties: PropertySnapshot::default(),
            rentals: RentalSnapshot::default(),
            treasury: TreasurySnapshot::default(),
            debts: DebtSnapshot::default(),
            finance: FinanceSnapshot {
                coverages: vec![FinanceCoverage {
                    id: "coverage".into(),
                    revision_id: "coverage-r".into(),
                    valid_from: "2024-01-01".into(),
                    valid_to: None,
                    assets_complete: true,
                    debts_complete: true,
                    rentals_complete: true,
                    notes: "Inventaire synthétique explicitement complet pour ce scénario".into(),
                }],
                ..Default::default()
            },
        }
    }

    fn report(&self, person: Option<&str>, date: &str) -> FinancialReport {
        financial_report(
            &self.registry,
            &self.properties,
            &self.rentals,
            &self.treasury,
            &self.debts,
            &self.finance,
            &FinancialReportRequest {
                context: if person.is_some() { "person" } else { "family" }.into(),
                person_id: person.map(str::to_owned),
                date: date.into(),
                knowledge_sequence: None,
            },
        )
        .unwrap()
    }

    fn validate(&self) -> patrimoine_domain::Result<()> {
        validate_finance(
            &self.registry,
            &self.properties,
            &self.rentals,
            &self.treasury,
            &self.debts,
            &self.finance,
        )
    }

    fn asset(&mut self, id: &str, value: &str) {
        self.registry.assets.push(Asset {
            id: id.into(),
            revision_id: format!("{id}-r"),
            name: id.into(),
            country: "FR".into(),
            category: "other".into(),
            notes: String::new(),
            archived: false,
        });
        self.registry.valuations.push(Valuation {
            id: format!("{id}-value"),
            revision_id: format!("{id}-value-r"),
            target_kind: "asset".into(),
            target_id: id.into(),
            amount: value.into(),
            currency: "EUR".into(),
            effective_on: "2024-01-01".into(),
            source: "Estimation synthétique, distincte du journal bancaire".into(),
        });
    }

    fn own(&mut self, id: &str, asset: &str, owner: &str, n: &str, d: &str) {
        self.registry.ownerships.push(Ownership {
            id: id.into(),
            revision_id: format!("{id}-r"),
            asset_id: asset.into(),
            owner_entity_id: owner.into(),
            legal_share: Some(share(n, d)),
            economic_share: Some(share(n, d)),
            valid_from: "2024-01-01".into(),
            valid_to: None,
        });
    }

    fn company(&mut self, id: &str, member_part: Fraction, outside_part: Fraction, mode: &str) {
        self.registry.entities.push(entity(id, "company"));
        for (owner, fraction) in [("a", member_part), ("outside", outside_part)] {
            self.registry.participations.push(Participation {
                id: format!("{id}-{owner}"),
                revision_id: format!("{id}-{owner}-r"),
                owner_entity_id: owner.into(),
                company_id: id.into(),
                legal_share: Some(fraction.clone()),
                economic_share: Some(fraction.clone()),
                voting_share: Some(fraction),
                valid_from: "2024-01-01".into(),
                valid_to: None,
            });
        }
        self.registry.company_policies.push(CompanyPolicy {
            id: format!("{id}-policy"),
            revision_id: format!("{id}-policy-r"),
            company_id: id.into(),
            mode: mode.into(),
            coverage_complete: true,
            liability_qualified: true,
            valid_from: "2024-01-01".into(),
            valid_to: None,
        });
    }

    fn loan(&mut self, amount: &str, creditor: &str, debtor: &str) {
        self.debts.contracts.push(DebtContract {
            id: "loan".into(),
            revision_id: "loan-r".into(),
            name: "Contrat unique synthétique".into(),
            currency: "EUR".into(),
            tracking_start: "2024-01-01".into(),
            notes: String::new(),
            archived: false,
        });
        self.debts.terms.push(DebtTerms {
            id: "loan-terms".into(),
            revision_id: "loan-terms-r".into(),
            debt_id: "loan".into(),
            valid_from: "2024-01-01".into(),
            valid_to: None,
            principal_status: "complete".into(),
            interest_rate: None,
            interest_convention: String::new(),
            notes: String::new(),
            parties: [("creditor", creditor), ("debtor", debtor)]
                .into_iter()
                .map(|(side, party)| FinancingParty {
                    line_id: format!("loan-{side}"),
                    side: side.into(),
                    entity_id: party.into(),
                    legal_share: Some(share("1", "1")),
                    economic_share: Some(share("1", "1")),
                    liability_kind: "single".into(),
                    notes: String::new(),
                })
                .collect(),
        });
        self.debts.movements.push(DebtMovement {
            id: "loan-open".into(),
            revision_id: "loan-open-r".into(),
            debt_id: "loan".into(),
            kind: "opening".into(),
            effective_on: "2024-01-01".into(),
            principal_change: amount.into(),
            interest_paid: "0".into(),
            fees_paid: "0".into(),
            unlinked_reason: "Encours historique d’ouverture, aucun cash nouveau".into(),
            notes: String::new(),
        });
    }

    fn bank(&mut self, opening: Option<&str>) {
        self.asset("wallet-asset", "999999");
        self.own("wallet-owner", "wallet-asset", "a", "1", "1");
        self.treasury.accounts.push(TreasuryAccount {
            id: "wallet".into(),
            revision_id: "wallet-r".into(),
            asset_id: "wallet-asset".into(),
            kind: "bank".into(),
            currency: "EUR".into(),
            tracking_start: "2024-01-01".into(),
            legal_owner_entity_id: "a".into(),
            bank_name: "Banque synthétique".into(),
            reference: String::new(),
            country: "FR".into(),
            storage_location: String::new(),
            notes: String::new(),
            archived: false,
        });
        if let Some(value) = opening {
            self.cash("wallet-open", "opening", value, "2024-01-01", "opening");
        }
    }

    fn cash(&mut self, id: &str, kind: &str, amount: &str, date: &str, role: &str) {
        self.treasury.transactions.push(TreasuryTransaction {
            id: id.into(),
            revision_id: format!("{id}-r"),
            kind: kind.into(),
            effective_on: date.into(),
            label: "Mouvement synthétique réel déclaré".into(),
            reference: String::new(),
            notes: String::new(),
            entries: vec![TreasuryEntry {
                line_id: format!("{id}-line"),
                account_id: "wallet".into(),
                amount: amount.into(),
                role: role.into(),
            }],
        });
    }

    fn link(&mut self, id: &str, kind: &str, source: &str, cash: &str, amount: &str) {
        self.treasury.cash_links.push(CashLink {
            id: id.into(),
            revision_id: format!("{id}-r"),
            source_kind: kind.into(),
            source_id: source.into(),
            source_revision_id: format!("{source}-r"),
            transaction_revision_id: format!("{cash}-r"),
            line_id: format!("{cash}-line"),
            amount: amount.into(),
            component: "full".into(),
            side: String::new(),
            notes: String::new(),
        });
    }

    fn rental(&mut self, tenant: &str) {
        self.asset("home", "0");
        self.own("home-owner", "home", "a", "1", "1");
        self.properties.properties.push(Property {
            id: "property".into(),
            revision_id: "property-r".into(),
            asset_id: "home".into(),
            property_type: "house".into(),
            city: String::new(),
            address: String::new(),
            surface_m2: None,
            status: "rented".into(),
            valid_from: "2024-01-01".into(),
            valid_to: None,
            notes: String::new(),
        });
        self.properties.units.push(RentalUnit {
            id: "unit".into(),
            revision_id: "unit-r".into(),
            property_asset_id: "home".into(),
            reference: "U1".into(),
            name: "Unité synthétique".into(),
            surface_m2: None,
            valid_from: "2024-01-01".into(),
            valid_to: None,
            available_for_rent: true,
            notes: String::new(),
        });
        self.rentals.leases.push(Lease {
            id: "lease".into(),
            revision_id: "lease-r".into(),
            unit_id: "unit".into(),
            tenant_entity_id: tenant.into(),
            valid_from: "2024-03-01".into(),
            valid_to: None,
            notes: String::new(),
        });
        let terms = LeaseTerms {
            id: "rent-terms".into(),
            revision_id: "rent-terms-r".into(),
            lease_id: "lease".into(),
            rent: "1000".into(),
            charges: "0".into(),
            currency: "EUR".into(),
            frequency: "monthly".into(),
            due_day: 5,
            deposit_required: "2000".into(),
            valid_from: "2024-03-01".into(),
            valid_to: None,
            notes: String::new(),
        };
        self.rentals.lease_terms.push(terms.clone());
        self.rentals.lease_term_revisions.push(terms);
        self.rentals.dues.push(RentDue {
            id: "due".into(),
            revision_id: "due-r".into(),
            lease_id: "lease".into(),
            terms_revision_id: "rent-terms-r".into(),
            period_start: "2024-03-01".into(),
            period_end: "2024-04-01".into(),
            due_on: "2024-03-05".into(),
            rent: "1000".into(),
            charges: "0".into(),
            currency: "EUR".into(),
            generation_key: "march-due".into(),
            notes: String::new(),
        });
        self.finance
            .rent_claims
            .push(claim("claim-a", "a", "2024-03-01", None));
    }

    fn receipt(&mut self, amount: &str, allocated: &str, date: &str) {
        self.rentals.payments.push(RentPayment {
            id: "rent-payment".into(),
            revision_id: "rent-payment-r".into(),
            lease_id: "lease".into(),
            kind: "receipt".into(),
            amount: amount.into(),
            currency: "EUR".into(),
            effective_on: date.into(),
            original_payment_id: None,
            recipient_entity_id: "a".into(),
            reference: "Réception synthétique".into(),
            unlinked_reason: "Source rapprochée explicitement par le scénario".into(),
        });
        self.rentals.allocations.push(RentAllocation {
            id: "rent-allocation".into(),
            revision_id: "rent-allocation-r".into(),
            payment_id: "rent-payment".into(),
            due_id: "due".into(),
            component: "rent".into(),
            amount: allocated.into(),
            currency: "EUR".into(),
            effective_on: date.into(),
        });
        self.cash("rent-cash", "rent", amount, date, "principal");
        self.link(
            "rent-link",
            "rent_payment",
            "rent-payment",
            "rent-cash",
            amount,
        );
    }

    fn deposit(&mut self, id: &str, kind: &str, amount: &str, date: &str) {
        self.treasury.deposits.push(DepositMovement {
            id: id.into(),
            revision_id: format!("{id}-r"),
            lease_id: "lease".into(),
            kind: kind.into(),
            amount: amount.into(),
            currency: "EUR".into(),
            effective_on: date.into(),
            recipient_entity_id: "a".into(),
            due_id: (kind == "applied_to_due").then(|| "due".into()),
            component: if kind == "applied_to_due" {
                "rent".into()
            } else {
                String::new()
            },
            notes: "Décision de dépôt synthétique documentée".into(),
        });
        if kind == "received" || kind == "returned" {
            let cash = format!("{id}-cash");
            self.cash(&cash, "deposit", amount, date, "principal");
            self.link(&format!("{id}-link"), "deposit_movement", id, &cash, amount);
        }
    }
}

fn claim(id: &str, owner: &str, from: &str, to: Option<&str>) -> RentClaim {
    RentClaim {
        id: id.into(),
        revision_id: format!("{id}-r"),
        due_id: "due".into(),
        parties: vec![RentClaimParty {
            entity_id: owner.into(),
            economic_share: Some(share("1", "1")),
        }],
        valid_from: from.into(),
        valid_to: to.map(str::to_owned),
        represented_by_debt_id: None,
        notes: "Droits explicites de cette créance, indépendants du propriétaire courant".into(),
    }
}

fn totals(report: &FinancialReport, assets: &str, liabilities: &str, net: &str) {
    assert_eq!(
        report.assets_eur.as_deref(),
        Some(assets),
        "assets; warnings: {:?}",
        report.warnings
    );
    assert_eq!(
        report.liabilities_eur.as_deref(),
        Some(liabilities),
        "liabilities; warnings: {:?}",
        report.warnings
    );
    assert_eq!(
        report.net_eur.as_deref(),
        Some(net),
        "net; warnings: {:?}",
        report.warnings
    );
    assert_eq!(report.status, "complete");
}

#[test]
fn calculated_bank_balance_replaces_generic_estimate_and_excludes_future_cash() {
    let mut s = Sources::complete();
    s.bank(Some("100"));
    s.cash("receipt", "receipt", "25", "2024-03-01", "principal");
    s.cash("future", "receipt", "500", "2024-04-01", "principal");
    let result = s.report(None, "2024-03-31");
    totals(&result, "125.00", "0.00", "125.00");
    assert!(!result
        .source_ids
        .iter()
        .any(|id| id == "wallet-asset-value-r" || id == "future-r"));
    assert!(result
        .positions
        .iter()
        .any(|p| p.kind == "treasury" && p.amount_eur.as_deref() == Some("125.00")));
}

#[test]
fn absent_opening_and_unconfirmed_inventory_never_become_complete_zeroes() {
    let mut s = Sources::complete();
    s.bank(None);
    let missing = s.report(None, "2024-03-31");
    assert!(missing.assets_eur.is_none() && missing.net_eur.is_none());
    assert!(missing
        .warnings
        .iter()
        .any(|w| w.code == "missing_account_opening"));
    s.treasury.transactions.push(TreasuryTransaction {
        id: "open".into(),
        revision_id: "open-r".into(),
        kind: "opening".into(),
        effective_on: "2024-01-01".into(),
        label: "Ouverture nulle confirmée".into(),
        reference: String::new(),
        notes: String::new(),
        entries: vec![TreasuryEntry {
            line_id: "open-line".into(),
            account_id: "wallet".into(),
            amount: "0".into(),
            role: "opening".into(),
        }],
    });
    totals(&s.report(None, "2024-03-31"), "0.00", "0.00", "0.00");
    s.finance.coverages.clear();
    let uncovered = s.report(None, "2024-03-31");
    assert!(!uncovered.coverage_confirmed && uncovered.net_eur.is_none());
}

#[test]
fn sole_owner_bank_overdraft_is_one_positive_liability() {
    let mut s = Sources::complete();
    s.bank(Some("0"));
    s.cash("outgoing", "payment", "-50", "2024-03-01", "principal");
    let result = s.report(None, "2024-03-31");
    totals(&result, "0.00", "50.00", "-50.00");
    assert_eq!(
        result
            .positions
            .iter()
            .filter(|p| p.kind == "overdraft")
            .count(),
        1
    );
    assert!(!result.positions.iter().any(|p| p.kind == "treasury"));
}

#[test]
fn personal_debt_is_not_divided_by_the_financed_property_shares() {
    let mut s = Sources::complete();
    s.asset("house", "400000");
    s.own("house-a", "house", "a", "1", "2");
    s.own("house-b", "house", "b", "1", "2");
    s.loan("100000", "outside", "a");
    s.debts.asset_links.push(DebtAssetLink {
        id: "collateral".into(),
        revision_id: "collateral-r".into(),
        debt_id: "loan".into(),
        asset_id: "house".into(),
        relation_kind: "collateral".into(),
        attributed_principal_share: None,
        valid_from: "2024-01-01".into(),
        valid_to: None,
        notes: String::new(),
    });
    totals(
        &s.report(Some("a"), "2024-03-31"),
        "200000.00",
        "100000.00",
        "100000.00",
    );
    totals(
        &s.report(Some("b"), "2024-03-31"),
        "200000.00",
        "0.00",
        "200000.00",
    );
    totals(
        &s.report(None, "2024-03-31"),
        "400000.00",
        "100000.00",
        "300000.00",
    );
    assert!(
        s.treasury.transactions.is_empty(),
        "Historical loan opening cannot fabricate a bank flow"
    );
}

#[test]
fn one_intrafamily_contract_eliminates_both_faces_only_in_family_view() {
    let mut s = Sources::complete();
    s.loan("10000", "a", "b");
    totals(
        &s.report(Some("a"), "2024-03-31"),
        "10000.00",
        "0.00",
        "10000.00",
    );
    totals(
        &s.report(Some("b"), "2024-03-31"),
        "0.00",
        "10000.00",
        "-10000.00",
    );
    let family = s.report(None, "2024-03-31");
    totals(&family, "0.00", "0.00", "0.00");
    assert_eq!(family.eliminations.len(), 1);
    assert_eq!(family.eliminations[0].exact_eliminated, share("10000", "1"));
}

fn partially_owned_companies() -> Sources {
    let mut s = Sources::complete();
    s.company("creditco", share("3", "4"), share("1", "4"), "detailed");
    s.company("debtco", share("2", "5"), share("3", "5"), "detailed");
    s.loan("10000", "creditco", "debtco");
    s
}

#[test]
fn partially_owned_companies_eliminate_only_the_common_nominal_fraction() {
    let s = partially_owned_companies();
    let result = s.report(None, "2024-03-31");
    totals(&result, "3500.00", "0.00", "3500.00");
    let e = &result.eliminations[0];
    assert_eq!(e.creditor_before_eur, "7500.00");
    assert_eq!(e.debtor_before_eur, "4000.00");
    assert_eq!(e.eliminated_eur, "4000.00");
    assert_eq!(e.creditor_after_eur, "3500.00");
    assert_eq!(e.debtor_after_eur, "0.00");
}

#[test]
fn creditor_impairment_leaves_an_unmatched_debtor_residual() {
    let mut s = partially_owned_companies();
    s.debts
        .receivable_adjustments
        .push(DebtReceivableAdjustment {
            id: "impairment".into(),
            revision_id: "impairment-r".into(),
            debt_id: "loan".into(),
            creditor_entity_id: "creditco".into(),
            kind: "impairment".into(),
            effective_on: "2024-03-01".into(),
            valuation_delta: "-1000".into(),
            source: "Dépréciation documentée du prêteur ; aucune remise au débiteur".into(),
            notes: String::new(),
        });
    let result = s.report(None, "2024-03-31");
    totals(&result, "3150.00", "400.00", "2750.00");
    let e = &result.eliminations[0];
    assert_eq!(e.creditor_before_eur, "6750.00");
    assert_eq!(e.debtor_before_eur, "4000.00");
    assert_eq!(e.eliminated_eur, "3600.00");
    assert_eq!(e.creditor_after_eur, "3150.00");
    assert_eq!(e.debtor_after_eur, "400.00");
}

#[test]
fn opaque_equity_and_personal_claim_are_not_rededucted_when_debt_is_included() {
    let mut s = Sources::complete();
    s.company("opaque", share("3", "5"), share("2", "5"), "global_equity");
    s.registry.valuations.push(Valuation {
        id: "equity".into(),
        revision_id: "equity-r".into(),
        target_kind: "company".into(),
        target_id: "opaque".into(),
        amount: "100000".into(),
        currency: "EUR".into(),
        effective_on: "2024-01-01".into(),
        source: "Equity après passif du prêt inclus".into(),
    });
    s.loan("100000", "a", "opaque");
    s.debts.coverages.push(DebtCoverage {
        id: "included".into(),
        revision_id: "included-r".into(),
        debt_id: "loan".into(),
        company_id: "opaque".into(),
        side: "debtor".into(),
        valid_from: "2024-01-01".into(),
        valid_to: None,
        included: true,
        source: "Le passif exact est déjà inclus dans cette valeur nette".into(),
        notes: String::new(),
    });
    let covered = s.report(None, "2024-03-31");
    totals(&covered, "160000.00", "0.00", "160000.00");
    assert!(
        covered.eliminations.is_empty(),
        "Hidden liability must not eliminate the personal creditor a second time"
    );
    s.debts.coverages.clear();
    let unknown = s.report(None, "2024-03-31");
    assert!(
        unknown.net_eur.is_none(),
        "Missing opaque debt coverage is not implicit inclusion"
    );
}

#[test]
fn bank_receipt_and_only_remaining_rent_claim_total_the_original_due_once() {
    let mut s = Sources::complete();
    s.rental("outside");
    s.bank(Some("0"));
    s.receipt("400", "400", "2024-03-05");
    let result = s.report(None, "2024-03-31");
    totals(&result, "1000.00", "0.00", "1000.00");
    assert!(result
        .positions
        .iter()
        .any(|p| p.amount_eur.as_deref() == Some("400.00")));
    assert!(result
        .positions
        .iter()
        .any(|p| p.amount_eur.as_deref() == Some("600.00")));
    s.treasury.cash_links.clear();
    assert!(
        s.report(None, "2024-03-31").net_eur.is_none(),
        "Unmatched rent receipt cannot confirm a complete net"
    );
}

#[test]
fn matching_requires_exact_source_and_ledger_revisions_not_just_ids() {
    let mut s = Sources::complete();
    s.rental("outside");
    s.bank(Some("0"));
    s.receipt("400", "400", "2024-03-05");
    s.treasury.cash_links[0].source_revision_id = "old-payment-revision".into();
    assert!(s.validate().is_err());
    s.treasury.cash_links[0].source_revision_id = "rent-payment-r".into();
    s.treasury.cash_links[0].transaction_revision_id = "old-ledger-revision".into();
    assert!(s.validate().is_err());
}

#[test]
fn rent_owed_between_two_family_members_has_paired_asset_and_liability() {
    let mut s = Sources::complete();
    s.rental("b");
    totals(
        &s.report(Some("a"), "2024-03-31"),
        "1000.00",
        "0.00",
        "1000.00",
    );
    totals(
        &s.report(Some("b"), "2024-03-31"),
        "0.00",
        "1000.00",
        "-1000.00",
    );
    let result = s.report(None, "2024-03-31");
    totals(&result, "0.00", "0.00", "0.00");
    assert_eq!(result.eliminations[0].eliminated_eur, "1000.00");
}

#[test]
fn money_allocated_to_future_rent_remains_an_advance_until_exigibility() {
    let mut s = Sources::complete();
    s.rental("outside");
    s.bank(Some("0"));
    s.receipt("1500", "1000", "2024-02-01");
    totals(&s.report(None, "2024-02-15"), "1500.00", "1500.00", "0.00");
    totals(
        &s.report(None, "2024-03-31"),
        "1500.00",
        "500.00",
        "1000.00",
    );
}

#[test]
fn received_and_returned_deposit_change_cash_and_obligation_not_income() {
    let mut s = Sources::complete();
    s.rental("outside");
    s.bank(Some("0"));
    s.deposit("deposit-in", "received", "2000", "2024-01-02");
    totals(&s.report(None, "2024-01-31"), "2000.00", "2000.00", "0.00");
    s.deposit("deposit-return", "returned", "-500", "2024-02-20");
    totals(&s.report(None, "2024-02-28"), "1500.00", "1500.00", "0.00");
    assert!(s.rentals.payments.is_empty());
}

#[test]
fn deposit_application_reduces_claim_and_obligation_without_a_new_cash_receipt() {
    let mut s = Sources::complete();
    s.rental("outside");
    s.bank(Some("0"));
    s.deposit("deposit-in", "received", "2000", "2024-01-02");
    s.deposit("apply", "applied_to_due", "-300", "2024-03-10");
    let result = s.report(None, "2024-03-31");
    totals(&result, "2700.00", "1700.00", "1000.00");
    assert!(result
        .positions
        .iter()
        .any(|p| p.amount_eur.as_deref() == Some("700.00")));
    assert_eq!(s.treasury.transactions.len(), 2);
    assert!(s.rentals.payments.is_empty());
}

#[test]
fn deposit_holder_cannot_spend_another_creditors_rent_component() {
    let mut s = Sources::complete();
    s.rental("outside");
    s.bank(Some("0"));
    for term in s
        .rentals
        .lease_terms
        .iter_mut()
        .chain(s.rentals.lease_term_revisions.iter_mut())
    {
        term.charges = "100".into();
    }
    s.rentals.dues[0].charges = "100".into();
    s.finance.rent_claims[0].parties = vec![
        RentClaimParty {
            entity_id: "a".into(),
            economic_share: Some(share("1", "2")),
        },
        RentClaimParty {
            entity_id: "b".into(),
            economic_share: Some(share("1", "2")),
        },
    ];
    s.deposit("deposit-in", "received", "2000", "2024-01-02");
    s.deposit("apply", "applied_to_due", "-550", "2024-03-10");
    assert!(
        s.validate().is_err(),
        "A holds only500 of rent: its50 of charges cannot authorize550 against rent"
    );
}

#[test]
fn claim_transfer_after_deposit_application_moves_only_the_seven_hundred_remaining() {
    let mut s = Sources::complete();
    s.rental("outside");
    s.bank(Some("0"));
    s.deposit("deposit-in", "received", "2000", "2024-01-02");
    s.deposit("apply", "applied_to_due", "-300", "2024-03-10");
    s.finance.rent_claims[0].valid_to = Some("2024-03-15".into());
    s.finance
        .rent_claims
        .push(claim("claim-b", "b", "2024-03-15", None));
    totals(
        &s.report(Some("b"), "2024-03-31"),
        "700.00",
        "0.00",
        "700.00",
    );
    totals(
        &s.report(Some("a"), "2024-03-31"),
        "2000.00",
        "1700.00",
        "300.00",
    );
    totals(
        &s.report(None, "2024-03-31"),
        "2700.00",
        "1700.00",
        "1000.00",
    );
}

fn represented_rent() -> Sources {
    let mut s = Sources::complete();
    s.bank(Some("0"));
    s.rental("outside");
    s.loan("1000", "a", "outside");
    s.finance.rent_claims[0].valid_from = "2024-03-05".into();
    s.finance.rent_claims[0].represented_by_debt_id = Some("loan".into());
    s
}

fn assert_representation_rejected(s: &Sources) {
    let error = s.validate().unwrap_err();
    assert_eq!(error.code, "validation");
    let report = s.report(None, "2024-03-31");
    assert_eq!(report.status, "partial");
    assert_eq!(report.net_eur, None);
    assert!(report
        .warnings
        .iter()
        .any(|w| w.code == "rent_loan_representation_mismatch"));
}

#[test]
fn a_representative_loan_requires_the_tenant_and_the_actual_creditors() {
    let s = represented_rent();
    s.validate().unwrap();
    totals(
        &s.report(Some("a"), "2024-03-31"),
        "1000.00",
        "0.00",
        "1000.00",
    );
    for (creditor, debtor) in [("outside", "a"), ("a", "b"), ("b", "outside")] {
        let mut invalid = represented_rent();
        invalid.debts.terms[0].parties[0].entity_id = creditor.into();
        invalid.debts.terms[0].parties[1].entity_id = debtor.into();
        assert_representation_rejected(&invalid);
    }
}

#[test]
fn shared_rent_creditors_must_have_the_same_exact_loan_fractions() {
    let mut s = represented_rent();
    s.finance.rent_claims[0].parties = vec![
        RentClaimParty {
            entity_id: "a".into(),
            economic_share: Some(share("1", "3")),
        },
        RentClaimParty {
            entity_id: "b".into(),
            economic_share: Some(share("2", "3")),
        },
    ];
    let mut other = s.debts.terms[0].parties[0].clone();
    other.line_id = "loan-creditor-b".into();
    other.entity_id = "b".into();
    other.legal_share = Some(share("2", "3"));
    other.economic_share = Some(share("2", "3"));
    s.debts.terms[0].parties[0].legal_share = Some(share("1", "3"));
    s.debts.terms[0].parties[0].economic_share = Some(share("1", "3"));
    s.debts.terms[0].parties.push(other);
    s.validate().unwrap();
    assert_eq!(
        s.report(Some("a"), "2024-03-31").net_eur.as_deref(),
        Some("333.33")
    );
    s.debts.terms[0].parties[0].economic_share = Some(share("1", "2"));
    s.debts.terms[0].parties[2].economic_share = Some(share("1", "2"));
    assert_representation_rejected(&s);
    s.debts.terms[0].parties[0].economic_share = None;
    assert_representation_rejected(&s);
}

#[test]
fn several_represented_dues_share_the_loans_principal_capacity() {
    let mut s = represented_rent();
    let mut due = s.rentals.dues[0].clone();
    due.id = "due-two".into();
    due.revision_id = "due-two-r".into();
    due.generation_key = "second-due".into();
    due.period_start = "2024-04-01".into();
    due.period_end = "2024-05-01".into();
    due.due_on = "2024-04-05".into();
    s.rentals.dues.push(due);
    let mut second = s.finance.rent_claims[0].clone();
    second.id = "second-claim".into();
    second.revision_id = "second-claim-r".into();
    second.due_id = "due-two".into();
    second.valid_from = "2024-04-05".into();
    s.finance.rent_claims[0].valid_from = "2024-04-05".into();
    s.finance.rent_claims.push(second);
    assert!(
        s.validate().is_err(),
        "one principal cannot replace two full dues"
    );
    let invalid = s.report(None, "2024-04-30");
    assert_eq!(invalid.net_eur, None);
    assert!(invalid
        .warnings
        .iter()
        .any(|w| w.code == "rent_loan_representation_mismatch"));
    s.debts.movements[0].principal_change = "2000".into();
    s.validate().unwrap();
    totals(&s.report(None, "2024-04-30"), "2000.00", "0.00", "2000.00");
}

#[test]
fn representation_checks_term_continuity_and_dated_changes() {
    let mut s = represented_rent();
    s.debts.terms[0].valid_to = Some("2024-03-20".into());
    assert_representation_rejected(&s);
    let mut later = s.debts.terms[0].clone();
    later.id = "later-terms".into();
    later.revision_id = "later-terms-r".into();
    later.valid_from = "2024-03-20".into();
    later.valid_to = None;
    later.parties[0].entity_id = "b".into();
    s.debts.terms.push(later);
    assert_representation_rejected(&s);
    assert_eq!(s.report(None, "2024-03-10").status, "complete");
}

#[test]
fn represented_rent_is_settled_from_the_loan_source_after_transfer() {
    let mut s = represented_rent();
    s.debts.movements.push(DebtMovement {
        id: "loan-paid".into(),
        revision_id: "loan-paid-r".into(),
        debt_id: "loan".into(),
        kind: "principal_repayment".into(),
        effective_on: "2024-03-10".into(),
        principal_change: "-250".into(),
        interest_paid: "0".into(),
        fees_paid: "0".into(),
        unlinked_reason: "Remboursement rapproché de son entrée bancaire".into(),
        notes: String::new(),
    });
    s.cash("loan-receipt", "loan", "250", "2024-03-10", "principal");
    s.link(
        "loan-receipt-link",
        "debt_movement",
        "loan-paid",
        "loan-receipt",
        "250",
    );
    let link = s.treasury.cash_links.last_mut().unwrap();
    link.component = "principal".into();
    link.side = "creditor".into();
    s.validate().unwrap();
    totals(&s.report(None, "2024-03-31"), "1000.00", "0.00", "1000.00");
    // A rental-only receipt cannot leave an unchanged representative loan and
    // still certify a complete balance, including when it settles the full due.
    let mut stale = represented_rent();
    stale.receipt("1000", "1000", "2024-03-10");
    assert_representation_rejected(&stale);
}

#[test]
fn represented_rent_cannot_be_settled_again_by_a_later_deposit_application() {
    let mut s = represented_rent();
    s.deposit("deposit-in", "received", "2000", "2024-03-01");
    s.validate().unwrap();
    assert_eq!(
        s.report(None, "2024-03-06").net_eur.as_deref(),
        Some("1000.00")
    );
    s.deposit("apply", "applied_to_due", "-300", "2024-03-10");
    // This state is already forbidden by the pre-existing non-cash settlement
    // invariant, before a report can consume it. A rejected write cannot create
    // this future state in a persisted historical snapshot.
    let error = s.validate().unwrap_err();
    assert_eq!(error.code, "validation");
    assert!(error.message.contains("source unique avant compensation"));
}
