CREATE TABLE objects_finance (
 id TEXT PRIMARY KEY,
 kind TEXT NOT NULL CHECK(kind IN ('workspace','document','document_link','entity','membership','participation','asset','ownership','valuation','company_policy','exchange_rate','profile_period','property','rental_unit','property_event','lease','lease_terms','rent_due','rent_payment','rent_allocation','rental_expense','treasury_account','treasury_transaction','account_reconciliation','cash_link','deposit_movement','debt_contract','debt_terms','debt_movement','debt_receivable_adjustment','debt_schedule','debt_payment_allocation','debt_asset_link','debt_guarantee','debt_coverage','finance_coverage','rent_claim','rent_claim_adjustment')),
 created_at TEXT NOT NULL
) STRICT;
INSERT INTO objects_finance SELECT id,kind,created_at FROM objects;
DROP TABLE objects;
ALTER TABLE objects_finance RENAME TO objects;
CREATE TABLE treasury_accounts (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), asset_id TEXT NOT NULL REFERENCES assets(id),
 kind TEXT NOT NULL CHECK(kind IN ('bank','cash')), currency TEXT NOT NULL REFERENCES currencies(code),
 tracking_start TEXT NOT NULL, legal_owner_entity_id TEXT NOT NULL REFERENCES entities(id),
 bank_name TEXT NOT NULL, reference TEXT NOT NULL, country TEXT NOT NULL REFERENCES countries(code),
 storage_location TEXT NOT NULL, notes TEXT NOT NULL, archived INTEGER NOT NULL CHECK(archived IN(0,1))
) STRICT;
CREATE INDEX treasury_accounts_asset ON treasury_accounts(asset_id);
CREATE TABLE treasury_transactions (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), kind TEXT NOT NULL, effective_on TEXT NOT NULL,
 label TEXT NOT NULL, reference TEXT NOT NULL, notes TEXT NOT NULL
) STRICT;
CREATE INDEX treasury_transactions_date ON treasury_transactions(effective_on);
CREATE TABLE treasury_entries (
 transaction_revision_id TEXT NOT NULL REFERENCES treasury_transactions(revision_id), line_id TEXT NOT NULL,
 account_id TEXT NOT NULL REFERENCES objects(id), amount TEXT NOT NULL, role TEXT NOT NULL,
 PRIMARY KEY(transaction_revision_id,line_id)
) STRICT;
CREATE INDEX treasury_entries_account ON treasury_entries(account_id);
CREATE TABLE account_reconciliations (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), account_id TEXT NOT NULL REFERENCES objects(id),
 effective_on TEXT NOT NULL, statement_balance TEXT NOT NULL, statement_reference TEXT NOT NULL,
 document_id TEXT REFERENCES documents(id), notes TEXT NOT NULL
) STRICT;
CREATE TABLE cash_links (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), source_kind TEXT NOT NULL,
 source_id TEXT NOT NULL REFERENCES objects(id), source_revision_id TEXT NOT NULL REFERENCES revisions(id),
 transaction_revision_id TEXT NOT NULL, line_id TEXT NOT NULL, amount TEXT NOT NULL,
 component TEXT NOT NULL, side TEXT NOT NULL, notes TEXT NOT NULL,
 FOREIGN KEY(transaction_revision_id,line_id) REFERENCES treasury_entries(transaction_revision_id,line_id)
) STRICT;
CREATE INDEX cash_links_source ON cash_links(source_id,source_revision_id);
CREATE INDEX cash_links_entry ON cash_links(transaction_revision_id,line_id);
CREATE TABLE deposit_movements (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), lease_id TEXT NOT NULL REFERENCES objects(id),
 kind TEXT NOT NULL, amount TEXT NOT NULL, currency TEXT NOT NULL REFERENCES currencies(code),
 effective_on TEXT NOT NULL, recipient_entity_id TEXT NOT NULL REFERENCES entities(id),
 due_id TEXT REFERENCES objects(id), component TEXT NOT NULL, notes TEXT NOT NULL
) STRICT;
CREATE INDEX deposit_movements_lease ON deposit_movements(lease_id,effective_on);
CREATE TABLE debt_contracts (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
 name TEXT NOT NULL, currency TEXT NOT NULL REFERENCES currencies(code),
 tracking_start TEXT NOT NULL, notes TEXT NOT NULL, archived INTEGER NOT NULL CHECK(archived IN(0,1))
) STRICT;
CREATE TABLE debt_terms (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), debt_id TEXT NOT NULL REFERENCES objects(id),
 valid_from TEXT NOT NULL, valid_to TEXT, principal_status TEXT NOT NULL,
 interest_rate TEXT, interest_convention TEXT NOT NULL, notes TEXT NOT NULL,
 CHECK(valid_to IS NULL OR valid_to>valid_from)
) STRICT;
CREATE INDEX debt_terms_contract_period ON debt_terms(debt_id,valid_from,valid_to);
CREATE TABLE debt_parties (
 terms_revision_id TEXT NOT NULL REFERENCES debt_terms(revision_id), line_id TEXT NOT NULL,
 side TEXT NOT NULL CHECK(side IN('creditor','debtor')), entity_id TEXT NOT NULL REFERENCES entities(id),
 legal_share_num TEXT, legal_share_den TEXT, economic_share_num TEXT, economic_share_den TEXT,
 liability_kind TEXT NOT NULL, notes TEXT NOT NULL,
 PRIMARY KEY(terms_revision_id,line_id),
 CHECK((legal_share_num IS NULL)=(legal_share_den IS NULL)),
 CHECK((economic_share_num IS NULL)=(economic_share_den IS NULL))
) STRICT;
CREATE TABLE debt_movements (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), debt_id TEXT NOT NULL REFERENCES objects(id),
 kind TEXT NOT NULL, effective_on TEXT NOT NULL, principal_change TEXT NOT NULL,
 interest_paid TEXT NOT NULL, fees_paid TEXT NOT NULL, unlinked_reason TEXT NOT NULL, notes TEXT NOT NULL
) STRICT;
CREATE INDEX debt_movements_contract_date ON debt_movements(debt_id,effective_on);
CREATE TABLE debt_receivable_adjustments (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), debt_id TEXT NOT NULL REFERENCES objects(id),
 creditor_entity_id TEXT NOT NULL REFERENCES entities(id), kind TEXT NOT NULL,
 effective_on TEXT NOT NULL, valuation_delta TEXT NOT NULL, source TEXT NOT NULL, notes TEXT NOT NULL
) STRICT;
CREATE INDEX debt_adjustments_contract_date ON debt_receivable_adjustments(debt_id,effective_on);
CREATE TABLE debt_schedules (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), debt_id TEXT NOT NULL REFERENCES objects(id),
 due_on TEXT NOT NULL, expected_principal TEXT NOT NULL, expected_interest TEXT NOT NULL,
 expected_fees TEXT NOT NULL, status TEXT NOT NULL, notes TEXT NOT NULL
) STRICT;
CREATE INDEX debt_schedules_contract_date ON debt_schedules(debt_id,due_on);
CREATE TABLE debt_payment_allocations (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), movement_id TEXT NOT NULL REFERENCES objects(id),
 schedule_id TEXT NOT NULL REFERENCES objects(id), effective_on TEXT NOT NULL,
 principal TEXT NOT NULL, interest TEXT NOT NULL, fees TEXT NOT NULL
) STRICT;
CREATE INDEX debt_allocations_movement_date ON debt_payment_allocations(movement_id,effective_on);
CREATE TABLE debt_asset_links (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), debt_id TEXT NOT NULL REFERENCES objects(id),
 asset_id TEXT NOT NULL REFERENCES assets(id), relation_kind TEXT NOT NULL,
 attributed_principal_share_num TEXT, attributed_principal_share_den TEXT,
 valid_from TEXT NOT NULL, valid_to TEXT, notes TEXT NOT NULL,
 CHECK((attributed_principal_share_num IS NULL)=(attributed_principal_share_den IS NULL)),
 CHECK(valid_to IS NULL OR valid_to>valid_from)
) STRICT;
CREATE TABLE debt_guarantees (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), debt_id TEXT NOT NULL REFERENCES objects(id),
 guarantor_entity_id TEXT NOT NULL REFERENCES entities(id), limit_amount TEXT,
 valid_from TEXT NOT NULL, valid_to TEXT, notes TEXT NOT NULL,
 CHECK(valid_to IS NULL OR valid_to>valid_from)
) STRICT;
CREATE TABLE debt_coverages (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), debt_id TEXT NOT NULL REFERENCES objects(id),
 company_id TEXT NOT NULL REFERENCES entities(id), side TEXT NOT NULL,
 valid_from TEXT NOT NULL, valid_to TEXT, included INTEGER NOT NULL CHECK(included IN(0,1)),
 source TEXT NOT NULL, notes TEXT NOT NULL,
 CHECK(valid_to IS NULL OR valid_to>valid_from)
) STRICT;
CREATE TABLE finance_coverages (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), valid_from TEXT NOT NULL, valid_to TEXT,
 assets_complete INTEGER NOT NULL CHECK(assets_complete IN(0,1)),
 debts_complete INTEGER NOT NULL CHECK(debts_complete IN(0,1)),
 rentals_complete INTEGER NOT NULL CHECK(rentals_complete IN(0,1)), notes TEXT NOT NULL,
 CHECK(valid_to IS NULL OR valid_to>valid_from)
) STRICT;
CREATE TABLE rent_claims (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), due_id TEXT NOT NULL REFERENCES objects(id),
 valid_from TEXT NOT NULL, valid_to TEXT, represented_by_debt_id TEXT REFERENCES objects(id), notes TEXT NOT NULL,
 CHECK(valid_to IS NULL OR valid_to>valid_from)
) STRICT;
CREATE INDEX rent_claims_due ON rent_claims(due_id,valid_from,valid_to);
CREATE TABLE rent_claim_parties (
 claim_revision_id TEXT NOT NULL REFERENCES rent_claims(revision_id), entity_id TEXT NOT NULL REFERENCES entities(id),
 economic_share_num TEXT, economic_share_den TEXT, PRIMARY KEY(claim_revision_id,entity_id),
 CHECK((economic_share_num IS NULL)=(economic_share_den IS NULL))
) STRICT;
CREATE TABLE rent_claim_adjustments (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), due_id TEXT NOT NULL REFERENCES objects(id),
 creditor_entity_id TEXT NOT NULL REFERENCES entities(id), kind TEXT NOT NULL, amount TEXT NOT NULL,
 effective_on TEXT NOT NULL, source TEXT NOT NULL, notes TEXT NOT NULL
) STRICT;
CREATE INDEX rent_claim_adjustments_due ON rent_claim_adjustments(due_id,effective_on);
